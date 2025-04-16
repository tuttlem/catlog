mod db;

use crate::db::entry::EntryType;
use crate::db::LogEntry;


fn main() {
    let entry = LogEntry {
        entry_type: EntryType::Put,
        timestamp: 1680000000000,
        key: "name".to_string(),
        value: b"Bob".to_vec(),
    };

    let bytes = entry.encode();

    println!("{:x?}", bytes); // Debug-print hex view

    let entry_2 = LogEntry::decode(&bytes).unwrap();

    println!("{:?}", entry_2);
    println!("{}", entry_2.value_as_string().unwrap());
}