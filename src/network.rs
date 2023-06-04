use pnet::datalink;
use pnet::datalink::{Channel, MacAddr};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::{MutablePacket, Packet};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

/// Represents a host with its IP and MAC addresses.
///
/// The `Host` struct contains the IP address and MAC address of a host in a network.
///
/// # Fields
///
/// * `ip_address` - The IP address of the host.
/// * `mac_address` - The MAC address of the host.
///
/// # Example
///
/// ```
/// use std::net::IpAddr;
/// use pnet::datalink::MacAddr;
/// use network::Host;
///
/// let host = Host {
///     ip_address: IpAddr::V4("192.168.1.100".parse().unwrap()),
///     mac_address: MacAddr::new(0x01, 0x23, 0x45, 0x67, 0x89, 0xab),
/// };
/// ```
pub struct Host {
    pub ip_address: IpAddr,
    pub mac_address: MacAddr,
}

/// Initialize a host
pub fn new_host(ip_address: &str, mac_address: &str) -> Host {
    let instantiated_ip = match IpAddr::from_str(ip_address) {
        Ok(ip) => ip,
        Err(e) => panic!(
            "Error parsing ip address: {}, look for: {} in your configuiration file.",
            e, ip_address
        ),
    };
    let instantiated_mac = match MacAddr::from_str(mac_address) {
        Ok(mac) => mac,
        Err(e) => panic!(
            "Error parsing mac address: {}, look for: {} in your configuiration file.",
            e, mac_address
        ),
    };

    Host {
        ip_address: instantiated_ip,
        mac_address: instantiated_mac,
    }
}

/// Finds a `Host` object in the provided slice of hosts that matches the specified target IP address.
///
/// This function searches through the `hosts` slice using an iterator and checks if any `Host` object has an IP address
/// that matches the `target_ip` parameter. If a match is found, it returns a reference to the matching `Host` object.
/// Otherwise, it returns `None`.
///
/// # Arguments
///
/// * `hosts` - A slice of `Host` objects representing the available hosts.
/// * `target_ip` - The target IPv4 address to match against.
///
/// # Returns
///
/// An optional reference to the matching `Host` object, or `None` if no match is found.
///
/// # Example
///
/// ```
/// use std::net::Ipv4Addr;
/// use network::Host;
/// use network::find_host_by_ip;
///
/// let hosts = [
///     Host { ip_address: Ipv4Addr::new(192, 168, 1, 100), mac_address: /* MAC Address */ },
///     Host { ip_address: Ipv4Addr::new(192, 168, 1, 101), mac_address: /* MAC Address */ },
///     Host { ip_address: Ipv4Addr::new(192, 168, 1, 102), mac_address: /* MAC Address */ },
/// ];
///
/// let target_ip = Ipv4Addr::new(192, 168, 1, 101);
/// if let Some(host) = find_host_by_ip(&hosts, target_ip) {
///     println!("Host found: {:?}", host);
/// } else {
///     println!("Host not found for IP: {:?}", target_ip);
/// }
/// ```
fn find_host_by_ip(hosts: &[Host], target_ip: Ipv4Addr) -> Option<&Host> {
    hosts.iter().find(|host| host.ip_address == target_ip)
}

/// Crafts and sends an ARP response packet to the network using the provided parameters.
///
/// This function constructs an ARP response packet with the specified sender and destination IP and MAC addresses.
/// The constructed packet is then sent to the network interface specified by the `interface` parameter.
///
/// # Arguments
///
/// * `sender_ip_address` - The IPv4 address of the sender in the ARP response.
/// * `sender_mac_address` - The MAC address of the sender in the ARP response.
/// * `destination_ip_address` - The IPv4 address of the destination in the ARP response.
/// * `destination_mac_address` - The MAC address of the destination in the ARP response.
/// * `interface` - The network interface to send the ARP response packet.
///
/// # Example
///
/// ```
/// use std::net::Ipv4Addr;
/// use pnet::datalink::MacAddr;
/// use pnet::datalink::NetworkInterface;
///
/// let sender_ip = Ipv4Addr::new(192, 168, 1, 100);
/// let sender_mac = MacAddr::new(0x01, 0x23, 0x45, 0x67, 0x89, 0xab);
/// let destination_ip = Ipv4Addr::new(192, 168, 1, 1);
/// let destination_mac = MacAddr::new(0xcd, 0xef, 0x12, 0x34, 0x56, 0x78);
///
/// let interface = NetworkInterface::default();
///
/// forge_arp_response(sender_ip, sender_mac, destination_ip, destination_mac, &interface);
/// ```
fn forge_arp_response(
    sender_ip_address: Ipv4Addr,
    sender_mac_address: MacAddr,
    destination_ip_address: Ipv4Addr,
    destination_mac_address: MacAddr,
    interface: &datalink::NetworkInterface,
) {
    // create a buffer to store the data
    let mut ethernet_buffer = [0u8; 42];
    // Create an empty packet
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    //setup the packet
    ethernet_packet.set_destination(destination_mac_address);
    ethernet_packet.set_source(sender_mac_address);
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    // this is the size of the whole arp packet
    let mut arp_buffer = [0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

    // setup the packet https://en.wikipedia.org/wiki/Address_Resolution_Protocol
    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Reply);
    arp_packet.set_sender_hw_addr(sender_mac_address);
    arp_packet.set_sender_proto_addr(sender_ip_address);
    arp_packet.set_target_hw_addr(destination_mac_address);
    arp_packet.set_target_proto_addr(destination_ip_address);

    // so RN we have the ethernet channel, the ethernet packet and the arp packet

    // Load the ethernet packet with the arp packet
    ethernet_packet.set_payload(arp_packet.packet_mut());

    // TODO: Improve these panic messages.
    // Open an ethernet channel to send and receive data
    let (mut sender, mut _receiver) = match pnet::datalink::channel(interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happend {}", e),
    };
    // Send the packet
    sender
        .send_to(ethernet_packet.packet(), None)
        .unwrap()
        .unwrap();
    log::debug!(
        "ARP Reply: Sent to ip: {}, mac: {}",
        destination_ip_address,
        destination_mac_address
    );
}

/// Responds to ARP queries received on the specified network interface with the provided list of hosts.
///
/// This function listens for ARP queries on the network interface specified by `interface_name`. When an ARP query is received,
/// it checks if the query's target IP address matches any of the IP addresses in the `hosts` list. If a match is found,
/// it crafts and sends an ARP response with the corresponding MAC address for the target IP address.
///
/// # Arguments
///
/// * `interface_name` - The name of the network interface to listen on for ARP queries.
/// * `hosts` - A vector of `Host` objects representing the IP-MAC address mappings for the valid targets.
///
/// # Example
///
/// ```
/// use network::Host;
/// use network::respond_arp_queries;
///
/// let hosts = vec![
///     Host { ip_address: "192.168.1.100".parse().unwrap(), mac_address: "01:23:45:67:89:ab".parse().unwrap() },
///     Host { ip_address: "192.168.1.101".parse().unwrap(), mac_address: "cd:ef:12:34:56:78".parse().unwrap() },
/// ];
///
/// respond_arp_queries("eth0", hosts);
/// ```
pub fn respond_arp_queries(interface_name: &str, hosts: Vec<Host>) {
    // instantiate the interface
    let source_interface = datalink::interfaces()
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .unwrap_or_else(|| panic!("Interface not found!",));

    // TODO: Improve these panic messages.
    // Open an ethernet channel to send and receive data
    let (_sender, mut receiver) =
        match pnet::datalink::channel(&source_interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unknown channel type"),
            Err(e) => panic!("Error happend {}", e),
        };

    // let's hear to the network
    loop {
        let buf = receiver.next().unwrap();
        let arp = ArpPacket::new(&buf[MutableEthernetPacket::minimum_packet_size()..]).unwrap();
        // Check if the packet is an ARP request and if the target address is defined in our configuration
        if arp.get_operation() == ArpOperations::Request
            && hosts
                .iter()
                .any(|host| host.ip_address == arp.get_target_proto_addr())
        {
            // Get the host that needs to "respond" to the ARP request
            if let Some(sender_host) = find_host_by_ip(&hosts, arp.get_target_proto_addr()) {
                log::debug!(
                    "[ARP Request]: from ip: {}, mac: {}",
                    sender_host.ip_address,
                    sender_host.mac_address
                );
                forge_arp_response(
                    arp.get_target_proto_addr(),
                    sender_host.mac_address,
                    arp.get_sender_proto_addr(),
                    arp.get_sender_hw_addr(),
                    &source_interface,
                )
            }
        }
    }
}
