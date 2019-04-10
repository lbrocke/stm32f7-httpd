//! HTTP Server Module

use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};
use log::{debug, info, warn};
use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache};
use smoltcp::socket::{SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use stm32f7::stm32f7x6::{ETHERNET_DMA, ETHERNET_MAC, RCC, SYSCFG};
use stm32f7_discovery::ethernet::{self, PhyError};
use stm32f7_discovery::{self, system_clock};

mod request;
pub use self::request::Request;
mod response;
pub use self::response::Response;
mod status;
pub use self::status::Status;

mod parser;
use parser::{HTTPParser, ParseError};

pub struct HTTPD {
    ethernet_interface: EthernetInterface<'static, 'static, 'static, ethernet::EthernetDevice>,
    sockets: SocketSet<'static, 'static, 'static>,
    tcp_handle: SocketHandle,
    port: u16,
    connected: bool,
    routes_callback: Option<&'static Fn(&Request, &Vec<u8>) -> Response>,
    input_buffer: Vec<u8>,
    request_state: RequestState,
}

impl HTTPD {
    pub fn new(
        rcc: &mut RCC,
        syscfg: &mut SYSCFG,
        ethernet_mac: &mut ETHERNET_MAC,
        ethernet_dma: ETHERNET_DMA,
        ethernet_addr: EthernetAddress,
        ip_addr: IpAddress,
        port: u16,
    ) -> Result<HTTPD, PhyError> {
        ethernet::EthernetDevice::new(
            Default::default(),
            Default::default(),
            rcc,
            syscfg,
            ethernet_mac,
            ethernet_dma,
            ethernet_addr,
        )
        .map(|ethernet_device| {
            let ip_addresses = [IpCidr::new(ip_addr, 24)];

            let tcp_receive_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
            let tcp_send_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
            let tcp_socket = TcpSocket::new(tcp_receive_buffer, tcp_send_buffer);

            // ARP cache of MAC address => IP address mappings
            let neighbor_cache = NeighborCache::new(BTreeMap::new());

            let ethernet_interface = EthernetInterfaceBuilder::new(ethernet_device)
                .ethernet_addr(ethernet_addr)
                .neighbor_cache(neighbor_cache)
                .ip_addrs(ip_addresses)
                .finalize();

            let mut sockets = SocketSet::new(vec![]);
            let tcp_handle = sockets.add(tcp_socket);

            HTTPD {
                ethernet_interface,
                sockets,
                tcp_handle,
                port,
                connected: false,
                routes_callback: None,
                input_buffer: vec![],
                request_state: RequestState::Wait,
            }
        })
    }

    pub fn routes(&mut self, routes_callback: &'static Fn(&Request, &Vec<u8>) -> Response) {
        self.routes_callback = Some(routes_callback);
    }

    pub fn poll(&mut self) {
        let timestamp = Instant::from_millis(system_clock::ms() as i64);

        match self.ethernet_interface.poll(&mut self.sockets, timestamp) {
            Ok(_) => {}
            Err(e) => {
                debug!("polling error: {}", e);
            }
        }

        match self.poll_socket() {
            PollStatus::Established => self.request_init(),
            PollStatus::Received(data) => self.request_receive(data),
            PollStatus::Closed => self.request_close(),
            PollStatus::Inactive => (),
        }
    }

    fn request_init(&mut self) {
        info!("Connection opened");
        self.input_buffer = vec![];
        self.request_state = RequestState::ReadHead;
    }

    fn request_receive(&mut self, chunk: Vec<u8>) {
        let read = chunk.len();
        debug!("Received {} bytes", read);

        self.input_buffer.extend(chunk);

        match &self.request_state {
            RequestState::ReadHead => self.read_head(),
            RequestState::ReadBody(request, bytes_to_read) => {
                self.read_body(request.clone(), *bytes_to_read, read)
            }
            state => warn!("Can't receive in state {:?}", state),
        }
    }

    fn read_body(&mut self, request: Request, bytes_to_read: usize, read: usize) {
        self.request_state = if read >= bytes_to_read {
            self.input_buffer
                .truncate(self.input_buffer.len() - (read - bytes_to_read));
            RequestState::RequestRead(request, self.input_buffer.clone())
        } else {
            RequestState::ReadBody(request, bytes_to_read - read)
        }
    }

    fn read_head(&mut self) {
        // TODO: do this better (i'm sure it's possible)
        // This expression basically copies the buffer
        //                      v----------------------------v
        match String::from_utf8(self.input_buffer[..].to_vec()) {
            Ok(input_string) => {
                let mut parser = HTTPParser::new(&input_string);

                match parser.parse_head() {
                    Ok(request_head) => {
                        debug!("Request head parsed.");

                        let content_length_res =
                            request_head.headers().get("content-length").and_then(
                                |content_length_field| content_length_field.parse::<usize>().ok(),
                            );

                        match content_length_res {
                            Some(content_length) => {
                                self.input_buffer = parser.source.as_bytes().to_vec();
                                self.read_body(
                                    request_head,
                                    content_length,
                                    self.input_buffer.len(),
                                );
                            }
                            None => {
                                self.request_state = RequestState::RequestRead(request_head, vec![])
                            }
                        }
                    }
                    Err(ParseError::NotEnoughInput) => {
                        debug!("Request header incomplete");
                    }
                    Err(ParseError::Fatal) => {
                        warn!("Could not parse request header");
                        self.request_state = RequestState::ParseError;
                    }
                }
            }
            Err(_e) => {
                warn!("Request header is not UTF-8");
                self.request_state = RequestState::ParseError;
            }
        }
    }

    fn request_response(&mut self) {}

    fn request_close(&self) {
        info!("Connection closed");
    }

    fn want_receive(&self) -> bool {
        match self.request_state {
            RequestState::ReadHead | RequestState::ReadBody(_, _) => true,
            _ => false,
        }
    }

    fn poll_socket(&mut self) -> PollStatus {
        // Needed below, but 'let mut socket' keeps a mut ref to self
        let want_receive = self.want_receive();

        let mut socket = self.sockets.get::<TcpSocket>(self.tcp_handle);

        let old_connection_status = self.connected;
        self.connected = socket.is_active();

        if old_connection_status != self.connected {
            return if self.connected {
                PollStatus::Established
            } else {
                PollStatus::Closed
            };
        }

        if !socket.is_open() {
            socket.listen(self.port).expect("Could not listen");
            info!("Listening...");
        }

        if socket.may_recv() && want_receive {
            let data = socket
                .recv(|recv_buffer| (recv_buffer.len(), recv_buffer.to_owned()))
                .unwrap();

            if data.len() > 0 {
                return PollStatus::Received(data);
            }
        } else if socket.may_send() {
            match &self.request_state {
                RequestState::RequestRead(request, body) => {
                    debug!("Request head:");
                    debug!("{:?}", request);
                    debug!("Body:");
                    debug!("{:?}", body);

                    let response = match self.routes_callback {
                        None => unimplemented!(),
                        Some(routes_cb) => routes_cb(request, body),
                    };
                    let (status_num, status_text) = response.status.numerical_and_text();

                    socket
                        .send_slice(
                            format!("HTTP/1.1 {} {}\r\n", status_num, status_text).as_bytes(),
                        )
                        .expect("Could not send head line");

                    for (key, value) in response.headers {
                        socket.send_slice(format!("{}: {}\r\n", key, value).as_bytes()).expect("Could not send header");
                    }

                    socket.send_slice("\r\n".as_bytes()).expect("Could not send end-of-header");

                    socket.send_slice(&response.body).expect("Could not send body");;
                }
                _ => warn!("Request not read"),
            }

            socket.close();
        }

        PollStatus::Inactive
    }
}

enum PollStatus {
    Established,
    Received(Vec<u8>),
    Closed,
    Inactive,
}

#[derive(Clone, Debug)]
enum RequestState {
    Wait,
    ReadHead,
    ReadBody(Request, usize),
    RequestRead(Request, Vec<u8>),
    ParseError,
}
