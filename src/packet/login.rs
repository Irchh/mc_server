use uuid::Uuid;
use crate::error::ServerError;
use crate::packet::*;

#[derive(Debug)]
pub enum LoginPacketType {
    // Serverbound
    LoginStart {name: String, uuid: Uuid},
    LoginPluginResponse {message_id: String, success: bool, data: Option<Vec<u8>>},
    LoginAcknowledged,
}

#[repr(i32)]
pub enum LoginPacketResponse {
    Disconnect = 0x00,
    LoginSuccess = 0x02,
}

impl MCPacketType for LoginPacketResponse {
    fn id(self) -> i32 {
        self as i32
    }
}

impl LoginPacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            return Err(ServerError::WrongPacketSize{expected: iterator.len(), got: length});
        }
        let id = next_varint(&mut iterator)?;
        match id {
            0x00 => {
                Ok(Self::LoginStart {
                    name: next_string(&mut iterator)?,
                    uuid: Uuid::from_u128(next_u128(&mut iterator)?)
                })
            }
            0x02 => {
                Ok(Self::LoginPluginResponse {
                    message_id: next_string(&mut iterator)?,
                    success: next_bool(&mut iterator)?,
                    data: if iterator.len() > 0 {
                        Some(iterator.map(|r| *r).collect::<Vec<u8>>())
                    } else {
                        None
                    },
                })
            }
            0x03 => {
                Ok(Self::LoginAcknowledged)
            }
            _ => unimplemented!("Invalid login packet id: {}", id)
        }
    }
}