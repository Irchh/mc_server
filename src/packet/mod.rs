mod handshake;
mod status;
mod login;
mod configure;
mod play;

use std::slice::Iter;
use crate::error::ServerError;

pub use status::StatusPacketType;
pub use handshake::HandshakePacketType;
pub use login::{LoginPacketType, LoginPacketResponse};
pub use configure::{ConfigurationPacketType, ConfigurationPacketResponse};
pub use play::{PlayPacketServerBound, PlayPacketClientBound};

pub trait MCPacketType {
    fn id(self) -> i32;
}

// Packet structure:
//      length: VarInt
//      packet_id: VarInt
//      data: ByteArray


fn next_varint(data: &mut Iter<u8>) -> Result<i32, ServerError> {
    let mut value = 0;
    let mut shift = 0;
    loop {
        let byte = *data.next().ok_or(ServerError::EndOfPacket)?;
        value |= (byte as i32&0x7F)<<shift;
        if byte&0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            panic!("VarInt too big");
        }
    }
    Ok(value)
}

fn next_u8(data: &mut Iter<u8>) -> Result<u8, ServerError> {
    Ok(*data.next().ok_or(ServerError::EndOfPacket)?)
}

fn next_u16(data: &mut Iter<u8>) -> Result<u16, ServerError> {
    Ok(u16::from_be_bytes([*data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?]))
}

fn next_f32(data: &mut Iter<u8>) -> Result<f32, ServerError> {
    Ok(f32::from_be_bytes([*data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?]))
}

fn next_f64(data: &mut Iter<u8>) -> Result<f64, ServerError> {
    Ok(f64::from_be_bytes([
        *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?,
        *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?
    ]))
}

fn next_string(data: &mut Iter<u8>) -> Result<String, ServerError> {
    let length = next_varint(data)?;
    let utf8 = data.take(length as usize).map(|n| *n).collect::<Vec<u8>>();
    Ok(String::from_utf8(utf8)?)
}

fn next_bool(data: &mut Iter<u8>) -> Result<bool, ServerError> {
    Ok(*data.next().ok_or(ServerError::EndOfPacket)? != 0)
}

fn next_u128(data: &mut Iter<u8>) -> Result<u128, ServerError> {
    Ok(u128::from_be_bytes(
        [
            *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?,
            *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?,
            *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?,
            *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?,
        ]
    ))
}