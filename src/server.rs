use std::io::ErrorKind;
use std::net::TcpListener;
use std::sync::mpsc::TryRecvError;
use std::thread;
use log::*;
use mc_world_parser::World;
use crate::resource_manager::ResourceManager;
use crate::server_connection::MCServerConnection;
use crate::server_util::{DescriptionInfo, PlayerInfo, PlayerSample, ServerConnectionThreadBound, ServerInfo, ServerMainThreadBound, VersionInfo};

pub struct MCServer {
    server_info: ServerInfo,
    resource_manager: ResourceManager,
    world: World
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
            resource_manager: ResourceManager::new("resources").unwrap(),
            world: World::load("world").unwrap(),
        }
    }

    pub fn run(mut self, listen: &str) {
        let listener = TcpListener::bind(listen).unwrap();
        listener.set_nonblocking(true).unwrap();
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
                    let block_reg = self.resource_manager.block_registry_ref().clone();
                    threads.push(thread::spawn(|| {
                        MCServerConnection::new(connection, ch_from_thread.0, ch_to_thread.1, server_info, block_reg).run()
                    }));
                    channels.push((ch_to_thread.0, ch_from_thread.1));
                }
                Err(err) => {
                    if err.kind() != ErrorKind::WouldBlock {
                        error!("Error accepting connection: {}", err)
                    }
                },
            }
            // Drop any threads and channels that are finished
            for i in (0..threads.len()).rev() {
                if threads[i].is_finished() {
                    let thread = threads.remove(i);
                    let _channel = channels.remove(i);
                    if let Err(err) = thread.join() {
                        error!("Thread panicked: {}", err.downcast::<std::io::Error>().map(|e| e.to_string()).unwrap_or("Unknown reason".to_string()));
                    }
                }
            }
            // Handle all channel messages both ways
            // TODO: Add a message queue
            for i in 0..channels.len() {
                let (send, rec) = &channels[i];
                match rec.try_recv() {
                    Ok(request) => {
                        match request {
                            ServerMainThreadBound::RequestRegistryInfo => {
                                for (id, entries) in self.resource_manager.registries_ref() {
                                    let _ = send.send(ServerConnectionThreadBound::RegistryInfo {
                                        registry_id: id.clone(),
                                        entries: entries.clone(),
                                    });
                                }
                                let _ = send.send(ServerConnectionThreadBound::RegistryInfoFinished);
                            }
                            ServerMainThreadBound::RequestTagInfo => {
                                let _ = send.send(ServerConnectionThreadBound::TagInfo(self.resource_manager.tags_ref().clone()));
                            }
                            ServerMainThreadBound::RequestChunk(pos) => {
                                let _ = send.send(ServerConnectionThreadBound::ChunkData(self.world.get_chunk(pos)));
                            }
                            ServerMainThreadBound::ChatMessage { player_name, message, timestamp, salt } => {
                                for (channel_send, _) in &channels {
                                    let _= channel_send.send(ServerConnectionThreadBound::ChatMessage {player_name: player_name.clone(), message: message.clone(), timestamp, salt});
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }
}