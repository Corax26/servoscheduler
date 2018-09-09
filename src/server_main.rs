#![feature(plugin, use_extern_macros, proc_macro_path_invoc)]
#![plugin(tarpc_plugins)]

#[macro_use]
extern crate tarpc;

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate tokio_core;

#[macro_use]
extern crate bitflags;
extern crate chrono;
extern crate num;

// #[macro_use]
// extern crate log;
// extern crate env_logger;

mod server;
mod utils;
mod rpc;

use tarpc::sync;

use server::*;
use rpc::{RpcServer, SyncServiceExt};

fn main() {
    let rpc_server = RpcServer::new();

    let actuator = Actuator {
        name: "act".to_string(),
        actuator_type: ActuatorType::Toggle
    };
    rpc_server.server.write().unwrap().add_actuator(actuator, ActuatorState::Toggle(false)).unwrap();
    println!("Server added actuator");

    let handle = rpc_server.listen("localhost:4242", sync::server::Options::default())
        .unwrap();
    handle.run();
}
