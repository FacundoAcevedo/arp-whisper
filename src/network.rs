use pnet::datalink::{self, Channel, MacAddr};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::MutablePacket;
use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

const ETHERNET_FRAME_SIZE: usize = 42;
const ARP_FRAME_SIZE: usize = 28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub ip_address: IpAddr,
    pub mac_address: MacAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArpRequest {
    pub sender_ip: Ipv4Addr,
    pub sender_mac: MacAddr,
    pub target_ip: Ipv4Addr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArpReply {
    pub sender_ip: Ipv4Addr,
    pub sender_mac: MacAddr,
    pub target_ip: Ipv4Addr,
    pub target_mac: MacAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidIpAddress { value: String },
    InvalidMacAddress { value: String },
    InvalidFrame,
    MissingHostsSection,
    InterfaceNotFound { name: String },
    ChannelOpen { message: String },
    ChannelSend { message: String },
    PacketBuild { layer: &'static str },
    UnsupportedChannel,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIpAddress { value } => write!(f, "invalid IP address: {value}"),
            Self::InvalidMacAddress { value } => write!(f, "invalid MAC address: {value}"),
            Self::InvalidFrame => write!(f, "frame is too short or malformed"),
            Self::MissingHostsSection => write!(f, "missing [Hosts] section in configuration"),
            Self::InterfaceNotFound { name } => write!(f, "interface not found: {name}"),
            Self::ChannelOpen { message } => {
                write!(f, "failed to open datalink channel: {message}")
            }
            Self::ChannelSend { message } => write!(f, "failed to send packet: {message}"),
            Self::PacketBuild { layer } => write!(f, "failed to build {layer} packet"),
            Self::UnsupportedChannel => write!(f, "unsupported datalink channel"),
        }
    }
}

impl Error for NetworkError {}

pub fn parse_host(ip_address: &str, mac_address: &str) -> Result<Host, NetworkError> {
    let ip_address = IpAddr::from_str(ip_address).map_err(|_| NetworkError::InvalidIpAddress {
        value: ip_address.to_owned(),
    })?;
    let mac_address =
        MacAddr::from_str(mac_address).map_err(|_| NetworkError::InvalidMacAddress {
            value: mac_address.to_owned(),
        })?;

    Ok(Host {
        ip_address,
        mac_address,
    })
}

pub fn find_host_by_ip(hosts: &[Host], target_ip: Ipv4Addr) -> Option<&Host> {
    hosts
        .iter()
        .find(|host| host.ip_address == IpAddr::V4(target_ip))
}

pub fn parse_arp_request(frame: &[u8]) -> Result<Option<ArpRequest>, NetworkError> {
    let arp_offset = MutableEthernetPacket::minimum_packet_size();
    let arp_bytes = frame.get(arp_offset..).ok_or(NetworkError::InvalidFrame)?;
    let arp_packet = ArpPacket::new(arp_bytes).ok_or(NetworkError::InvalidFrame)?;

    if arp_packet.get_operation() != ArpOperations::Request {
        return Ok(None);
    }

    Ok(Some(ArpRequest {
        sender_ip: arp_packet.get_sender_proto_addr(),
        sender_mac: arp_packet.get_sender_hw_addr(),
        target_ip: arp_packet.get_target_proto_addr(),
    }))
}

pub fn resolve_arp_reply(request: &ArpRequest, hosts: &[Host]) -> Option<ArpReply> {
    let host = find_host_by_ip(hosts, request.target_ip)?;

    Some(ArpReply {
        sender_ip: request.target_ip,
        sender_mac: host.mac_address,
        target_ip: request.sender_ip,
        target_mac: request.sender_mac,
    })
}

pub fn process_arp_frame(frame: &[u8], hosts: &[Host]) -> Result<Option<ArpReply>, NetworkError> {
    let Some(request) = parse_arp_request(frame)? else {
        return Ok(None);
    };

    Ok(resolve_arp_reply(&request, hosts))
}

pub fn build_arp_reply_packet(reply: &ArpReply) -> Result<[u8; ETHERNET_FRAME_SIZE], NetworkError> {
    let mut ethernet_buffer = [0u8; ETHERNET_FRAME_SIZE];
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer)
        .ok_or(NetworkError::PacketBuild { layer: "ethernet" })?;

    ethernet_packet.set_destination(reply.target_mac);
    ethernet_packet.set_source(reply.sender_mac);
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer = [0u8; ARP_FRAME_SIZE];
    let mut arp_packet =
        MutableArpPacket::new(&mut arp_buffer).ok_or(NetworkError::PacketBuild { layer: "arp" })?;

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Reply);
    arp_packet.set_sender_hw_addr(reply.sender_mac);
    arp_packet.set_sender_proto_addr(reply.sender_ip);
    arp_packet.set_target_hw_addr(reply.target_mac);
    arp_packet.set_target_proto_addr(reply.target_ip);

    ethernet_packet.set_payload(arp_packet.packet_mut());

    Ok(ethernet_buffer)
}

fn send_arp_reply(
    interface: &datalink::NetworkInterface,
    reply: &ArpReply,
) -> Result<(), NetworkError> {
    let packet = build_arp_reply_packet(reply)?;

    let (mut sender, _receiver) = match datalink::channel(interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => return Err(NetworkError::UnsupportedChannel),
        Err(error) => {
            return Err(NetworkError::ChannelOpen {
                message: error.to_string(),
            })
        }
    };

    match sender.send_to(&packet, None) {
        Some(Ok(_)) => Ok(()),
        Some(Err(error)) => Err(NetworkError::ChannelSend {
            message: error.to_string(),
        }),
        None => Err(NetworkError::ChannelSend {
            message: "datalink sender dropped the packet".to_owned(),
        }),
    }
}

pub fn respond_arp_queries(interface_name: &str, hosts: &[Host]) -> Result<(), NetworkError> {
    let source_interface = datalink::interfaces()
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound {
            name: interface_name.to_owned(),
        })?;

    let (_sender, mut receiver) = match datalink::channel(&source_interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => return Err(NetworkError::UnsupportedChannel),
        Err(error) => {
            return Err(NetworkError::ChannelOpen {
                message: error.to_string(),
            })
        }
    };

    log::info!(
        "Listening for ARP requests on {} with {} configured host(s)",
        interface_name,
        hosts.len()
    );

    loop {
        let frame = match receiver.next() {
            Ok(frame) => frame,
            Err(error) => {
                log::warn!("Failed to read frame: {error}");
                continue;
            }
        };

        match process_arp_frame(frame, hosts) {
            Ok(Some(reply)) => {
                log::debug!(
                    "ARP request for {} from {} ({})",
                    reply.sender_ip,
                    reply.target_ip,
                    reply.target_mac
                );

                if let Err(error) = send_arp_reply(&source_interface, &reply) {
                    log::error!("Failed to send ARP reply: {error}");
                    return Err(error);
                }

                log::debug!(
                    "Sent ARP reply for {} to {} ({})",
                    reply.sender_ip,
                    reply.target_ip,
                    reply.target_mac
                );
            }
            Ok(None) => {}
            Err(NetworkError::InvalidFrame) => {
                log::warn!("Ignoring malformed Ethernet or ARP frame");
            }
            Err(error) => {
                log::warn!("Ignoring frame due to error: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
    use pnet::packet::ethernet::MutableEthernetPacket;

    fn host(ip: &str, mac: &str) -> Host {
        parse_host(ip, mac).expect("valid host")
    }

    fn arp_request_frame(
        sender_ip: Ipv4Addr,
        sender_mac: MacAddr,
        target_ip: Ipv4Addr,
    ) -> [u8; ETHERNET_FRAME_SIZE] {
        let mut ethernet_buffer = [0u8; ETHERNET_FRAME_SIZE];
        let mut ethernet_packet =
            MutableEthernetPacket::new(&mut ethernet_buffer).expect("ethernet packet should fit");

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(sender_mac);
        ethernet_packet.set_ethertype(EtherTypes::Arp);

        let mut arp_buffer = [0u8; ARP_FRAME_SIZE];
        let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).expect("arp packet should fit");

        arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp_packet.set_protocol_type(EtherTypes::Ipv4);
        arp_packet.set_hw_addr_len(6);
        arp_packet.set_proto_addr_len(4);
        arp_packet.set_operation(ArpOperations::Request);
        arp_packet.set_sender_hw_addr(sender_mac);
        arp_packet.set_sender_proto_addr(sender_ip);
        arp_packet.set_target_hw_addr(MacAddr::zero());
        arp_packet.set_target_proto_addr(target_ip);

        ethernet_packet.set_payload(arp_packet.packet_mut());

        ethernet_buffer
    }

    #[test]
    fn parse_host_should_parse_valid_addresses() {
        let parsed = parse_host("192.168.1.100", "01:23:45:67:89:ab").expect("valid host");

        assert_eq!(
            parsed.ip_address,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))
        );
    }

    #[test]
    fn parse_host_should_fail_for_invalid_mac() {
        let error = parse_host("192.168.1.100", "invalid").expect_err("invalid mac");

        assert_eq!(
            error,
            NetworkError::InvalidMacAddress {
                value: "invalid".to_owned()
            }
        );
    }

    #[test]
    fn find_host_by_ip_should_return_matching_host() {
        let hosts = vec![
            host("192.168.1.100", "01:23:45:67:89:ab"),
            host("192.168.1.101", "cd:ef:12:34:56:78"),
        ];

        let found = find_host_by_ip(&hosts, Ipv4Addr::new(192, 168, 1, 101));

        assert_eq!(
            found.map(|host| host.mac_address),
            Some(MacAddr::new(0xcd, 0xef, 0x12, 0x34, 0x56, 0x78))
        );
    }

    #[test]
    fn process_arp_frame_should_return_reply_for_configured_target() {
        let frame = arp_request_frame(
            Ipv4Addr::new(192, 168, 1, 10),
            MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01),
            Ipv4Addr::new(192, 168, 1, 100),
        );
        let hosts = vec![host("192.168.1.100", "01:23:45:67:89:ab")];

        let reply = process_arp_frame(&frame, &hosts)
            .expect("frame should parse")
            .expect("request should match");

        assert_eq!(reply.sender_ip, Ipv4Addr::new(192, 168, 1, 100));
    }

    #[test]
    fn process_arp_frame_should_ignore_unrelated_request() {
        let frame = arp_request_frame(
            Ipv4Addr::new(192, 168, 1, 10),
            MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01),
            Ipv4Addr::new(192, 168, 1, 200),
        );
        let hosts = vec![host("192.168.1.100", "01:23:45:67:89:ab")];

        let reply = process_arp_frame(&frame, &hosts).expect("frame should parse");

        assert!(reply.is_none());
    }

    #[test]
    fn build_arp_reply_packet_should_encode_expected_addresses() {
        let reply = ArpReply {
            sender_ip: Ipv4Addr::new(192, 168, 1, 100),
            sender_mac: MacAddr::new(0x01, 0x23, 0x45, 0x67, 0x89, 0xab),
            target_ip: Ipv4Addr::new(192, 168, 1, 10),
            target_mac: MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01),
        };

        let packet = build_arp_reply_packet(&reply).expect("packet should build");
        let arp_packet = ArpPacket::new(&packet[MutableEthernetPacket::minimum_packet_size()..])
            .expect("packet should contain an arp frame");

        assert_eq!(arp_packet.get_operation(), ArpOperations::Reply);
        assert_eq!(arp_packet.get_sender_proto_addr(), reply.sender_ip);
    }
}
