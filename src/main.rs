mod db;

use crate::db::Engine;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::thread;

type SharedEngine = Arc<RwLock<Engine>>;

fn handle_client(mut stream: TcpStream, engine: SharedEngine) {
    let peer = stream.peer_addr().unwrap();
    println!("[CONN] {}", peer);

    let reader = BufReader::new(stream.try_clone().unwrap());

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let mut parts = line.splitn(3, ' ');
        let cmd = parts.next().unwrap_or("");
        let key = parts.next().unwrap_or("");
        let val = parts.next();

        let response = match cmd {
            "GET" => {
                let val = engine.read().unwrap().get(key);
                match val {
                    Ok(Some(bytes)) => format!("VALUE {}\n", String::from_utf8_lossy(&bytes)),
                    Ok(None) => "NOT_FOUND\n".to_string(),
                    Err(e) => format!("ERROR {}\n", e),
                }
            }

            "PUT" => {
                if let Some(v) = val {
                    let result = engine.write().unwrap().put(key, v.into());
                    match result {
                        Ok(_) => "OK\n".to_string(),
                        Err(e) => format!("ERROR {}\n", e),
                    }
                } else {
                    "ERROR Missing value\n".to_string()
                }
            }

            "DELETE" => {
                let result = engine.write().unwrap().delete(key);
                match result {
                    Ok(_) => "OK\n".to_string(),
                    Err(e) => format!("ERROR {}\n", e),
                }
            }

            _ => "ERROR Unknown command\n".to_string(),
        };

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }

    println!("[CONN CLOSED] {}", peer);
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine: SharedEngine = Arc::new(
        RwLock::new(
            Engine::open("data")?
        )
    );

    let listener = TcpListener::bind("127.0.0.1:4000")?;
    println!("[LISTENING] 127.0.0.1:4000");

    for stream in listener.incoming() {
        let engine = Arc::clone(&engine);
        if let Ok(stream) = stream {
            thread::spawn(move || {
                handle_client(stream, engine);
            });
        }
    }

    Ok(())
}