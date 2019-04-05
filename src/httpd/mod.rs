//! HTTP Server Module

use alloc::vec::Vec;
use smoltcp::iface::EthernetInterface;
use smoltcp::time::Instant;
use smoltcp::socket::{Socket, SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::wire::{EthernetAddress, IpAddress, IpEndpoint, Ipv4Address};
use stm32f7::stm32f7x6::{ETHERNET_DMA, ETHERNET_MAC, RCC, SYSCFG};
use stm32f7_discovery::{self, print, println, system_clock};
use stm32f7_discovery::ethernet::{self, PhyError};

pub struct HTTPD<'a> {
    ethernet_interface: EthernetInterface<'static, 'static, 'static, ethernet::EthernetDevice<'a>>,
    sockets: SocketSet<'static, 'static, 'static>,
}

impl<'a> HTTPD<'a> {
    pub fn poll(&mut self) {
        let timestamp = Instant::from_millis(system_clock::ms() as i64);

        match self.ethernet_interface.poll(&mut self.sockets, timestamp) {
            Err(::smoltcp::Error::Exhausted) => println!("Exhausted"),
            Err(::smoltcp::Error::Unrecognized) => println!("Unrecongnized"),
            Err(e) => println!("Error: {:?}", e),
            Ok(changed) => {
                if !changed {
                    return
                }

                for mut socket in self.sockets.iter_mut() {
                    println!("polling socket...");

                    match *socket {
                        Socket::Tcp(ref mut socket) => {
                            if !socket.can_recv() {
                                println!("socket cannot receive");
                            }

                            println!("receiving data...");
                        },
                        _ => {}
                    }
                }
            },
        }
    }
}

pub fn init<'a>(
    rcc: &mut RCC,
    syscfg: &mut SYSCFG,
    ethernet_mac: &mut ETHERNET_MAC,
    ethernet_dma: &'a mut ETHERNET_DMA,
    ethernet_addr: EthernetAddress,
    ip_addr: Ipv4Address,
    port: u16,
) -> Result<HTTPD<'a>, PhyError> {
    let ethernet_interface = ethernet::EthernetDevice::new(
        Default::default(),
        Default::default(),
        rcc,
        syscfg,
        ethernet_mac,
        ethernet_dma,
        ethernet_addr,
    )
    .map(|device| device.into_interface());

    if let Err(e) = ethernet_interface {
        return Err(e);
    }

    let mut sockets = SocketSet::new(Vec::new());

    let endpoint = IpEndpoint::new(IpAddress::Ipv4(ip_addr), port);

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; ethernet::MTU]);

    let mut tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
    tcp_socket.listen(endpoint).unwrap();
    sockets.add(tcp_socket);

    Ok(HTTPD {
        ethernet_interface: ethernet_interface.unwrap(),
        sockets: sockets,
    })
}
