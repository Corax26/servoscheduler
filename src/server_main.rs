#![feature(plugin, use_extern_macros, proc_macro_path_invoc)]
#![plugin(tarpc_plugins)]

#[macro_use]
extern crate tarpc;

#[macro_use]
extern crate serde_derive;

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
mod rpc;
mod rpc_server;
mod server;
mod time;
mod time_slot;
mod utils;

use tarpc::sync;

use actuator::*;
use rpc::SyncServiceExt;
use rpc_server::RpcServer;
use server::Server;

fn main() {
    let mut server = Server::new();

    server.add_actuator(Actuator::new(
        ActuatorInfo {
            name: "switch".to_string(),
            actuator_type: ActuatorType::Toggle
        },
        ActuatorState::Toggle(false)
    )).unwrap();
    server.add_actuator(Actuator::new(
        ActuatorInfo {
            name: "knob".to_string(),
            actuator_type: ActuatorType::FloatValue { min: 0.0, max: 1.0 }
        },
        ActuatorState::FloatValue(0.5)
    )).unwrap();
    println!("Server added actuators");

    let rpc_server = RpcServer::new(server);

    let handle = rpc_server.listen("localhost:4242", sync::server::Options::default())
        .unwrap();
    handle.run();
}
