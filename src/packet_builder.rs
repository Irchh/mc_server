use uuid::Uuid;
use crate::datatypes::{MCString, VarInt};
use crate::packet::MCPacketType;

pub struct PacketBuilder {
    packet_id: Option<i32>,
    proto_packet: Vec<u8>,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            packet_id: None,
            proto_packet: vec![],
        }
    }

    pub fn set_id<P: MCPacketType>(mut self, packet_id: P) -> Self {
        self.packet_id = Some(packet_id.id());
        self
    }

    pub fn add_string<S: Into<String>>(mut self, string: S) -> Self {
        self.proto_packet.append(&mut MCString::new(string.into()).unwrap().bytes);
        self
    }

    pub fn add_uuid(mut self, uuid: Uuid) -> Self {
        // TODO: Byte endianness
        self.proto_packet.append(&mut uuid.as_u128().to_be_bytes().to_vec());
        self
    }

    pub fn add_varint(mut self, int: i32) -> Self {
        self.proto_packet.append(&mut VarInt::new(int).bytes);
        self
    }

    pub fn add_bool(mut self, value: bool) -> Self {
        self.proto_packet.push(value as u8);
        self
    }

    pub fn build(mut self) -> Option<Vec<u8>> {
        let mut packet = vec![];
        packet.append(&mut VarInt::new(self.packet_id?).bytes);
        packet.append(&mut self.proto_packet);
        packet.reverse();
        let mut packet_length = VarInt::new(packet.len() as i32).bytes;
        packet_length.reverse();
        packet.append(&mut packet_length);
        packet.reverse();
        Some(packet)
    }
}