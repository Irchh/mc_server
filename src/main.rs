mod packet;
mod packet_builder;
mod datatypes;
mod error;
mod server_util;
mod server;
mod server_connection;

use std::env;
use crate::server::MCServer;


fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "debug")
    }
    env_logger::init();

    let server = MCServer::new();
    server.run("0.0.0.0:25565");
}
