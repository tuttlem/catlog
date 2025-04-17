use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::db::{EntryType, LogEntry, Segment};
use crate::db::entry::DecodeError;

const MAX_SEGMENT_SIZE: u64 = 1024 * 1024 * 10; // 10MB segment size

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX_EPOCH")
        .as_millis() as u64
}

pub struct Engine {
    segments: Vec<Segment>,     // ordered oldest to newest
    active: Segment,            // current segment we're writing to
    dir: PathBuf,               // the data folder
    next_segment_id: u64,       // the next segment id
}

impl Engine {
    fn rotate_segment(&mut self) -> std::io::Result<()> {
        let old_active = std::mem::replace(
            &mut self.active,
            Segment::open(self.dir.join(format!("segment-{:05}.log", self.next_segment_id)))?,
        );

        self.segments.push(old_active);
        self.next_segment_id += 1;

        Ok(())
    }

    pub fn open<P: AsRef<Path>>(dir: P) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        let mut segment_ids: Vec<u64> = std::fs::read_dir(&dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    e.path()
                        .file_stem()
                        .and_then(|f| f.to_str())
                        .and_then(|f| f.strip_prefix("segment-"))
                        .and_then(|id| id.parse::<u64>().ok())
                })
            })
            .collect();

        segment_ids.sort_unstable();

        let mut segments = Vec::new();

        for id in &segment_ids {
            let filename = format!("segment-{:05}.log", id);
            let path = dir.join(filename);
            let segment = Segment::open(path)?;

            segments.push(segment);
        }

        let next_id = segment_ids.last().copied().unwrap_or(0) + 1;
        let active = segments.pop().unwrap_or_else(|| {
            Segment::open(dir.join(format!("segment-00001.log"))).expect("Failed to create initial segment")
        });

        Ok(Self {
            segments,
            active,
            dir,
            next_segment_id: next_id,
        })
    }

    pub fn get<K: Into<String>>(&self, key: K) -> Result<Option<Vec<u8>>, DecodeError> {
        let key = key.into();
        let mut value: Option<Vec<u8>> = None;

        for segment in &self.segments {
            for entry_result in segment.iter_entries()? {
                let entry = entry_result?;
                if entry.key == key {
                    match entry.entry_type {
                        EntryType::Put => value = Some(entry.value.clone()),
                        EntryType::Delete => value = None,
                    }
                }
            }
        }

        // Check active segment last
        for entry_result in self.active.iter_entries()? {
            let entry = entry_result?;
            if entry.key == key {
                match entry.entry_type {
                    EntryType::Put => value = Some(entry.value.clone()),
                    EntryType::Delete => value = None,
                }
            }
        }

        Ok(value)
    }

    pub fn put<K: Into<String>>(&mut self, key: K, value: Vec<u8>) -> std::io::Result<()> {
        let entry = LogEntry {
            entry_type: EntryType::Put,
            timestamp: current_timestamp(),
            key: key.into(),
            value,
        };

        self.active.append_entry(&entry)?;

        if self.active.write_offset >= MAX_SEGMENT_SIZE {
            self.rotate_segment()?;
        }

        Ok(())
    }

    pub fn delete<K: Into<String>>(&mut self, key: K) -> std::io::Result<()> {
        let entry = LogEntry {
            entry_type: EntryType::Delete,
            timestamp: current_timestamp(),
            key: key.into(),
            value: vec![]
        };

        self.active.append_entry(&entry)?;

        if self.active.write_offset >= MAX_SEGMENT_SIZE {
            self.rotate_segment()?;
        }

        Ok(())
    }
}