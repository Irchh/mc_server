use std::slice::Iter;
use crate::error::ServerError;

pub trait MCPacketType {
    fn id(self) -> i32;
}

pub enum PacketType {
    Status,
    Login,
    Play,
}

// Packet structure:
//      length: VarInt
//      packet_id: VarInt
//      data: ByteArray
pub enum StatusPacketType {
    // Serverbound
    Handshake {protocol: i32, server_addr: String, server_port: u16, next_state: i32},
    Ping {raw: Vec<u8>},
    // Clientbound
    StatusResponse,
}

impl MCPacketType for StatusPacketType {
    fn id(self) -> i32 {
        match self {
            StatusPacketType::Handshake { .. } => 0x00, // Serverbound so won't really be needed
            StatusPacketType::Ping { .. } => 0x01,
            StatusPacketType::StatusResponse { .. } => 0x00,
        }
    }
}

impl StatusPacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = Self::next_varint(&mut iterator)?;
        let id = Self::next_varint(&mut iterator)?;
        match id {
            0x00 => {
                if length == 1 {
                    Ok(Self::Handshake {
                        protocol: 0,
                        server_addr: "".to_string(),
                        server_port: 0,
                        next_state: 0,
                    })
                } else {
                    Ok(Self::Handshake {
                        protocol: Self::next_varint(&mut iterator)?,
                        server_addr: Self::next_string(&mut iterator)?,
                        server_port: Self::next_u16(&mut iterator)?,
                        next_state: Self::next_varint(&mut iterator)?,
                    })
                }
            }
            0x01 => {
                Ok(Self::Ping { raw: bytes })
            }
            _ => unimplemented!("Not yet implemented: {}", id)
        }
    }

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

    fn next_u16(data: &mut Iter<u8>) -> Result<u16, ServerError> {
        Ok(u16::from_be_bytes([*data.next().ok_or(ServerError::EndOfPacket)?, *data.next().ok_or(ServerError::EndOfPacket)?]))
    }

    fn next_string(data: &mut Iter<u8>) -> Result<String, ServerError> {
        let length = Self::next_varint(data)?;
        let utf8 = data.take(length as usize).map(|n| *n).collect::<Vec<u8>>();
        Ok(String::from_utf8(utf8)?)
    }
}