use crate::internal_comms::{LocalInfo, FetchedDataMessage};

use std::net::IpAddr;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};

use pnet;
use std::process::Command;

pub fn fetch_and_return_local_info(tx: Sender<FetchedDataMessage>, interface: String) {
    let interfaces = pnet::datalink::interfaces();
    for iface in interfaces {
        if iface.name == *interface {
            // Get local IP
            let local_ip = iface.ips[0].ip().to_string();

            // Get subnet mask
            let subnet_mask = iface.ips[0].prefix().to_string();

            // Get gateway
            let gateway = get_default_gateway(&interface);

            let gateway = match gateway {
                Ok(gateway) => Some(gateway),
                Err(_) => None
            };

            let local_info = LocalInfo {
                local_ip: Some(local_ip),
                subnet_mask: Some(subnet_mask),
                gateway: gateway
            };

            tx.send(FetchedDataMessage::LocalInfo(local_info)).unwrap();
        }
    }
}

pub fn get_interface_ip(interface: &String) -> Result<IpAddr, ()> {
    let interfaces = pnet::datalink::interfaces();
    for iface in interfaces {
        if iface.name == *interface {
            return Ok(iface.ips[0].ip());
        }
    }

    Err(())
}

fn get_default_gateway(interface: &String) -> Result<String, ()> {
    let output = Command::new("ip")
        .arg("route")
        .arg("show")
        .arg("dev")
        .arg(interface)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(_) => return Err(())
    };

    let output_str = std::str::from_utf8(&output.stdout);

    let output_str = match output_str {
        Ok(output_str) => output_str,
        Err(_) => return Err(())
    };

    for line in output_str.lines() {
        if line.contains("default") {
            let gateway = line.split_whitespace().nth(2);

            let gateway = match gateway {
                Some(gateway) => gateway,
                None => return Err(())
            };

            return Ok(gateway.to_string());
        }
    }

    Err(())
}