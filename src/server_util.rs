use inbt::NbtTag;
use mc_world_parser::chunk::Chunk;
use mc_world_parser::Position;
use serde::Serialize;
use crate::block_registry::BlockRegistry;

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
pub struct TagEntryData {
    pub entries: Vec<i32>,
    pub tag_name: String, // Identifier
}

#[derive(Debug, Clone)]
pub struct TagEntry {
    pub id: String,
    pub data: Vec<TagEntryData>,
}

pub enum ServerMainThreadBound {
    RequestRegistryInfo,
    RequestTagInfo,
    RequestChunk(Position),
    ChatMessage { player_name: String, message: String, timestamp: i64, salt: i64, },
}

pub enum ServerConnectionThreadBound {
    RegistryInfo { registry_id: String, entries: Vec<RegistryEntry> },
    RegistryInfoFinished,
    TagInfo(Vec<TagEntry>),
    ChunkData(Option<Chunk>),
    ChatMessage { player_name: String, message: String, timestamp: i64, salt: i64, },
}