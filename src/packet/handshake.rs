use crate::error::ServerError;
use crate::packet::*;

#[derive(Debug)]
pub enum HandshakePacketType {
    // Serverbound
    Handshake {protocol: i32, server_addr: String, server_port: u16, next_state: i32},
}

impl MCPacketType for HandshakePacketType {
    fn id(self) -> i32 {
        match self {
            HandshakePacketType::Handshake { .. } => 0x00,
        }
    }
}

impl HandshakePacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            return Err(ServerError::WrongPacketSize{expected: iterator.len(), got: length});
        }
        let id = next_varint(&mut iterator)?;
        match id {
            0x00 => {
                Ok(Self::Handshake {
                    protocol: next_varint(&mut iterator)?,
                    server_addr: next_string(&mut iterator)?,
                    server_port: next_u16(&mut iterator)?,
                    next_state: next_varint(&mut iterator)?,
                })
            }
            _ => unimplemented!("Invalid handshake packet id: {}", id)
        }
    }
}