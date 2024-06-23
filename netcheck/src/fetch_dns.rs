use crate::internal_comms::{DNSInfo, DNSServer, FetchedDataMessage};

use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};

use rustdns::Message;

use resolv_conf::Config;

use std::time::Duration;

use socket2::{Socket, Domain, Type, Protocol, SockAddr};
use std::net::{UdpSocket, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::fetch_local::get_interface_ip;

pub fn fetch_and_return_dns_info(tx: Sender<FetchedDataMessage>, interface: String) {
    let dns_servers = get_dns_servers();

    let dns_servers = match dns_servers {
        Ok(dns_servers) => dns_servers,
        Err(_) => {
            tx.send(FetchedDataMessage::DNSInfo(DNSInfo {
                can_fetch: Some(false),
                can_bind_interface: None,
                dns_servers: Vec::new(),
            })).unwrap();
            return;
        }
    };

    let interface_ip = get_interface_ip(&interface);

    let interface_ip = match interface_ip {
        Ok(interface_ip) => interface_ip,
        Err(_) => {
            tx.send(FetchedDataMessage::DNSInfo(DNSInfo {
                can_fetch: Some(false),
                can_bind_interface: Some(false),
                dns_servers: Vec::new(),
            })).unwrap();
            return;
        }
    };

    // Eliminate duplicates while preserving order
    let dns_servers_length = dns_servers.len();
    let dns_servers = dns_servers.into_iter().fold(Vec::with_capacity(dns_servers_length), |mut acc, x| {
        if !acc.contains(&x) {
            acc.push(x);
        }
        acc
    });

    let mut dns_info = DNSInfo {
        can_fetch: Some(true),
        can_bind_interface: None,
        dns_servers: dns_servers.iter().map(|server| DNSServer {
            ip: server.to_string(),
            can_resolve: None,
        }).collect(),
    };

    tx.send(FetchedDataMessage::DNSInfo(dns_info.clone())).unwrap();

    // Now start checking if we can resolve DNS through them

    for server in dns_servers {
        let can_resolve = check_dns_resolution(&server, interface_ip);

        if can_resolve == CheckDNSResolutionResponse::CannotBind {
            tx.send(FetchedDataMessage::DNSInfo(DNSInfo {
                can_fetch: Some(false),
                can_bind_interface: Some(false),
                dns_servers: Vec::new(),
            })).unwrap();
            return;
        }

        for dns_server in dns_info.dns_servers.iter_mut() {
            if dns_server.ip == server {
                dns_server.can_resolve = Some(can_resolve == CheckDNSResolutionResponse::Success);
                break;
            }
        }

        tx.send(FetchedDataMessage::DNSInfo(dns_info.clone())).unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CheckDNSResolutionResponse {
    Success,
    Failure,
    CannotBind
}

fn check_dns_resolution(server: &str, ip_addr: IpAddr) -> CheckDNSResolutionResponse {
    // go to example.com and resolve it
    let mut message = Message::default();
    message.add_question("example.com", rustdns::Type::A, rustdns::Class::Internet);

    let message = message.to_vec().unwrap();

    let socket = match ip_addr.is_ipv4() {
        true => {
            // Set bind address to the interface IP
            let bind_addr = SocketAddr::new(ip_addr, 5000);

            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP));

            let socket = match socket {
                Ok(socket) => socket,
                Err(_) => return CheckDNSResolutionResponse::CannotBind
            };

            if socket.bind(&SockAddr::from(bind_addr)).is_err() {
                return CheckDNSResolutionResponse::CannotBind;
            };

            socket
        },
        false => {
            // Set bind address to the interface IP
            let bind_addr = SocketAddr::new(ip_addr, 5000);

            let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP));

            let socket = match socket {
                Ok(socket) => socket,
                Err(_) => return CheckDNSResolutionResponse::CannotBind
            };

            if socket.bind(&SockAddr::from(bind_addr)).is_err() {
                return CheckDNSResolutionResponse::CannotBind;
            };

            socket
        }
    };

    // Now send the message
    let udp_socket = UdpSocket::from(socket);

    // Set a timeout of 1 second
    if udp_socket.set_read_timeout(Some(Duration::from_secs(1))).is_err() {
        return CheckDNSResolutionResponse::Failure;
    };

    let resp =  udp_socket.connect(format!("{}:53", server));

    match resp {
        Ok(_) => {},
        Err(error) => {
            return CheckDNSResolutionResponse::Failure
        }
    };

    if udp_socket.send(&message).is_err() {
        return CheckDNSResolutionResponse::Failure;
    };

    let mut buf = [0u8; 512];

    let resp_len = match udp_socket.recv(&mut buf) {
        Ok(resp_len) => resp_len,
        Err(_) => {
            return CheckDNSResolutionResponse::Failure
        }
    };

    let resp = match Message::from_slice(&buf[..resp_len]) {
        Ok(resp) => resp,
        Err(_) => {
            return CheckDNSResolutionResponse::Failure
        }
    };

    match resp.rcode == rustdns::Rcode::NoError {
        true => CheckDNSResolutionResponse::Success,
        false => CheckDNSResolutionResponse::Failure
    }
}

fn get_dns_servers() -> Result<Vec<String>, ()> {
    let file = std::fs::read_to_string("/etc/resolv.conf");

    let file = match file {
        Ok(file) => file,
        Err(_) => return Err(())
    };

    let config = Config::parse(&file);

    let config = match config {
        Ok(config) => config,
        Err(_) => return Err(())
    };

    let mut dns_servers = Vec::new();

    for nameserver in config.nameservers {
        dns_servers.push(nameserver.to_string());
    }

    Ok(dns_servers)
}