use crate::error::ServerError;
use crate::packet::*;
use crate::packet_builder::PacketBuilder;

#[derive(Debug)]
pub enum PlayPacketServerBound {
}

#[repr(i32)]
pub enum PlayPacketClientBound {
    AcknowledgeBlockChange = 0x05,
    Login = 0x2B,
    SyncPlayerPosition = 0x40,
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
            _ => unimplemented!("Invalid configuration packet id: {}", id)
        }
    }
}