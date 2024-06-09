use serde::Serialize;

#[derive(Serialize)]
pub struct VersionInfo {
    pub name: String,
    pub protocol: i32
}
#[derive(Serialize)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}
#[derive(Serialize)]
pub struct PlayerInfo {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<PlayerSample>
}
#[derive(Serialize)]
pub struct DescriptionInfo {
    pub text: String
}
#[derive(Serialize)]
pub struct ServerInfo {
    pub description: DescriptionInfo,
    pub players: PlayerInfo,
    pub version: VersionInfo,
    pub favicon: String
}