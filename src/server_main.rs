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

extern crate regex;

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

    {
        let mut server = rpc_server.server.write().unwrap();
        server.add_actuator(Actuator {
            name: "switch".to_string(),
            actuator_type: ActuatorType::Toggle
        }, ActuatorState::Toggle(false)).unwrap();
        server.add_actuator(Actuator {
            name: "knob".to_string(),
            actuator_type: ActuatorType::FloatValue { min: 0.0, max: 1.0 }
        }, ActuatorState::FloatValue(0.5)).unwrap();
        println!("Server added actuators");
    }

    let handle = rpc_server.listen("localhost:4242", sync::server::Options::default())
        .unwrap();
    handle.run();
}
