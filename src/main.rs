mod packet;
mod packet_builder;
mod datatypes;
mod error;
mod server_util;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::packet::{PacketType, StatusPacketType};
use crate::packet_builder::PacketBuilder;
use crate::server_util::{DescriptionInfo, PlayerInfo, PlayerSample, ServerInfo, VersionInfo};

fn connection_handler(mut connection: TcpStream, _sender: Sender<i32>, _receiver: Receiver<i32>) {
    connection.set_nonblocking(true).unwrap();
    let mut raw_data = [0; 32767]; // Max client to server packet size;
    let mut next_handshake_state = 0;
    'outer: loop {
        match connection.read(&mut raw_data) {
            Ok(size) => {
                if size == 0 {
                    break 'outer;
                }
                println!("Raw data: {:02X?}", &raw_data[0..size]);
                let data = raw_data[0..size].to_vec();
                let packet = StatusPacketType::parse(data).unwrap();
                match packet {
                    StatusPacketType::Handshake { protocol, server_addr:_, server_port:_, next_state } => {
                        println!("Handshake from {}\n\tnext state: {}",
                                 connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()),
                                 next_state
                        );
                        if protocol == 0 {
                            if next_handshake_state == 1 {
                                let server_info = ServerInfo {
                                    description: DescriptionInfo { text: "RustMC 1.20.5 dev".to_string() },
                                    players: PlayerInfo {
                                        max: 100,
                                        online: 1,
                                        sample: vec![PlayerSample { name: "Irchh".to_string(), id: "00000000-0000-0000-0000-000000000000".to_string() }],
                                    },
                                    version: VersionInfo { name: "RustMC 1.20.5".to_string(), protocol: 766 },
                                    favicon: "data:image/png;base64,<data>".to_string(),
                                };
                                let status_json = serde_json::to_string(&server_info).unwrap();
                                let packet = PacketBuilder::new()
                                    .set_state(PacketType::Status)
                                    .set_id(StatusPacketType::StatusResponse)
                                    .add_string(status_json)
                                    .build()
                                    .unwrap();
                                println!("Responding with: {:02X?}", packet);
                                connection.write(packet.as_slice()).unwrap();
                            }
                        } else {
                            next_handshake_state = next_state;
                        }
                    }
                    StatusPacketType::Ping { raw } => {
                        connection.write(raw.as_slice()).unwrap();
                    }
                    _ => unimplemented!()
                }
            }
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::WouldBlock => {}
                    _ => {
                        println!("Error receiving data from {}: {}", connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()), err);
                        connection.shutdown(Shutdown::Both).unwrap();
                        break 'outer;
                    }
                }
            }
        }
    }
}

fn main() {
    let listen = "0.0.0.0:25565";
    let listener = TcpListener::bind(listen).unwrap();
    println!("Listening on: {}", listen);
    let mut threads = vec![];
    let mut channels = vec![];
    for stream in listener.incoming() {
        match stream {
            Ok(connection) => {
                println!("New connection from: {}", connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()));
                let ch_to_thread = std::sync::mpsc::channel();
                let ch_from_thread = std::sync::mpsc::channel();
                threads.push(thread::spawn(|| {
                    connection_handler(connection, ch_from_thread.0, ch_to_thread.1);
                }));
                channels.push((ch_to_thread.0, ch_from_thread.1));
            }
            Err(err) => eprintln!("Error accepting connection: {}", err),
        }
    }
}
