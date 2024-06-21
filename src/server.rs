use std::net::TcpListener;
use std::thread;
use log::*;
use crate::server_connection::MCServerConnection;
use crate::server_util::{DescriptionInfo, PlayerInfo, PlayerSample, ServerInfo, VersionInfo};

pub struct MCServer {
    server_info: ServerInfo,
}

impl MCServer {
    pub fn new() -> Self {
        Self {
            server_info: ServerInfo {
                description: DescriptionInfo { text: "RustMC 1.21-dev".to_string() },
                players: PlayerInfo {
                    max: 100,
                    online: 1,
                    sample: vec![PlayerSample { name: "Irchh".to_string(), id: "00000000-0000-0000-0000-000000000000".to_string() }],
                },
                version: VersionInfo { name: "RustMC 1.21".to_string(), protocol: 767 },
                favicon: "data:image/png;base64,<data>".to_string(),
            },
        }
    }

    pub fn run(self, listen: &str) {
        let listener = TcpListener::bind(listen).unwrap();
        info!("Listening on: {}", listen);
        let mut threads = vec![];
        let mut channels = vec![];
        for stream in listener.incoming() {
            match stream {
                Ok(connection) => {
                    info!("New connection from: {}", connection.peer_addr().map(|s| s.to_string()).unwrap_or("Unknown".to_string()));
                    let ch_to_thread = std::sync::mpsc::channel();
                    let ch_from_thread = std::sync::mpsc::channel();
                    let server_info = self.server_info.clone();
                    threads.push(thread::spawn(|| {
                        MCServerConnection::new(connection, ch_from_thread.0, ch_to_thread.1, server_info).run()
                    }));
                    channels.push((ch_to_thread.0, ch_from_thread.1));
                }
                Err(err) => error!("Error accepting connection: {}", err),
            }
        }
    }
}