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
mod actuator_controller;
mod rpc;
mod rpc_server;
mod schedule;
mod server;
mod time;
mod time_slot;
mod utils;

use std::path::Path;

use tarpc::sync;

use actuator::*;
use actuator_controller::*;
use rpc::SyncServiceExt;
use rpc_server::RpcServer;
use server::Server;

fn main() {
    let mut server = Server::new();

    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => {
            server.add_actuator(Actuator::new(
                ActuatorInfo {
                    name: "switch".to_string(),
                    actuator_type: ActuatorType::Toggle
                },
                ActuatorState::Toggle(false),
                FileActuatorController::new(Path::new("fake_ctl_files/switch")).unwrap(),
            )).unwrap();
            server.add_actuator(Actuator::new(
                ActuatorInfo {
                    name: "knob".to_string(),
                    actuator_type: ActuatorType::FloatValue { min: 0.0, max: 1.0 }
                },
                ActuatorState::FloatValue(0.5),
                FileActuatorController::new(Path::new("fake_ctl_files/knob")).unwrap(),
            )).unwrap();
        },
        2 => {
            server.add_actuator(Actuator::new(
                ActuatorInfo {
                    name: "switch".to_string(),
                    actuator_type: ActuatorType::Toggle
                },
                ActuatorState::Toggle(false),
                FileActuatorController::new(Path::new(&args[1])).unwrap(),
            )).unwrap();
        },
        _ => std::process::exit(1),
    }

    let rpc_server = RpcServer::new(server);

    let handle = rpc_server.listen("localhost:4242", sync::server::Options::default())
        .unwrap();
    handle.run();
}
