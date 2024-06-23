use log::error;
use crate::error::ServerError;
use crate::packet::*;
use crate::packet_builder::PacketBuilder;
use crate::server_util::RegistryEntry;

#[derive(Debug)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

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
    ServerBoundKnownPacks { known_packs: Vec<KnownPack> },
}

#[repr(i32)]
pub enum ConfigurationPacketResponse {
    ClientBoundPluginMessage = 0x01,
    Disconnect = 0x02,
    FinishConfiguration = 0x03,
    RegistryData = 0x07,
    ClientBoundKnownPacks = 0x0E,
}

impl MCPacketType for ConfigurationPacketResponse {
    fn id(self) -> i32 {
        self as i32
    }
}

impl ConfigurationPacketResponse {
    pub fn registry_data(registry_id: String, entries: Vec<RegistryEntry>) -> Vec<u8> {
        let mut packet = PacketBuilder::new()
            .set_id(Self::RegistryData)
            .add_string(registry_id)
            .add_varint(entries.len() as i32);

        for entry in entries {
            packet = packet.add_string(entry.id)
                //.add_bool(entry.data.is_some());
                .add_bool(false);
            if entry.data.is_some() {
                //packet = packet.add_nbt(entry.data.unwrap();
            }
        }

        packet.build().unwrap()
    }
}

impl ConfigurationPacketType {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            error!("Real size: {}, Reported size: {}", iterator.len(), length);
            return Err(ServerError::WrongPacketSize {expected: length, got: iterator.len()});
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
            0x07 => {
                let known_pack_count = next_varint(&mut iterator)?;
                let mut known_packs = vec![];
                for _ in 0..known_pack_count {
                    known_packs.push(KnownPack {
                            namespace: next_string(&mut iterator)?,
                            id: next_string(&mut iterator)?,
                            version: next_string(&mut iterator)?,
                        }
                    );
                }
                Ok(Self::ServerBoundKnownPacks { known_packs })
            }
            _ => unimplemented!("Invalid configuration packet id: {}", id)
        }
    }
}