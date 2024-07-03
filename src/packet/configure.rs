use std::collections::BTreeMap;
use log::{debug, error};
use crate::block_registry::BlockRegistry;
use crate::error::ServerError;
use crate::packet::*;
use crate::packet_builder::PacketBuilder;
use crate::resource_manager::ResourceManager;
use crate::server_util::{RegistryEntry, TagEntry};

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
    UpdateTags = 0x0D,
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

    pub fn update_tags(tags: Vec<TagEntry>, resource_manager: &ResourceManager) -> Vec<u8> {
        let mut packet = PacketBuilder::new()
            .set_id(Self::UpdateTags)
            .add_varint(tags.len() as i32);

        for tag in tags {
            packet = packet.add_string(tag.id.clone())
                .add_varint(tag.data.len() as i32);
            for tag_array in tag.data {
                packet = packet.add_string(tag_array.tag_name)
                    .add_varint(tag_array.entries.len() as i32);
                for entry in tag_array.entries {
                    if tag.id.eq("minecraft:block") {
                        if let Some(entry_id) = resource_manager.block_registry_ref().get_default_blockstate_of_block(entry.clone()) {
                            packet = packet.add_varint(entry_id);
                        } else {
                            error!("Error getting id of {} from {}", entry, tag.id);
                        }
                    } else {
                        if let Some(registry) = resource_manager.registries_ref().get(&tag.id) {
                            for (id, reg_entry) in registry.iter().enumerate() {
                                if reg_entry.id.eq(&entry) {
                                    packet = packet.add_varint(id as i32);
                                    break;
                                }
                            }
                        } else {
                            error!("NO REGISTRY CALLED {}. Error getting id of {} from {}", tag.id, entry, tag.id);
                            //debug!("registries: {:?}", registries);
                            panic!();
                        }
                    }
                }
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