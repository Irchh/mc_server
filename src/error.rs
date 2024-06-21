use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed parsing NBT: {0}")]
    NbtParseError(#[from] inbt::NbtParseError),
    #[error("Failed parsing world: {0}")]
    McaParseError(#[from] mc_world_parser::McaParseError),
    #[error("{0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("Parsed VarInt too big")]
    VarIntTooBig,
    #[error("Integer too big to be converted to a VarInt {0}")]
    IntTooBig(i32),
    #[error("Size specified in packet is wrong. Got {got}, expected {expected}")]
    WrongPacketSize{expected: usize, got: usize},
    #[error("Invalid server state: {0}")]
    InvalidServerState(i32),
    #[error("Reached end of packet data")]
    EndOfPacket,
}