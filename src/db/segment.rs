use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use crate::db::entry::DecodeError;
use crate::db::LogEntry;

pub struct Segment {
    pub file: File,
    pub path: PathBuf,
    pub write_offset: u64,
}


impl Segment {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let write_offset = file.seek(SeekFrom::End(0))?;

        Ok(Segment {
            file,
            path: path.as_ref().to_path_buf(),
            write_offset,
        })
    }

    pub fn append_entry(&mut self, entry: &LogEntry) -> std::io::Result<u64> {
        let encoded = entry.encode();
        let offset = self.write_offset;

        self.file.write_all(&encoded)?;
        self.write_offset += encoded.len() as u64;

        Ok(offset)
    }

    pub fn iter_entries(&self) -> std::io::Result<SegmentIterator> {
        let file = File::open(&self.path)?;
        Ok(SegmentIterator {
            reader: BufReader::new(file),
            offset: 0
        })
    }
}


pub struct SegmentIterator {
    reader: BufReader<File>,
    offset: u64,
}

impl Iterator for SegmentIterator {
    type Item = Result<LogEntry, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        // seek to the current offset
        if self.reader.seek(SeekFrom::Start(self.offset)).is_err() {
            return None;
        }

        let mut header = [0u8; 21]; // 4 + 1 + 8 + 4 + 4
        if let Err(_) = self.reader.read_exact(&mut header) {
            return None; // Likely EOF
        }

        // Extract key and value lengths
        let key_len = u32::from_le_bytes(header[13..17].try_into().unwrap()) as usize;
        let value_len = u32::from_le_bytes(header[17..21].try_into().unwrap()) as usize;

        let total_len = 21 + key_len + value_len;

        // Allocate exact size buffer for full entry
        let mut full_buf = Vec::with_capacity(total_len);
        full_buf.extend_from_slice(&header);

        let mut remaining = vec![0u8; key_len + value_len];
        if let Err(e) = self.reader.read_exact(&mut remaining) {
            return Some(Err(DecodeError::from(e)));
        }
        full_buf.extend_from_slice(&remaining);

        match LogEntry::decode(&full_buf) {
            Ok(entry) => {
                self.offset += full_buf.len() as u64;
                Some(Ok(entry))
            }
            Err(e) => Some(Err(e)),
        }
    }
}