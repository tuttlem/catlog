mod db;

use crate::db::{Engine, EntryType};
use crate::db::entry::DecodeError;
use crate::db::LogEntry;
use crate::db::Segment;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::open("data").unwrap();

    for i in 1..1_000_000 {
        let key = format!("{}", i);
        let value = (i as u64).to_le_bytes().to_vec();

        engine.put(key, value)?;
    }

    Ok(())
}