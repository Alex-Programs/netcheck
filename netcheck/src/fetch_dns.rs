use crate::internal_comms::{DNSInfo, DNSServer, FetchedDataMessage};

use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};

use rustdns::Message;
use rustdns::types::*;
use rustdns::Rcode;

use resolv_conf::Config;

use std::net::UdpSocket;
use std::time::Duration;

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
        let start_time = std::time::Instant::now();

        let can_resolve = check_dns_resolution(&server);

        for dns_server in dns_info.dns_servers.iter_mut() {
            if dns_server.ip == server {
                dns_server.can_resolve = Some(can_resolve);

                break;
            }
        }

        tx.send(FetchedDataMessage::DNSInfo(dns_info.clone())).unwrap();

        // Ensure at least 50ms between checks
        let min_time = Duration::from_millis(50);
        let elapsed = start_time.elapsed();

        if elapsed < min_time {
            std::thread::sleep(min_time - elapsed);
        }
    }
}

fn check_dns_resolution(server: &str) -> bool {
    // go to example.com and resolve it
    let mut message = Message::default();
    message.add_question("example.com", Type::A, Class::Internet);

    let message = message.to_vec().unwrap();

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return false
    };

    if socket.set_read_timeout(Some(Duration::from_secs(1))).is_err() {
        return false;
    }

    if socket.connect(format!("{}:53", server)).is_err() {
        return false;
    }

    let sent = socket.send(&message);

    if sent.is_err() {
        return false;
    }

    let mut resp = [0; 4096];

    let resp_len = socket.recv(&mut resp);

    let resp_len = match resp_len {
        Ok(resp_len) => resp_len,
        Err(_) => return false
    };

    let resp = match Message::from_slice(&resp[..resp_len]) {
        Ok(resp) => resp,
        Err(_) => return false
    };

    resp.rcode == Rcode::NoError
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