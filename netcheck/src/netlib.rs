extern crate pnet;

// Get list of network interfaces
pub fn get_interfaces() -> Vec<String> {
    let interfaces = pnet::datalink::interfaces();
    let mut interface_names = Vec::new();
    for interface in interfaces {
        interface_names.push(interface.name);
    }
    interface_names
}