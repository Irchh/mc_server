use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use log::*;
use crate::packet::{HandshakePacketType, StatusPacketType};
use crate::packet_builder::PacketBuilder;
use crate::server_util::{ServerInfo, ServerConnectionThreadBound, ServerMainThreadBound};

pub enum ConnectionStatusType {
    Status,
    Login,
    Play,
}

pub struct MCServerConnection {
    connection: TcpStream,
    connection_string: String,
    connection_status: ConnectionStatusType,
    sender: Sender<ServerMainThreadBound>,
    receiver: Receiver<ServerConnectionThreadBound>,
    state: i32,
    server_info: ServerInfo,
}

impl MCServerConnection {
    pub fn new(connection: TcpStream, sender: Sender<ServerMainThreadBound>, receiver: Receiver<ServerConnectionThreadBound>, server_info: ServerInfo) -> Self {
        connection.set_nonblocking(true).unwrap();
        Self {
            connection_string: connection.peer_addr().map(|a| {a.to_string()}).unwrap_or("UNKNOWN".to_string()),
            connection,
            connection_status: ConnectionStatusType::Status,
            sender,
            receiver,
            state: 0,
            server_info,
        }
    }

    pub fn run(mut self) {
        let mut raw_data = [0; 32767]; // Max client to server packet size;
        'outer: loop {
            match self.connection.read(&mut raw_data) {
                Ok(size) => {
                    trace!("{}: Raw data: {:02X?}", self.connection_string, &raw_data[0..size]);
                    if size == 0 {
                        info!("{}: Ending connection", self.connection_string);
                        break 'outer;
                    }
                    let data = raw_data[0..size].to_vec();
                    match self.state {
                        0 => {
                            let packet = HandshakePacketType::parse(data).unwrap();
                            match packet {
                                HandshakePacketType::Handshake { protocol:_, server_addr:_, server_port:_, next_state } => {
                                    debug!("{}: New connection. Next state: {}", self.connection_string, next_state);
                                    self.state = next_state;
                                }
                            }
                        }
                        1 => {
                            let packet = StatusPacketType::parse(data).unwrap();
                            match packet {
                                StatusPacketType::Status => {
                                    let status_json = serde_json::to_string(&self.server_info).unwrap();
                                    let packet = PacketBuilder::new()
                                        .set_state(ConnectionStatusType::Status)
                                        .set_id(StatusPacketType::StatusResponse)
                                        .add_string(status_json)
                                        .build()
                                        .unwrap();
                                    trace!("Responding with: {:02X?}", packet);
                                    self.connection.write(packet.as_slice()).unwrap();
                                }
                                StatusPacketType::Ping { raw } => {
                                    self.connection.write(raw.as_slice()).unwrap();
                                }
                                // Wont happen
                                StatusPacketType::StatusResponse => {}
                            }
                        }
                        _ => {
                            error!("Invalid server state: {}", self.state);
                            break 'outer;
                        }
                    }
                }
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::WouldBlock => {}
                        _ => {
                            error!("Error receiving data from {}: {}", self.connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()), err);
                            self.connection.shutdown(Shutdown::Both).unwrap();
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
}