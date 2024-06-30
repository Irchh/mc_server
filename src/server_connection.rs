use std::cmp::{Ordering, PartialEq};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use log::*;
use mc_world_parser::Position;
use rand::random;
use mc_datatypes::{BlockPos, MCString, VarInt};
use crate::block_registry::BlockRegistry;
use crate::command::CommandNode;
use crate::error::ServerError;
use crate::packet::{ConfigurationPacketResponse, ConfigurationPacketType, HandshakePacketType, LoginPacketResponse, LoginPacketType, PlayPacketClientBound, PlayPacketServerBound, StatusPacketType};
use crate::packet_builder::PacketBuilder;
use crate::server_util::{ServerInfo, ServerConnectionThreadBound, ServerMainThreadBound};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatusType {
    Handshake,
    Status,
    Login,
    Transfer, // what does this do?
    Configuration,
    Play,
}

pub struct Player {
    eid: i32,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
    confirm_tp_count: u32,
}

impl Player {
    pub fn new(eid: i32) -> Self {
        Self {
            eid,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            confirm_tp_count: 0,
        }
    }
    pub fn set_pos(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    pub fn set_yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
    }

    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.on_ground = on_ground;
    }

    pub fn block_pos(&self) -> BlockPos {
        let x = self.x.floor() as i32;
        let y = self.y.floor() as i32;
        let z = self.z.floor() as i32;
        BlockPos::new(x, y, z)
    }
}

pub struct MCServerConnection {
    connection: TcpStream,
    pretty_identifier: String,
    state: ConnectionStatusType,
    sender: Sender<ServerMainThreadBound>,
    receiver: Receiver<ServerConnectionThreadBound>,
    server_info: ServerInfo,
    packet_buffer: Vec<u8>,
    block_registry: BlockRegistry,
    client_loaded_chunks: Vec<Position>,
    player: Player,
    last_tick: SystemTime,
    waiting_for_confirm_teleport: Option<i32>,
    view_distance: i32
}

impl MCServerConnection {
    pub fn new(connection: TcpStream, sender: Sender<ServerMainThreadBound>, receiver: Receiver<ServerConnectionThreadBound>, server_info: ServerInfo, block_registry: BlockRegistry) -> Self {
        connection.set_nonblocking(true).unwrap();
        Self {
            pretty_identifier: connection.peer_addr().map(|a| {a.to_string()}).unwrap_or("UNKNOWN".to_string()),
            connection,
            state: ConnectionStatusType::Handshake,
            sender,
            receiver,
            server_info,
            packet_buffer: vec![],
            block_registry,
            client_loaded_chunks: vec![],
            player: Player::new(random()),
            last_tick: SystemTime::UNIX_EPOCH,
            waiting_for_confirm_teleport: None,
            view_distance: 12,
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
                            if self.packet_buffer.len() >= packet_size.value as usize + packet_size.bytes.len() {
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
                        let _= self.connection.shutdown(Shutdown::Both);
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
                        ServerConnectionThreadBound::TagInfo(tags) => {
                            self.send_packet(ConfigurationPacketResponse::update_tags(tags));
                            self.sender.send(ServerMainThreadBound::RequestRegistryInfo).unwrap();
                        }
                        ServerConnectionThreadBound::RegistryInfoFinished => {
                            // Next stage ig
                            let packet = PacketBuilder::new()
                                .set_id(ConfigurationPacketResponse::FinishConfiguration)
                                .build()
                                .unwrap();
                            self.send_packet(packet);
                        }
                        ServerConnectionThreadBound::ChunkData(chunk) => {
                            if let Some(chunk) = chunk {
                                self.send_packet(PlayPacketClientBound::chunk_data(chunk, Box::new(self.block_registry.clone())));
                            }
                        }
                        ServerConnectionThreadBound::ChatMessage { player_name, message, timestamp, salt } => {
                            self.send_packet(PlayPacketClientBound::player_chat_message_fake(player_name, message));
                        }
                    }
                }
                Err(_) => {}
            }

            if self.state == ConnectionStatusType::Play {
                self.handle_chunk_loading();
                self.handle_ticks();
            }
        }
    }

    fn handle_chunk_loading(&mut self) {
        // TODO: Handle position ig
        let mut chunk_to_load = vec![];
        let player_pos = self.player.block_pos();
        let player_x = (player_pos.x() - (player_pos.x() < 0) as i32)/16;
        let player_z = (player_pos.z() - (player_pos.z() < 0) as i32)/16;
        for x in -self.view_distance..self.view_distance {
            for z in -self.view_distance..self.view_distance {
                chunk_to_load.push(Position::new(x + player_x, 0, z + player_z));
            }
        }

        chunk_to_load.sort_by(|c1, c2| {
            let x_diff1 = player_x.abs_diff(c1.x) as i32;
            let z_diff1 = player_z.abs_diff(c1.z) as i32;

            let x_diff2 = player_x.abs_diff(c2.x) as i32;
            let z_diff2 = player_z.abs_diff(c2.z) as i32;

            let length1 = x_diff1.max(z_diff1);
            let length2 = x_diff2.max(z_diff2);

            if length1 < length2 {
                Ordering::Less
            } else if length1 > length2 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        for chunk in chunk_to_load {
            if !self.client_loaded_chunks.contains(&chunk) {
                self.client_loaded_chunks.push(chunk);
                self.sender.send(ServerMainThreadBound::RequestChunk(chunk)).unwrap();
            }
        }
        // Keep track of chunks the client will unload
        for i in (0..self.client_loaded_chunks.len()).rev() {
            let chunkpos = self.client_loaded_chunks.get(i).unwrap();
            let x_diff = player_x.abs_diff(chunkpos.x) as i32;
            let z_diff = player_z.abs_diff(chunkpos.z) as i32;
            let length = x_diff.max(z_diff);
            // Cube of size view_distance * 2 + 7
            if length > self.view_distance + 3 {
                // Safe since we start from the end
                self.client_loaded_chunks.remove(i);
            }
        }
    }

    fn handle_ticks(&mut self) {
        if self.last_tick == SystemTime::UNIX_EPOCH {
            // Send ticking state
            let packet = PacketBuilder::new()
                .set_id(PlayPacketClientBound::SetTickingState)
                .add_float(20f32)
                .add_bool(false)
                .build().unwrap();
            self.send_packet(packet);
            // Send first tick
            self.last_tick = SystemTime::now();
            let packet = PacketBuilder::new()
                .set_id(PlayPacketClientBound::StepTick)
                .add_varint(1)
                .build().unwrap();
            self.send_packet(packet);
        } else {
            let curr_time = SystemTime::now();
            if let Ok(duration) = curr_time.duration_since(self.last_tick) {
                if duration.as_secs_f64() > 1.0/20.0 {
                    let packet = PacketBuilder::new()
                        .set_id(PlayPacketClientBound::StepTick)
                        .add_varint(1)
                        .build().unwrap();
                    self.send_packet(packet);
                    self.last_tick = curr_time;
                }
            } else {
                self.last_tick = curr_time;
            }
        }
    }

    fn confirm_teleport(&mut self) {
        // Send sync player pos
        let confirm_id = self.player.confirm_tp_count;
        self.player.confirm_tp_count += 1;
        //sleep(Duration::from_secs_f64(0.1));
        let (new_y, mask) = if self.player.y < -80f64 {
            (-64f64, 0x1D)
        } else {
            (0f64, 0x1F)
        };
        let packet = PacketBuilder::new()
            .set_id(PlayPacketClientBound::SyncPlayerPosition)
            .add_double(0f64)
            .add_double(new_y)
            .add_double(0f64)
            .add_float(0f32)
            .add_float(0f32)
            .add_byte(mask)
            .add_varint(confirm_id as i32)
            ;
        self.waiting_for_confirm_teleport = Some(confirm_id as i32);
        self.send_packet(packet.build().unwrap())
    }

    fn play_mode_initialize_client(&mut self) {
        // Sends all required packets for clients to connect that don't get sent on different signals
        self.send_packet(PlayPacketClientBound::login(self.player.eid, false, vec!["minecraft:overworld".to_string(), "minecraft:the_end".to_string(), "minecraft:the_nether".to_string()], 20, self.view_distance));
        self.send_packet(PlayPacketClientBound::change_difficulty(1));
        self.send_packet(PlayPacketClientBound::commands(CommandNode::commands()));
        self.send_packet(PlayPacketClientBound::player_abilities());
        self.send_packet(PlayPacketClientBound::set_held_item(0));
        //self.send_packet(PlayPacketClientBound::set_recipes());
        self.send_packet(PlayPacketClientBound::entity_event(self.player.eid, 24));
        self.send_packet(PlayPacketClientBound::entity_effect(self.player.eid, 15, 1, 0x7F, 0x07));
    }

    fn set_pos(&mut self, x: f64, y: f64, z: f64) {
        let mut old_blockpos = self.player.block_pos();
        self.player.set_pos(x, y, z);
        let mut new_blockpos = self.player.block_pos();
        old_blockpos.set_y(0);
        new_blockpos.set_y(0);
        if old_blockpos.x()/16 != new_blockpos.x()/16 || old_blockpos.z()/16 != new_blockpos.z()/16 {
            self.send_packet(PlayPacketClientBound::set_center_chunk(new_blockpos))
        }
        //self.handle_chunk_loading();
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
            ConfigurationPacketType::ClientInformation { view_distance, .. } => {
                self.view_distance = view_distance.min(12) as i32;
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
                self.play_mode_initialize_client();
                let _= self.sender.send(ServerMainThreadBound::ChatMessage { player_name: self.pretty_identifier.clone(), message: "Joined the game".to_string(), timestamp: 0, salt: 0 });
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
                self.sender.send(ServerMainThreadBound::RequestTagInfo).unwrap();
                Ok(())
            }
        }
    }

    fn handle_play_packet(&mut self, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = PlayPacketServerBound::parse(data)?;
        trace!("Parsed play packet: {:?}", packet);
        match packet {
            PlayPacketServerBound::ConfirmTeleportation { id } => {
                if self.waiting_for_confirm_teleport == Some(id) {
                    self.waiting_for_confirm_teleport = None;
                }
            }
            PlayPacketServerBound::ChatCommand { command } => {
                info!("{} ran the command: {}", self.pretty_identifier, command);
                let block_state = command[6..].parse::<i32>().unwrap();
                self.send_packet(PlayPacketClientBound::block_update(block_state, self.player.block_pos()));
            }
            PlayPacketServerBound::ClientInformation { view_distance, .. } => {
                self.view_distance = view_distance.min(12) as i32;
            }
            PlayPacketServerBound::ChatMessage { message, timestamp, salt, .. } => {
                info!("[CHAT] <{}>: {}", self.pretty_identifier, message);
                let _= self.sender.send(ServerMainThreadBound::ChatMessage { player_name: self.pretty_identifier.clone(), message, timestamp, salt });
            }
            PlayPacketServerBound::CloseContainer( .. ) => {}
            PlayPacketServerBound::DebugSampleSubscription{ .. } => {}
            PlayPacketServerBound::SetPlayerPosition { x, y, z, on_ground } => {
                self.set_pos(x, y, z);
                self.player.set_on_ground(on_ground);
                if self.waiting_for_confirm_teleport.is_none() {
                    self.confirm_teleport();
                }
                trace!("pos: {x} / {y} / {z}, on_ground: {on_ground}")
            }
            PlayPacketServerBound::SetPlayerPositionAndRotation { x, y, z, yaw, pitch, on_ground } => {
                self.set_pos(x, y, z);
                self.player.set_yaw_pitch(yaw, pitch);
                self.player.set_on_ground(on_ground);
                if self.waiting_for_confirm_teleport.is_none() {
                    self.confirm_teleport();
                }
                trace!("pos: {x} / {y} / {z}, yaw: {yaw}, pitch: {pitch}, on_ground: {on_ground}")
            }
            PlayPacketServerBound::SetPlayerRotation { yaw, pitch, on_ground } => {
                self.player.set_yaw_pitch(yaw, pitch);
                self.player.set_on_ground(on_ground);
            }
            PlayPacketServerBound::SetPlayerOnGround(on_ground) => {
                self.player.set_on_ground(on_ground);
            }
            PlayPacketServerBound::PingRequest(ping_id) => {
                let packet = PacketBuilder::new()
                    .set_id(PlayPacketClientBound::PingResponse)
                    .add_long(ping_id)
                    .build().unwrap();
                self.send_packet(packet);
            }
            PlayPacketServerBound::PlayerAbilities { .. } => {}
            PlayPacketServerBound::PlayerAction { .. } => {}
            PlayPacketServerBound::PlayerCommand { .. } => {}
            PlayPacketServerBound::SetHeldItem { .. } => {}
            PlayPacketServerBound::SetCreativeModeSlot { slot, clicked_item } => {
                debug!("Set slot {slot} to {:?}", clicked_item.item_id)
            }
            PlayPacketServerBound::SwingArm { .. } => {}
            PlayPacketServerBound::UseItemOn { .. } => {}
            PlayPacketServerBound::UseItem { .. } => {}
        }
        Ok(())
    }
}