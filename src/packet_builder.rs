use crate::datatypes::{MCString, VarInt};
use crate::packet::MCPacketType;
use crate::server_connection::ConnectionStatusType;

pub struct PacketBuilder {
    packet_type: Option<ConnectionStatusType>,
    packet_id: Option<i32>,
    proto_packet: Vec<u8>,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            packet_type: None,
            packet_id: None,
            proto_packet: vec![],
        }
    }

    pub fn set_state(mut self, packet_type: ConnectionStatusType) -> Self {
        self.packet_type = Some(packet_type);
        self
    }

    pub fn set_id<P: MCPacketType>(mut self, packet_id: P) -> Self {
        self.packet_id = Some(packet_id.id());
        self
    }

    pub fn add_string<S: Into<String>>(mut self, string: S) -> Self {
        self.proto_packet.append(&mut MCString::new(string.into()).unwrap().bytes);
        self
    }

    pub fn build(mut self) -> Option<Vec<u8>> {
        let mut packet = vec![];
        packet.append(&mut VarInt::new(self.packet_id?).ok()?.bytes);
        packet.append(&mut self.proto_packet);
        packet.reverse();
        let mut packet_length = VarInt::new(packet.len() as i32).ok()?.bytes;
        packet_length.reverse();
        packet.append(&mut packet_length);
        packet.reverse();
        Some(packet)
    }
}