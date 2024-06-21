use log::trace;
use crate::error::ServerError;

#[derive(Debug)]
pub struct VarInt {
    pub value: i32,
    pub bytes: Vec<u8>
}

impl VarInt {
    pub fn new(value: i32) -> Self {
        let mut num = value as u64;
        let mut data = vec![];
        loop {
            if num > 0x80u64 {
                let b = num as u8 & (!0x80);
                data.push(b|0x80);
                num = num >> 7;
            } else {
                data.push(num as u8);
                break;
            }
        }
        Self {
            value,
            bytes: data,
        }
    }

    pub fn from(data: Vec<u8>) -> Result<Self, ServerError> {
        let mut iterator = data.iter();
        let mut bytes = vec![];
        let mut value = 0;
        let mut shift = 0;
        loop {
            let byte = *iterator.next().ok_or(ServerError::EndOfPacket)?;
            bytes.push(byte);
            value |= (byte as i32&0x7F)<<shift;
            if byte&0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 32 {
                return Err(ServerError::VarIntTooBig);
            }
        }
        Ok(Self {
            value,
            bytes,
        })
    }
}

#[derive(Debug)]
pub struct VarLong {
    pub value: i32,
    pub bytes: Vec<u8>
}

#[derive(Debug)]
pub struct MCString {
    pub value: String,
    pub bytes: Vec<u8>
}

impl MCString {
    pub fn new(value: String) -> Result<Self, ServerError> {
        let mut string_bytes = value.as_bytes().to_vec();
        let mut length_bytes = VarInt::new(string_bytes.len() as i32).bytes;
        let mut bytes = vec![];
        bytes.append(&mut length_bytes);
        bytes.append(&mut string_bytes);
        Ok(Self {
            value,
            bytes,
        })
    }

    pub fn from(data: Vec<u8>) -> Result<Self, ServerError> {
        let length = VarInt::from(data.clone())?;
        trace!("MCString length: {}", length.value);
        trace!("MCString byte count: {}", length.bytes.len());
        let utf8 = data[length.bytes.len()..].iter().take(length.value as usize).map(|n| *n).collect::<Vec<u8>>();
        Ok(Self {
            value: String::from_utf8(utf8)?,
            bytes: vec![],
        })
    }
}
