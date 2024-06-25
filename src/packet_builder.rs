use uuid::Uuid;
use mc_datatypes::{MCString, VarInt};
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

    pub fn add_byte(mut self, value: u8) -> Self {
        self.proto_packet.push(value);
        self
    }

    pub fn add_short(mut self, value: i16) -> Self {
        self.proto_packet.append(&mut value.to_be_bytes().to_vec());
        self
    }

    pub fn add_int(mut self, value: i32) -> Self {
        self.proto_packet.append(&mut value.to_be_bytes().to_vec());
        self
    }

    pub fn add_long(mut self, value: u64) -> Self {
        self.proto_packet.append(&mut value.to_be_bytes().to_vec());
        self
    }

    pub fn add_bytes(mut self, mut value: Vec<u8>) -> Self {
        self.proto_packet.append(&mut value);
        self
    }

    pub fn add_float(mut self, value: f32) -> Self {
        self.proto_packet.append(&mut value.to_be_bytes().to_vec());
        self
    }

    pub fn add_double(mut self, value: f64) -> Self {
        self.proto_packet.append(&mut value.to_be_bytes().to_vec());
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