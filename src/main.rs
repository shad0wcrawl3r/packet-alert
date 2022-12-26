#![allow(non_upper_case_globals)]
// extern crate pnet;
use chrono::{self, Local};
use core::fmt;
use std::hash::Hash;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ip::IpNextHeaderProtocols::{Tcp, Udp};
use pnet::packet::Packet;
use pnet::packet::{ipv4::Ipv4Packet, ipv6::Ipv6Packet};
use pnet::packet::{tcp::TcpPacket, udp::UdpPacket};
use pnet_datalink::Channel::Ethernet;
use pnet_datalink::{self as datalink};
use std::collections::HashMap;
// use std::fs::read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
// use std::time::Duration;

struct NetFlow {
    src_ip: IpAddr,
    dst_ip: IpAddr,
    src_port: u16,
    dst_port: u16,
    protocol: IpNextHeaderProtocol,
}

impl NetFlow {
    fn new(
        src_ip: IpAddr,
        dst_ip: IpAddr,
        src_port: u16,
        dst_port: u16,
        protocol: IpNextHeaderProtocol,
    ) -> NetFlow {
        NetFlow {
            src_ip: src_ip,
            dst_ip: dst_ip,
            src_port: src_port,
            dst_port: dst_port,
            protocol: protocol,
        }
    }
}
impl fmt::Display for NetFlow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // code to specify how the struct should be displayed goes here
        write!(
            f,
            "{:?}:{} =={}==> {:?}:{}",
            self.src_ip, self.src_port, self.protocol, self.dst_ip, self.dst_port
        )
    }
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

fn handle_v4_packet(ethernet_packet: EthernetPacket, packet: &[u8]) -> NetFlow {
    let ipv4_packet = Ipv4Packet::new(ethernet_packet.payload()).unwrap();
    let src_ip: Ipv4Addr = ipv4_packet.get_source();
    let dst_ip: Ipv4Addr = ipv4_packet.get_destination();
    let protocol = ipv4_packet.get_next_level_protocol();
    let (src_port, dst_port) = resolve_targets(packet, protocol);
    // pretty_print(src_ip, dst_ip, src_port, dst_port, protocol);
    NetFlow::new(src_ip.into(), dst_ip.into(), src_port, dst_port, protocol)
}

fn handle_v6_packet(ethernet_packet: EthernetPacket, packet: &[u8]) -> NetFlow {
    let ipv6_packet = Ipv6Packet::new(ethernet_packet.payload()).unwrap();
    let src_ip: Ipv6Addr = ipv6_packet.get_source();
    let dst_ip: Ipv6Addr = ipv6_packet.get_destination();
    let protocol = ipv6_packet.get_next_header();
    let (src_port, dst_port) = resolve_targets(packet, protocol);
    NetFlow::new(src_ip.into(), dst_ip.into(), src_port, dst_port, protocol)
}

fn process_packet(packet: &[u8], reflections: &mut HashMap<IpAddr, i64>) {
    // Parse the Ethernet packet
    let ethernet_packet = EthernetPacket::new(packet).unwrap();
    match ethernet_packet.get_ethertype() {
        // Check the ethertype and handle the packet accordingly
        EtherTypes::Ipv4 => {
            let flow = handle_v4_packet(ethernet_packet, packet);
            println!("{}", flow);
            reflections.insert(flow.src_ip.try_into().unwrap(), Local::now().timestamp());
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
    println!("{}", ip);
}
fn check_timeouts(reflections: &mut HashMap<IpAddr, i64>) {}

fn main() {
    let reflections: Arc<RwLock<HashMap<IpAddr, i64>>> = Arc::new(RwLock::new(HashMap::new()));
    let (message_transmit,message_recieve) = mpsc::channel::<IpAddr>();
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
            let mut write_reflections = reflections.write().unwrap();
            for each_ip in message_recieve.iter(){
                write_reflections.remove(&each_ip);
            }
            
            match rx.next() {
                Ok(x) => process_packet(x, &mut write_reflections),
                Err(err) => println!("{:?}", err),
            }
        }
    });

    let reader = thread::spawn({
        
        let mut last_clean = Local::now().timestamp();
        move || loop {
        let mut alerted: Vec<IpAddr> = vec![];
        let read_reflections = reflections.read().unwrap();
        let now = Local::now().timestamp();
        for (ip, last_time) in read_reflections.iter() {
            if alerted.contains(ip) {
                continue;
            }
            let time_diff = now - last_time;
            if time_diff > 30 {
                alert(ip);
                alerted.push(*ip);
            }
        }
        for each_ip in alerted {
            message_transmit.send(each_ip).unwrap();
        }
    }});
    reader.join().unwrap();
    writer.join().unwrap();
}
