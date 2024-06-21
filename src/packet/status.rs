use crate::error::ServerError;
use crate::packet::*;

pub enum StatusPacketType {
    // Serverbound
    Status,
    // Clientbound
    StatusResponse,
    // Both ways
    Ping {raw: Vec<u8>},
}

impl MCPacketType for StatusPacketType {
    fn id(self) -> i32 {
        match self {
            StatusPacketType::Status => 0x00,
            StatusPacketType::StatusResponse { .. } => 0x00,
            StatusPacketType::Ping { .. } => 0x01,
        }
    }
}

impl StatusPacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            return Err(ServerError::WrongPacketSize{expected: iterator.len(), got: length});
        }
        let id = next_varint(&mut iterator)?;
        match id {
            0x00 => {
                Ok(Self::Status)
            }
            0x01 => {
                Ok(Self::Ping { raw: bytes })
            }
            _ => unimplemented!("Invalid status packet id: {}", id)
        }
    }
}