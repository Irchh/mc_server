use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use log::*;
use crate::datatypes::{MCString, VarInt};
use crate::error::ServerError;
use crate::packet::{ConfigurationPacketResponse, ConfigurationPacketType, HandshakePacketType, LoginPacketResponse, LoginPacketType, PlayPacketClientBound, PlayPacketServerBound, StatusPacketType};
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
    packet_buffer: Vec<u8>,
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
            packet_buffer: vec![],
        }
    }

    fn send_packet(&mut self, packet: Vec<u8>) {
        trace!("Sending: {packet:02X?}");
        self.connection.write(packet.as_slice()).unwrap();
    }

    fn sync_player_pos(&mut self) {
        let packet = PacketBuilder::new()
            .set_id(PlayPacketClientBound::SyncPlayerPosition)
            .add_double(0f64)
            .add_double(0f64)
            .add_double(0f64)
            .add_float(0f32)
            .add_float(0f32)
            .add_byte(0x1F)
            .add_varint(0)
            .build()
            .unwrap();
        self.send_packet(packet);
    }

    pub fn run(mut self) {
        let mut raw_data = [0; 32768]; // Max client to server packet size;
        'outer: loop {
            match self.connection.read(&mut raw_data) {
                Ok(size) => {
                    trace!("{}: Raw data: {:02X?}", self.pretty_identifier, &raw_data[0..size]);
                    if size == 0 {
                        info!("{}: Ending connection", self.pretty_identifier);
                        break 'outer;
                    }
                    self.packet_buffer.append(&mut raw_data[0..size].to_vec());
                    loop {
                        // Cloning here is inefficient but heck
                        if let Ok(packet_size) = VarInt::from(self.packet_buffer.clone()) {
                            if self.packet_buffer.len() >= packet_size.value as usize {
                                let packet = self.packet_buffer.drain(0..(packet_size.value as usize + packet_size.bytes.len())).collect();
                                self.handle_packet(packet).unwrap();
                                continue;
                            }
                        }
                        break;
                    }
                }
                Err(err) => {
                    if err.kind() != ErrorKind::WouldBlock {
                        error!("Error receiving data from {}: {}", self.connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()), err);
                        self.connection.shutdown(Shutdown::Both).unwrap();
                        break 'outer;
                    }
                }
            }
            match self.receiver.try_recv() {
                Ok(message) => {
                    match message {
                        ServerConnectionThreadBound::RegistryInfo { registry_id, entries } => {
                            self.send_packet(ConfigurationPacketResponse::registry_data(registry_id, entries));
                        }
                        ServerConnectionThreadBound::RegistryInfoFinished => {
                            // Next stage ig
                            let packet = PacketBuilder::new()
                                .set_id(ConfigurationPacketResponse::FinishConfiguration)
                                .build()
                                .unwrap();
                            self.send_packet(packet);
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn handle_packet(&mut self, packet: Vec<u8>) -> Result<(), ServerError> {
        match self.state {
            ConnectionStatusType::Handshake => {
                if let Err(err) = self.handle_handshake_packet(packet) {
                    error!("Invalid handshake packet next_state: {}", err);
                    return Err(err);
                }
            }
            ConnectionStatusType::Status => {
                self.handle_status_packet(packet).unwrap();
            }
            ConnectionStatusType::Login => {
                self.handle_login_packet(packet).unwrap();
            }
            ConnectionStatusType::Configuration => {
                self.handle_config_packet(packet).unwrap();
            }
            ConnectionStatusType::Play => {
                self.handle_play_packet(packet).unwrap();
            }
            _ => {
                error!("Server state not yet implemented: {:?}", self.state);
                return Err(ServerError::ServerStateNotImplemented(self.state.clone()));
            }
        }
        Ok(())
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
                    .set_id(ConfigurationPacketResponse::ClientBoundKnownPacks)
                    .add_varint(1)
                    .add_string("minecraft")
                    .add_string("core")
                    .add_string("1.21")
                    .build()
                    .unwrap();
                self.send_packet(packet);
                Ok(())
            }
            ConfigurationPacketType::FinishConfigurationAck => {
                self.state = ConnectionStatusType::Play;
                debug!("Going into Play state");
                self.send_packet(PlayPacketClientBound::login(1, false, vec!["minecraft:overworld".to_string(), "minecraft:the_end".to_string(), "minecraft:the_nether".to_string()], 20, 10));
                Ok(())
            }
            ConfigurationPacketType::ServerBoundKnownPacks { known_packs } => {
                if let Some(pack) = known_packs.first() {
                    if !(pack.namespace == "minecraft" && pack.id == "core" && pack.version == "1.21") {
                        panic!("Server and client known pack mismatch");
                    }
                } else {
                    panic!("Client known packs empty");
                }
                self.sender.send(ServerMainThreadBound::RequestRegistryInfo).unwrap();
                Ok(())
            }
        }
    }

    fn handle_play_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = PlayPacketServerBound::parse(data)?;
        debug!("Parsed play packet: {:?}", packet);
        match packet {

        }
        Ok(())
    }
}