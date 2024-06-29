use log::debug;
use mc_world_parser::chunk::Chunk;
use uuid::Uuid;
use crate::error::ServerError;
use crate::packet::*;
use crate::packet_builder::PacketBuilder;

#[derive(Debug)]
pub struct Slot {
    pub(crate) count: i32,
    pub(crate) item_id: Option<i32>,
    pub(crate) components_to_add: Option<Vec<(i32, Vec<u8>)>>,
    pub(crate) number_to_remove: Option<Vec<i32>>,
}

#[derive(Debug)]
pub enum PlayPacketServerBound {
    ConfirmTeleportation { id: i32 },
    ChatMessage { message: String, timestamp: i64, salt: i64, signature: Option<Vec<u8>>, message_count: i32, acknowledged: Vec<u8> },
    CloseContainer(u8),
    ClientInformation { locale: String, view_distance: i8, chat_mode: i32, chat_has_colors: bool, /** This is a bit mask */ displayed_skin_parts: u8, main_hand: i32, enable_text_filtering: bool, allow_server_listings: bool,  },
    DebugSampleSubscription { sample_type: i32 },
    SetPlayerPosition { x: f64, y: f64, z: f64, on_ground: bool },
    SetPlayerPositionAndRotation { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool },
    SetPlayerRotation{ yaw: f32, pitch: f32, on_ground: bool },
    SetPlayerOnGround(bool),
    PingRequest(u64),
    PlayerAbilities(u8),
    PlayerAction { status: i32, packed_location: u64, face: u8, sequence: i32 },
    PlayerCommand { eid: i32, id: i32, jump_boost: i32 },
    SetHeldItem(u16),
    SetCreativeModeSlot { slot: u16, clicked_item: Slot },
    SwingArm{ off_hand: bool },
    UseItemOn { off_hand: bool, packed_location: u64, face: i32, cursor_x: f32, cursor_y: f32, cursor_z: f32, inside_block: bool, sequence: i32 },
    UseItem { off_hand: bool, sequence: i32, yaw: f32, pitch: f32 },
}

#[repr(i32)]
pub enum PlayPacketClientBound {
    AcknowledgeBlockChange = 0x05,
    ChangeDifficulty = 0x0B,
    DisguisedChatMessage = 0x1E,
    EntityEvent = 0x1F,
    ChunkDataAndUpdateLight = 0x27,
    PingResponse = 0x36,
    PlayerAbilities = 0x38,
    PlayerChatMessage = 0x39,
    Login = 0x2B,
    SyncPlayerPosition = 0x40,
    SetHeldItem = 0x53,
    SetTickingState = 0x71,
    StepTick = 0x72,
    EntityEffect = 0x76,
}

impl PlayPacketClientBound {
    pub fn login(eid: i32, hardcore: bool, dimension_names: Vec<String>, max_players: i32, view_dist: i32, ) -> Vec<u8> {
        let mut packet = PacketBuilder::new()
            .set_id(Self::Login)
            .add_int(eid)
            .add_bool(hardcore)
            .add_varint(dimension_names.len() as i32);

        for dim in dimension_names {
            packet = packet.add_string(dim);
        }

        packet = packet
            .add_varint(max_players)
            .add_varint(view_dist)
            .add_varint(view_dist) // Simulation dist
            .add_bool(false) // Reduced debug view
            .add_bool(false) // Enable respawn screen
            .add_bool(false) // Do limited crafting
            .add_varint(0) // Dimension Type ID
            .add_string("minecraft:overworld") // Dimension identifier
            .add_long(-6574177734957711742i64 as u64) // Hashed seed (used for biome noise)
            .add_byte(1) // Gamemode creative
            .add_byte(0xFF) // Previous gamemode (-1/0xFF is undefined)
            .add_bool(false) // Debug world
            .add_bool(false) // Flat world
            .add_bool(false) // Has death location
            .add_varint(0) // Portal cooldown
            .add_bool(false) // Enforces secure chat
        ;

        return packet.build().unwrap();
    }

    pub fn change_difficulty(difficulty: u8) -> Vec<u8> {
        PacketBuilder::new()
            .set_id(Self::ChangeDifficulty)
            .add_byte(difficulty)
            .add_bool(false)
            .build().unwrap()
    }

    pub fn player_abilities() -> Vec<u8> {
        PacketBuilder::new()
            .set_id(Self::PlayerAbilities)
            .add_byte(0x0D)
            .add_float(0.05)
            .add_float(0.1)
            .build().unwrap()
    }

    pub fn player_chat_message_fake(player_name: String, msg: String) -> Vec<u8> {
        let packet = PacketBuilder::new()
            .set_id(Self::DisguisedChatMessage)

            .add_byte(0x8) // String NBT Tag
            .add_short(msg.clone().into_bytes().len() as i16)
            .add_bytes(msg.into_bytes())

            .add_varint(1) // Chat type index into registry (i REALLY need to implement registries lol)

            .add_byte(0x8) // String NBT Tag
            .add_short(player_name.clone().into_bytes().len() as i16)
            .add_bytes(player_name.into_bytes())

            .add_bool(false)
            ;

        packet.build().unwrap()
    }

    pub fn player_chat_message(player_name: String, msg: String, timestamp: u64, salt: u64) -> Vec<u8> {
        let packet = PacketBuilder::new()
            .set_id(Self::PlayerChatMessage)
            .add_uuid(Uuid::from_u128(0)) // Player UUID. Zero for now
            .add_varint(0) // Index (?)
            .add_bool(false) // Signature present

            .add_string(msg) // Message
            .add_long(timestamp)
            .add_long(salt)

            .add_varint(0) // Previous msg count, max 20.

            .add_bool(false) // Unsigned content present
            .add_varint(0) // Filter type PASS_THROUGH

            .add_varint(1) // Chat type index into registry (i REALLY need to implement registries lol)

            .add_byte(0x8) // String NBT Tag
            .add_short(player_name.clone().into_bytes().len() as i16)
            .add_bytes(player_name.into_bytes())

            //.add_string(format!("{{\"text\": \"{player_name}\"}}")) // Sender name
            .add_bool(false) // Has target
            ;

        packet.build().unwrap()
    }

    /// See https://wiki.vg/Entity_statuses for event codes
    pub fn entity_event(eid: i32, event: u8) -> Vec<u8> {
        PacketBuilder::new()
            .set_id(Self::EntityEvent)
            .add_int(eid)
            .add_byte(event)
            .build().unwrap()
    }

    pub fn entity_effect(eid: i32, effect: i32, amplifier: i32, duration: i32, flags: u8) -> Vec<u8> {
        PacketBuilder::new()
            .set_id(Self::EntityEffect)
            .add_varint(eid)
            .add_varint(effect)
            .add_varint(amplifier)
            .add_varint(duration)
            .add_byte(flags)
            .build().unwrap()
    }

    pub fn set_held_item(slot: u8) -> Vec<u8> {
        PacketBuilder::new()
            .set_id(Self::SetHeldItem)
            .add_byte(slot)
            .build().unwrap()
    }

    pub fn chunk_data(chunk: Chunk) -> Vec<u8> {
        let data = chunk.network_data(|id| {
            let num = match id.as_str() {
                "minecraft:air" => 0,
                "minecraft:bedrock" => 79,
                //_ => 155,
                "minecraft:stone" => 1,
                "minecraft:granite" => 2,
                "minecraft:polished_granite" => 3,
                "minecraft:diorite" => 4,
                "minecraft:polished_diorite" => 5,
                "minecraft:andesite" => 6,
                "minecraft:polished_andesite" => 7,
                "minecraft:grass_block_but_snowy" => 8, // idk lol
                "minecraft:grass_block" => 9,
                "minecraft:dirt" => 10,
                "minecraft:coarse_dirt" => 11,
                "minecraft:podzol_but_snowy" => 12,
                "minecraft:podzol" => 13,
                "minecraft:cobblestone" => 14,
                "minecraft:oak_planks" => 15,
                "minecraft:spruce_planks" => 16,
                "minecraft:birch_planks" => 17,
                "minecraft:jungle_planks" => 18,
                "minecraft:acacia_planks" => 19,
                "minecraft:cherry_planks" => 20,
                "minecraft:dark_oak_planks" => 21,
                "minecraft:mangrove_planks" => 22,
                "minecraft:bamboo_planks" => 23,
                "minecraft:bamboo_mosaic" => 24,
                "minecraft:oak_sapling" => 25,
                "minecraft:oak_sapling_stage_1" => 26,
                "minecraft:spruce_sapling" => 27,
                "minecraft:spruce_sapling_stage_1" => 28,
                "minecraft:birch_sapling" => 29,
                "minecraft:birch_sapling_stage_1" => 30,
                "minecraft:jungle_sapling" => 31,
                "minecraft:jungle_sapling_stage_1" => 32,
                "minecraft:acacia_sapling" => 33,
                "minecraft:acacia_sapling_stage_1" => 34,
                "minecraft:cherry_sapling" => 35,
                "minecraft:cherry_sapling_stage_1" => 36,
                "minecraft:dark_oak_sapling" => 37,
                "minecraft:dark_oak_sapling_stage_1" => 38,
                // lots of propagule stuff
                "minecraft:bedrock" => 79,
                "minecraft:water" => 80,
                //"minecraft:water_level_7" => 81,
                //"minecraft:water_level_6" => 82,
                //"minecraft:water_level_5" => 83,
                //"minecraft:water_level_4" => 84,
                //"minecraft:water_level_3" => 85,
                //"minecraft:water_level_2" => 86,
                //"minecraft:water_level_1" => 87,
                // 88 - 95 weird flowing/falling water stuff
                // 96 - 110 weird lava that looks like water
                "minecraft:sand" => 112,
                "minecraft:suspicious_sand" => 113,
                // 114-118 different dusted levels of sus sand
                "minecraft:suspicious_gravel" => 119,
                // 120-122 different dusted levels of sus gravel
                "minecraft:gold_ore" => 123,
                "minecraft:deepslate_gold_ore" => 124,
                "minecraft:iron_ore" => 125,
                "minecraft:deepslate_iron_ore" => 126,
                "minecraft:coal_ore" => 127,
                "minecraft:deepslate_coal_ore" => 128,
                "minecraft:nether_gold_ore" => 129,
                //"minecraft:oak_log" => 130, // This is on the X axis
                "minecraft:oak_log" => 131, // This is on the Y axis
                //"minecraft:oak_log" => 132, // This is on the Z axis
                //"minecraft:spruce_log" => 133, // This is on the X axis
                "minecraft:spruce_log" => 134, // This is on the Y axis
                //"minecraft:spruce_log" => 135, // This is on the Z axis
                //"minecraft:birch_log" => 136, // This is on the X axis
                "minecraft:birch_log" => 137, // This is on the Y axis
                //"minecraft:birch_log" => 138, // This is on the Z axis

                //"minecraft:jungle_log" => 139, // This is on the X axis
                "minecraft:jungle_log" => 140, // This is on the Y axis
                //"minecraft:jungle_log" => 141, // This is on the Z axis

                //"minecraft:acacia_log" => 142, // This is on the X axis
                "minecraft:acacia_log" => 143, // This is on the Y axis
                //"minecraft:acacia_log" => 144, // This is on the Z axis

                //"minecraft:cherry_log" => 145, // This is on the X axis
                "minecraft:cherry_log" => 146, // This is on the Y axis
                //"minecraft:cherry_log" => 147, // This is on the Z axis

                //"minecraft:dark_oak_log" => 148, // This is on the X axis
                "minecraft:dark_oak_log" => 149, // This is on the Y axis
                //"minecraft:dark_oak_log" => 150, // This is on the Z axis

                //"minecraft:mangrove_log" => 151, // This is on the X axis
                "minecraft:mangrove_log" => 152, // This is on the Y axis
                //"minecraft:mangrove_log" => 153, // This is on the Z axis

                // 154 is waterlogged mangrove roots, BIG LAG
                _ => 0,
            };
            num
        });

        let packet = PacketBuilder::new()
            .set_id(Self::ChunkDataAndUpdateLight)
            .add_int(chunk.chunk_pos().x)
            .add_int(chunk.chunk_pos().z)
            .add_byte(0x0a) // Compound NBT Tag ID
            .add_byte(0x00) // NBT END Tag, since root tags have no name when transferred over the network.
            .add_varint(data.len() as i32)
            .add_bytes(data)
            .add_varint(0) // block entities

            .add_varint(0) // Bit sets
            .add_varint(0) // Bit sets
            .add_varint(0) // Bit sets
            .add_varint(0) // Bit sets

            .add_varint(0) // Skylight array size
            .add_varint(0) // Blocklight array size
            ;

        packet.build().unwrap()
    }
}

impl MCPacketType for PlayPacketClientBound {
    fn id(self) -> i32 {
        self as i32
    }
}

impl PlayPacketServerBound {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = bytes.iter();
        let length = next_varint(&mut iterator)? as usize;
        if iterator.len() != length {
            return Err(ServerError::WrongPacketSize{expected: iterator.len(), got: length});
        }
        let id = next_varint(&mut iterator)?;
        match id {
            0x00 => {
                Ok(Self::ConfirmTeleportation {
                    id: next_varint(&mut iterator)?,
                })
            }
            0x0A => {
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
            0x06 => {
                Ok(Self::ChatMessage {
                    message: next_string(&mut iterator)?,
                    timestamp: next_u64(&mut iterator)? as i64,
                    salt: next_u64(&mut iterator)? as i64,
                    signature: if next_bool(&mut iterator)? {
                        Some((&mut iterator).take(256).map(|n| *n).collect())
                    } else {
                        None
                    },
                    message_count: next_varint(&mut iterator)?,
                    acknowledged: iterator.take(3).map(|n| *n).collect(),
                })
            }
            0x0F => {
                Ok(Self::CloseContainer(next_u8(&mut iterator)?))
            }
            0x13 => {
                Ok(Self::DebugSampleSubscription {
                    sample_type: next_varint(&mut iterator)?,
                })
            }
            0x1A => {
                Ok(Self::SetPlayerPosition {
                    x: next_f64(&mut iterator)?,
                    y: next_f64(&mut iterator)?,
                    z: next_f64(&mut iterator)?,
                    on_ground: next_bool(&mut iterator)?,
                })
            }
            0x1B => {
                Ok(Self::SetPlayerPositionAndRotation {
                    x: next_f64(&mut iterator)?,
                    y: next_f64(&mut iterator)?,
                    z: next_f64(&mut iterator)?,
                    yaw: next_f32(&mut iterator)?,
                    pitch: next_f32(&mut iterator)?,
                    on_ground: next_bool(&mut iterator)?,
                })
            }
            0x1C => {
                Ok(Self::SetPlayerRotation {
                    yaw: 0.0,
                    pitch: 0.0,
                    on_ground: false,
                })
            }
            0x1D => {
                Ok(Self::SetPlayerOnGround(next_bool(&mut iterator)?))
            }
            0x21 => {
                Ok(Self::PingRequest(next_u64(&mut iterator)?))
            }
            0x23 => {
                Ok(Self::PlayerAbilities(next_u8(&mut iterator)?))
            }
            0x24 => {
                Ok(Self::PlayerAction {
                    status: next_varint(&mut iterator)?,
                    packed_location: next_u64(&mut iterator)?,
                    face: next_u8(&mut iterator)?,
                    sequence: next_varint(&mut iterator)?,
                })
            }
            0x25 => {
                Ok(Self::PlayerCommand {
                    eid: next_varint(&mut iterator)?,
                    id: next_varint(&mut iterator)?,
                    jump_boost: next_varint(&mut iterator)?,
                })
            }
            0x2F => {
                Ok(Self::SetHeldItem(next_u16(&mut iterator)?))
            }
            0x32 => {
                let slot = next_u16(&mut iterator)?;
                let count = next_varint(&mut iterator)?;
                if count > 0 {
                    Ok(Self::SetCreativeModeSlot {
                        slot,
                        clicked_item: Slot {
                            count,
                            item_id: Some(next_varint(&mut iterator)?),
                            components_to_add: None, // TODO
                            number_to_remove: None, // TODO
                        },
                    })
                } else {
                    Ok(Self::SetCreativeModeSlot {
                        slot,
                        clicked_item: Slot { count, item_id: None, components_to_add: None, number_to_remove: None, },
                    })
                }
            }
            0x36 => {
                Ok(Self::SwingArm { off_hand: next_bool(&mut iterator)? })
            }
            0x38 => {
                Ok(Self::UseItemOn {
                    off_hand: next_bool(&mut iterator)?,
                    packed_location: next_u64(&mut iterator)?,
                    face: next_varint(&mut iterator)?,
                    cursor_x: next_f32(&mut iterator)?,
                    cursor_y: next_f32(&mut iterator)?,
                    cursor_z: next_f32(&mut iterator)?,
                    inside_block: next_bool(&mut iterator)?,
                    sequence: next_varint(&mut iterator)?,
                })
            }
            0x39 => {
                Ok(Self::UseItem {
                    off_hand: next_bool(&mut iterator)?,
                    sequence: next_varint(&mut iterator)?,
                    yaw: next_f32(&mut iterator)?,
                    pitch: next_f32(&mut iterator)?,
                })
            }
            _ => unimplemented!("Invalid play packet id: {:02X}", id)
        }
    }
}