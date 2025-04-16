use std::{fmt, io};
use crc32fast::Hasher;

#[derive(Debug)]
pub enum DecodeError {
    Io(io::Error),
    InvalidFormat(String),
    UnexpectedEOF,
    BadChecksum,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::Io(e) => write!(f, "I/O error: {}", e),
            DecodeError::InvalidFormat(e) => write!(f, "Invalid format: {}", e),
            DecodeError::UnexpectedEOF => write!(f, "Unexpected end of file"),
            DecodeError::BadChecksum => write!(f, "Bad checksum"),
        }
    }
}

impl std::error::Error for DecodeError {}
impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        DecodeError::Io(e)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EntryType {
    Put = 0x01,
    Delete = 0x02,
}

impl EntryType {
    pub fn from_u8(n: u8) -> Option<EntryType> {
        match n {
            0x01 => Some(EntryType::Put),
            0x02 => Some(EntryType::Delete),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct LogEntry {
    pub entry_type: EntryType,
    pub timestamp: u64,
    pub key: String,
    pub value: Vec<u8>,
}

impl LogEntry {

    /*
    Layout for an encoded entry

    [ u32: CRC32 of everything after this ]
    [ u8 : entry_type ]
    [ u64: timestamp ]
    [ u32: key_len ]
    [ u32: value_len ]
    [ key bytes ]
    [ value bytes ]
    */

    pub fn encode(&self) -> Vec<u8> {
        let key_bytes = self.key.as_bytes();
        let key_len = key_bytes.len() as u32;
        let value_len = self.value.len() as u32;

        const BASE_SIZE: usize =
            1 +     // entry type
            8 +     // timestamp length
            4 +     // key length
            4;      // value length

        let mut payload = Vec::with_capacity(
            BASE_SIZE + key_len as usize + value_len as usize
        );

        payload.push(self.entry_type as u8);
        payload.extend_from_slice(&self.timestamp.to_le_bytes()); // timestamp
        payload.extend_from_slice(&key_len.to_le_bytes());        // data length of key
        payload.extend_from_slice(&value_len.to_le_bytes());      // data length of value
        payload.extend_from_slice(key_bytes);                     // write the key
        payload.extend_from_slice(&self.value);                   // write the value

        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let crc = hasher.finalize();

        let mut buffer = Vec::with_capacity(4 + payload.len());
        buffer.extend_from_slice(&crc.to_le_bytes());             // CRC
        buffer.extend_from_slice(&payload);                       // payload data

        buffer
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        const BASE_SIZE: usize =
            4 +     // CRC
            1 +     // entry type
            8 +     // timestamp length
            4 +     // key length
            4;      // value length

        if bytes.len() < BASE_SIZE {
            return Err(DecodeError::UnexpectedEOF);
        }

        // pull the stored crc first
        let stored_crc = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        // pul the remaining payload
        let payload = &bytes[4..];

        // compute the hash
        let mut hasher = Hasher::new();
        hasher.update(payload);
        let actual_crc = hasher.finalize();

        if stored_crc != actual_crc {
            return Err(DecodeError::BadChecksum);
        }

        // handle the entry_type
        let entry_type_byte = payload[0];
        let entry_type = EntryType::from_u8(entry_type_byte)
            .ok_or_else(|| DecodeError::InvalidFormat(format!("Invalid entry type: {}", entry_type_byte)))?;

        let timestamp = u64::from_le_bytes(payload[1..9].try_into().unwrap());
        let key_len = u32::from_le_bytes(payload[9..13].try_into().unwrap()) as usize;
        let value_len = u32::from_le_bytes(payload[13..17].try_into().unwrap()) as usize;

        let total_expected_len = BASE_SIZE + key_len + value_len;

        if bytes.len() < total_expected_len {
            return Err(DecodeError::UnexpectedEOF);
        }

        let key_start = 17;
        let key_end = key_start + key_len;
        let value_start = key_end;
        let value_end = value_start + value_len;

        let key = String::from_utf8(payload[key_start..key_end].to_vec())
            .map_err(|e| DecodeError::InvalidFormat(e.to_string()))?;

        let value = Vec::from(&payload[value_start..value_end]).to_vec();

        Ok(Self {
            entry_type,
            timestamp,
            key,
            value,
        })
    }

    pub fn value_as_string(&self) -> Option<String> {
        String::from_utf8(self.value.clone()).ok()
    }
}