#![feature(plugin, use_extern_macros, proc_macro_path_invoc)]
#![plugin(tarpc_plugins)]

#[macro_use]
extern crate tarpc;

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

// Only for FutureService
// extern crate futures;
// extern crate tokio_core;

#[macro_use]
extern crate bitflags;
extern crate chrono;
extern crate num;

extern crate regex;

// #[macro_use]
// extern crate log;
// extern crate env_logger;

mod actuator;
mod actuator_controller;
mod rpc;
mod rpc_server;
mod schedule;
mod server;
mod time;
mod time_slot;
mod utils;

use std::fs::File;
use std::path::Path;
use std::result;

use tarpc::sync;

use rpc::SyncServiceExt;
use rpc_server::RpcServer;
use server::Server;

fn main() -> result::Result<(), String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        return Err(format!("Usage: {} config_file.yaml", args[0]))
    }

    let config_file = File::open(Path::new(&args[1]))
        .map_err(|e| format!("Failed to open config file: {}", e))?;
    let server = Server::new(config_file)
        .map_err(|e| format!("Failed to create server: {}", e))?;

    let rpc_server = RpcServer::new(server);

    let handle = rpc_server.listen("localhost:4242", sync::server::Options::default())
        .unwrap();
    handle.run();
    Ok(())
}
