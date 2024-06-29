use inbt::NbtTag;
use mc_world_parser::chunk::Chunk;
use mc_world_parser::Position;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct VersionInfo {
    pub name: String,
    pub protocol: i32
}
#[derive(Serialize, Clone)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}
#[derive(Serialize, Clone)]
pub struct PlayerInfo {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<PlayerSample>
}
#[derive(Serialize, Clone)]
pub struct DescriptionInfo {
    pub text: String
}
#[derive(Serialize, Clone)]
pub struct ServerInfo {
    pub description: DescriptionInfo,
    pub players: PlayerInfo,
    pub version: VersionInfo,
    pub favicon: String
}

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: String,
    pub data: Option<NbtTag>,
}

#[derive(Debug, Clone)]
pub struct TagEntry {
    pub id: String,
    pub data: Option<NbtTag>,
}

pub enum ServerMainThreadBound {
    RequestRegistryInfo,
    RequestChunk(Position),
    ChatMessage { player_name: String, message: String, timestamp: i64, salt: i64, },
}

pub enum ServerConnectionThreadBound {
    RegistryInfo { registry_id: String, entries: Vec<RegistryEntry> },
    RegistryInfoFinished,
    ChunkData(Option<Chunk>),
    ChatMessage { player_name: String, message: String, timestamp: i64, salt: i64, },
}