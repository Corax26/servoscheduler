// #![feature(plugin, use_extern_macros)]
// #![plugin(tarpc_plugins)]

// #[macro_use]
// extern crate tarpc;

// #[macro_use]
// extern crate serde_derive;

// #[macro_use]
// extern crate log;
// extern crate env_logger;

#[macro_use]
extern crate bitflags;
extern crate chrono;
extern crate num;

mod client;
mod server;
mod utils;
// mod rpc;

fn main() {
    client::test_client();
}
