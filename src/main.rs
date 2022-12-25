#![allow(non_upper_case_globals)]
// extern crate pnet;
use chrono::{self, Local};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ip::IpNextHeaderProtocols::{Tcp, Udp};
use pnet::packet::Packet;
use pnet::packet::{ipv4::Ipv4Packet, ipv6::Ipv6Packet};
use pnet::packet::{tcp::TcpPacket, udp::UdpPacket};
use pnet_datalink::Channel::Ethernet;
use pnet_datalink::{self as datalink};
use std::collections::HashMap;
use std::fs::read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, Mutex, RwLock,mpsc};
use std::thread::{self, sleep, Builder as threadBuilder};
use std::time::Duration;

fn pretty_print<T: Into<IpAddr>>(
    src_ip: T,
    dst_ip: T,
    src_port: u16,
    dst_port: u16,
    protocol: IpNextHeaderProtocol,
) {
    println!(
        "{:?}:{} =={}==> {:?}:{}",
        src_ip.into(),
        src_port,
        protocol,
        dst_ip.into(),
        dst_port
    );
}

fn resolve_targets(packet: &[u8], protocol: IpNextHeaderProtocol) -> (u16, u16) {
    match protocol {
        Tcp => {
            let tcp_packet = TcpPacket::new(packet).unwrap();
            (tcp_packet.get_source(), tcp_packet.get_destination())
        }
        Udp => {
            let udp_packet = UdpPacket::new(packet).unwrap();
            (udp_packet.get_source(), udp_packet.get_destination())
        }
        _ => (0, 0),
    }
}

fn handle_v4_packet(
    ethernet_packet: EthernetPacket,
    packet: &[u8],
) -> (Ipv4Addr, Ipv4Addr, u16, u16, IpNextHeaderProtocol) {
    let ipv4_packet = Ipv4Packet::new(ethernet_packet.payload()).unwrap();
    let src_ip: Ipv4Addr = ipv4_packet.get_source();
    let dst_ip: Ipv4Addr = ipv4_packet.get_destination();
    let protocol = ipv4_packet.get_next_level_protocol();
    let (src_port, dst_port) = resolve_targets(packet, protocol);
    // pretty_print(src_ip, dst_ip, src_port, dst_port, protocol);
    (src_ip, dst_ip, src_port, dst_port, protocol)
}

fn handle_v6_packet(
    ethernet_packet: EthernetPacket,
    packet: &[u8],
) -> (Ipv6Addr, Ipv6Addr, u16, u16, IpNextHeaderProtocol) {
    let ipv6_packet = Ipv6Packet::new(ethernet_packet.payload()).unwrap();
    let src_ip: Ipv6Addr = ipv6_packet.get_source();
    let dst_ip: Ipv6Addr = ipv6_packet.get_destination();
    let protocol = ipv6_packet.get_next_header();
    let (src_port, dst_port) = resolve_targets(packet, protocol);
    (src_ip, dst_ip, src_port, dst_port, protocol)
}

fn process_packet(packet: &[u8], reflections: &mut HashMap<IpAddr, i64>) {
    // Parse the Ethernet packet
    let ethernet_packet = EthernetPacket::new(packet).unwrap();
    match ethernet_packet.get_ethertype() {
        // Check the ethertype and handle the packet accordingly
        EtherTypes::Ipv4 => {
            let (src_ip, dst_ip, src_port, dst_port, protocol) =
                handle_v4_packet(ethernet_packet, packet);
            reflections.insert(src_ip.try_into().unwrap(), Local::now().timestamp());
        }
        EtherTypes::Ipv6 => {
            // // This is me basically ignoring Ipv6 Packets
            // let (src_ip, dst_ip, src_port, dst_port, protocol) =
            //     handle_v6_packet(ethernet_packet, packet);
            // reflections.insert(src_ip.try_into().unwrap(), Local::now().timestamp());
        }
        _ => {
            // Unknown ethertype, skip this packet
        }
    }
}
fn alert(ip: &IpAddr) {
    println!("{}", ip)
}
fn check_timeouts(reflections: &mut HashMap<IpAddr, i64>) {}


fn main() { 
    let reflections: Arc<Mutex<HashMap<IpAddr, i64>>> = Arc::new(Mutex::new(HashMap::new()));
    // let (mptx,mprx) = mpsc::channel();
    // let mut reflections: HashMap<IpAddr, i64> = HashMap::new();
    let interface_name = "enp3s0";
    let interfaces = datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|i| i.name == interface_name && i.is_up())
        .expect("Failed to find network interface");
    println!("{:?}", interface);
    // // Open a channel to the network interface
    let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!(
            "An error occurred when creating the datalink channel: {}",
            e
        ),
    };
    let writer = thread::spawn({
        let reflections = Arc::clone(&reflections);
        move || loop {
            let mut write_reflections = reflections.lock().unwrap();
            match rx.next() {
                            Ok(x) => process_packet(x, &mut write_reflections),
                            Err(err) => println!("{:?}", err),
                        }
        }
    });

    let reader = thread::spawn(move || {
        loop {
            let read_reflections = reflections.lock().unwrap();
            for (ip, last_time) in read_reflections.iter() {
                let time_diff = Local::now().timestamp() - last_time;
                if time_diff > 30 {
                    alert(ip);
                }
            }
        }
    });

    reader.join().unwrap();
    writer.join().unwrap();
}