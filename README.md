# catlog: Simple Log-Structured Key-Value Store

This project is a **persistent, append-only key-value database** built from scratch in Rust. It uses a 
**log-structured storage engine** with segment file rotation and a simple **line-based TCP protocol** for client 
interaction.

## Features

- ✅ Log-structured, append-only write design
- ✅ Persistent segment files on disk
- ✅ Automatic segment file rollover
- ✅ Support for `PUT`, `GET`, and `DELETE` operations
- ✅ Multithreaded writer + reader safety via `RwLock`
- ✅ Simple text-based TCP protocol
- ✅ Fast `to_le_bytes()` encoding for numeric values
- ✅ Minimal dependencies, blazing fast

---

## How It Works

- All writes are appended to a segment file on disk.
- Each segment stores `LogEntry` structs that are CRC-verified and encoded.
- When a segment reaches a max size, it rotates to a new file.
- Reads scan all segments chronologically and replay the latest `PUT` or `DELETE` for the requested key.
- A simple TCP server accepts plain text commands over a socket.

---

## Running the Server

### Prerequisites

- Rust (2021 edition)
- `cargo` build system

### Build & Run

```bash
cargo build --release
cargo run
```

By default, the server will listen on:

```
127.0.0.1:4000
```

You will see logs like:

```
[LISTENING] 127.0.0.1:4000
[CONN] 127.0.0.1:50312
[CONN CLOSED]
```

---

## Protocol

### Basic Format

```
PUT <key> <value>
GET <key>
DELETE <key>
```

### Response Format

```
OK
VALUE <value>
NOT_FOUND
ERROR <message>
```

---

## Example Usage with netcat

```bash
echo "PUT name Bob" | nc 127.0.0.1 4000
# OK

echo "GET name" | nc 127.0.0.1 4000
# VALUE Bob

echo "DELETE name" | nc 127.0.0.1 4000
# OK

echo "GET name" | nc 127.0.0.1 4000
# NOT_FOUND
```

---

## Internals

- Log entry format:

  ```
  [u32: CRC32]
  [u8: entry_type]      // PUT = 1, DELETE = 2
  [u64: timestamp_ms]
  [u32: key_len]
  [u32: value_len]
  [key bytes]
  [value bytes]
  ```

- Segment files are named:

  ```
  data/segment-00001.log
  data/segment-00002.log
  ...
  ```

- Active segment is always the most recent file

---

## Thread Safety

The engine is protected by `Arc<RwLock<Engine>>` so that:
- Multiple readers can call `GET` concurrently
- Writers (`PUT` / `DELETE`) have exclusive access

---

## Future Improvements

- [ ] Snapshotting / checkpointing for fast startup
- [ ] In-memory key index to avoid scanning all segments
- [ ] Log compaction
- [ ] Expiry / TTL support
- [ ] Batching / pipelining
- [ ] More advanced protocol (e.g. RESP or JSON)

---

## Author Notes

This is a learning project that touches on real-world database engine internals — from segment files and CRCs to 
concurrency and wire protocols. It’s lean, fast, and open to grow.
