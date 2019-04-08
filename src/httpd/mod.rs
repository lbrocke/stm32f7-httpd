//! HTTP Server Module

use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};
use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache};
use smoltcp::socket::{SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use stm32f7::stm32f7x6::{ETHERNET_DMA, ETHERNET_MAC, RCC, SYSCFG};
use stm32f7_discovery::ethernet::{self, PhyError};
use stm32f7_discovery::{self, system_clock};
use log::{warn, debug};

mod request;
pub use self::request::Request;
mod response;
pub use self::response::Response;
mod status;
pub use self::status::Status;

mod parser;

pub struct HTTPD<'a> {
    ethernet_interface: EthernetInterface<'static, 'static, 'static, ethernet::EthernetDevice<'a>>,
    sockets: SocketSet<'static, 'static, 'static>,
    tcp_handle: SocketHandle,
    port: u16,
    connected: bool,
    routes_callback: Option<&'a Fn(&Request, &mut Response)>,
    input_buffer: Vec<u8>,
}

impl<'a> HTTPD<'a> {
    pub fn new<'b>(
        rcc: &mut RCC,
        syscfg: &mut SYSCFG,
        ethernet_mac: &mut ETHERNET_MAC,
        ethernet_dma: &'b mut ETHERNET_DMA,
        ethernet_addr: EthernetAddress,
        ip_addr: IpAddress,
        port: u16,
    ) -> Result<HTTPD<'b>, PhyError> {
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
            }
        })
    }

    pub fn routes(&mut self, routes: &'a Fn(&Request, &mut Response)) {
        self.routes_callback = Some(routes);
    }

    pub fn poll(&mut self) {
        let timestamp = Instant::from_millis(system_clock::ms() as i64);

        match self.ethernet_interface.poll(&mut self.sockets, timestamp) {
            Ok(_) => {}
            Err(e) => {
                warn!("polling error: {}", e);
            }
        }

        let mut socket = self.sockets.get::<TcpSocket>(self.tcp_handle);

        if !socket.is_open() {
            socket.listen(self.port).expect("Could not listen");
            debug!("Listening...");
        }

        // Socket becomes active
        if !self.connected && socket.is_active() {
            debug!("Connection established");
        }
        // Socket becomes inactive
        if self.connected && !socket.is_active() {
            debug!("Connection closed");
        }
        self.connected = socket.is_active();

        if socket.may_recv() {
            let data = socket
                .recv(|recv_buffer| (recv_buffer.len(), recv_buffer.to_owned()))
                .unwrap();

            if data.len() > 0 {
                self.input_buffer.extend_from_slice(&data);
                let input_as_str = String::from_utf8(self.input_buffer.to_owned()).unwrap();

                debug!("Input buffer: '{}'", input_as_str);

                let mut request_parser =
                    parser::HTTPParser::new(&input_as_str);

                match request_parser.parse_head() {
                    Err(parser::ParseError::Fatal) => {
                        debug!("Could not parse HTTP request");
                        socket.close();
                    },
                    Err(parser::ParseError::NotEnoughInput) => {
                        debug!("Waiting for more input");
                    },
                    Ok(request) => {
                        let mut response =
                            Response::new();
                        self.routes_callback.unwrap()(&request, &mut response);
                    }
                }
            }
        } else if socket.may_send() {
            debug!("Closing socket");
            socket.close();
            self.input_buffer = vec![];
        }
    }
}
