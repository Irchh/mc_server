use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use log::*;
use crate::datatypes::MCString;
use crate::error::ServerError;
use crate::packet::{ConfigurationPacketResponse, ConfigurationPacketType, HandshakePacketType, LoginPacketResponse, LoginPacketType, StatusPacketType};
use crate::packet_builder::PacketBuilder;
use crate::server_util::{ServerInfo, ServerConnectionThreadBound, ServerMainThreadBound};

#[derive(Debug, Clone)]
pub enum ConnectionStatusType {
    Handshake,
    Status,
    Login,
    Transfer, // what does this do?
    Configuration,
    Play,
}

pub struct MCServerConnection {
    connection: TcpStream,
    pretty_identifier: String,
    state: ConnectionStatusType,
    sender: Sender<ServerMainThreadBound>,
    receiver: Receiver<ServerConnectionThreadBound>,
    server_info: ServerInfo,
}

impl MCServerConnection {
    pub fn new(connection: TcpStream, sender: Sender<ServerMainThreadBound>, receiver: Receiver<ServerConnectionThreadBound>, server_info: ServerInfo) -> Self {
        connection.set_nonblocking(true).unwrap();
        Self {
            pretty_identifier: connection.peer_addr().map(|a| {a.to_string()}).unwrap_or("UNKNOWN".to_string()),
            connection,
            state: ConnectionStatusType::Handshake,
            sender,
            receiver,
            server_info,
        }
    }

    fn send_packet(&mut self, packet: Vec<u8>) {
        trace!("Sending: {packet:02X?}");
        self.connection.write(packet.as_slice()).unwrap();
    }

    pub fn run(mut self) {
        let mut raw_data = [0; 32767]; // Max client to server packet size;
        'outer: loop {
            match self.connection.read(&mut raw_data) {
                Ok(size) => {
                    trace!("{}: Raw data: {:02X?}", self.pretty_identifier, &raw_data[0..size]);
                    if size == 0 {
                        info!("{}: Ending connection", self.pretty_identifier);
                        break 'outer;
                    }
                    let data = raw_data[0..size].to_vec();
                    match self.state {
                        ConnectionStatusType::Handshake => {
                            if let Err(err) = self.handle_handshake_packet(data) {
                                error!("Invalid handshake packet next_state: {}", err);
                            }
                        }
                        ConnectionStatusType::Status => {
                            self.handle_status_packet(data).unwrap();
                        }
                        ConnectionStatusType::Login => {
                            self.handle_login_packet(data).unwrap();
                        }
                        ConnectionStatusType::Configuration => {
                            self.handle_config_packet(data).unwrap();
                        }
                        _ => {
                            error!("Server state not yet implemented: {:?}", self.state);
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

    fn handle_handshake_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = HandshakePacketType::parse(data).unwrap();
        debug!("Parsed handshake packet: {:?}", packet);
        match packet {
            HandshakePacketType::Handshake { protocol:_, server_addr:_, server_port:_, next_state } => {
                debug!("{}: New connection. Next state: {}", self.pretty_identifier, next_state);
                let state = match next_state {
                    0 => ConnectionStatusType::Handshake, // a bit weird but ok
                    1 => ConnectionStatusType::Status,
                    2 => ConnectionStatusType::Login,
                    3 => ConnectionStatusType::Transfer,
                    _ => {
                        return Err(ServerError::InvalidServerState(next_state));
                    }
                };
                self.state = state;
                Ok(())
            }
        }
    }

    fn handle_status_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = StatusPacketType::parse(data).unwrap();
        debug!("Parsed status packet: {:?}", packet);
        match packet {
            StatusPacketType::Status => {
                let status_json = serde_json::to_string(&self.server_info).unwrap();
                let packet = PacketBuilder::new()
                    .set_id(StatusPacketType::StatusResponse)
                    .add_string(status_json)
                    .build()
                    .unwrap();
                self.send_packet(packet);
                Ok(())
            }
            StatusPacketType::Ping { raw } => {
                self.send_packet(raw);
                Ok(())
            }
            // Wont happen
            StatusPacketType::StatusResponse => {
                Ok(())
            }
        }
    }

    fn handle_login_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = LoginPacketType::parse(data).unwrap();
        debug!("Parsed login packet: {:?}", packet);
        match packet {
            LoginPacketType::LoginStart { name, uuid } => {
                info!("Login from {}. Name: {} (UUID: {})", self.pretty_identifier, name, uuid.hyphenated());
                self.pretty_identifier = name.clone();
                let packet = PacketBuilder::new()
                    .set_id(LoginPacketResponse::LoginSuccess)
                    .add_uuid(uuid)
                    .add_string(name)
                    .add_varint(0)
                    .add_bool(false)
                    .build()
                    .unwrap();
                self.send_packet(packet);
                Ok(())
            }
            // Wont happen
            LoginPacketType::LoginPluginResponse { message_id, success, data } => {
                debug!("LoginPluginResponse {{ {message_id:?}, {success:?}, {data:?} }}");
                Ok(())
            }
            LoginPacketType::LoginAcknowledged => {
                self.state = ConnectionStatusType::Configuration;
                Ok(())
            }
        }
    }

    fn handle_config_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = ConfigurationPacketType::parse(data).unwrap();
        debug!("Parsed configuration packet: {:?}", packet);
        match packet {
            ConfigurationPacketType::ServerBoundPluginMessage { channel, data } => {
                let data_string = MCString::from(data.clone()).map(|s| s.value).unwrap_or_else(|_err| { format!("{data:?}") });
                info!("[Plugin Message from {}]: {} = {:?}", self.pretty_identifier, channel, data_string);
                match &*channel {
                    _ => {}
                }
                Ok(())
            }
            ConfigurationPacketType::ClientInformation { .. } => {
                let packet = PacketBuilder::new()
                    .set_id(ConfigurationPacketResponse::FinishConfiguration)
                    .build()
                    .unwrap();
                self.send_packet(packet);
                Ok(())
            }
            ConfigurationPacketType::FinishConfigurationAck => {
                self.state = ConnectionStatusType::Play;
                Ok(())
            }
        }
    }
}