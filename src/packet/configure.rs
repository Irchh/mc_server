use crate::error::ServerError;
use crate::packet::*;

#[derive(Debug)]
pub enum ConfigurationPacketType {
    ClientInformation {
        locale: String,
        view_distance: i8,
        chat_mode: i32,
        chat_has_colors: bool,
        displayed_skin_parts: u8, // bit mask
        main_hand: i32,
        enable_text_filtering: bool,
        allow_server_listings: bool,
    },
    ServerBoundPluginMessage { channel: String, data: Vec<u8> },
    FinishConfigurationAck,
}

#[repr(i32)]
pub enum ConfigurationPacketResponse {
    ClientBoundPluginMessage = 0x01,
    Disconnect = 0x02,
    FinishConfiguration = 0x03,
}

impl MCPacketType for ConfigurationPacketResponse {
    fn id(self) -> i32 {
        self as i32
    }
}

impl ConfigurationPacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            return Err(ServerError::WrongPacketSize{expected: iterator.len(), got: length});
        }
        let id = next_varint(&mut iterator)?;
        match id {
            0x00 => {
                Ok(Self::ClientInformation {
                    locale: next_string(&mut iterator)?,
                    view_distance: next_u8(&mut iterator)? as i8,
                    chat_mode: next_varint(&mut iterator)?,
                    chat_has_colors: next_bool(&mut iterator)?,
                    displayed_skin_parts: next_u8(&mut iterator)?,
                    main_hand: next_varint(&mut iterator)?,
                    enable_text_filtering: next_bool(&mut iterator)?,
                    allow_server_listings: next_bool(&mut iterator)?,
                })
            }
            0x02 => {
                Ok(Self::ServerBoundPluginMessage {
                    channel: next_string(&mut iterator)?,
                    data: iterator.map(|r| *r).collect::<Vec<u8>>(),
                })
            }
            0x03 => {
                Ok(Self::FinishConfigurationAck)
            }
            _ => unimplemented!("Invalid configuration packet id: {}", id)
        }
    }
}