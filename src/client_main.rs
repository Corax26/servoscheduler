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

mod server;
mod utils;
mod rpc;

use tarpc::sync;
use tarpc::sync::client::ClientExt;

use server::*;
use rpc::{SyncClient};

fn main() {
    let client = SyncClient::connect("localhost:4242", sync::client::Options::default())
        .unwrap();

    // TODO: get actuator list
    let actuator_id = 0;

    let mut time_period = TimePeriod {
        time_interval: TimeInterval {
            start: Time {
                hour: 23,
                minute: 5,
            },
            end: Time {
                hour: 3,
                minute: 5,
            },
        },
        date_range: DateRange {
            start: Date {
                year: 2017,
                month: 11,
                day: 8,
            },
            // end: Date {
                // year: 2017,
                // month: 5,
                // day: 8,
            // },
            end: Date::MAX,
        },
        days: WeekdaySet::all(),
    };

    let _time_slot_id = client.add_time_slot(actuator_id, time_period.clone(),
                                             ActuatorState::Toggle(true), true).unwrap();

    time_period.time_interval = TimeInterval {
        start: Time {
            hour: 18,
            minute: 5,
        },
        end: Time {
            hour: 23,
            minute: 4,
        },
    };

    client.add_time_slot(actuator_id, time_period, ActuatorState::Toggle(true), true).unwrap();

    let schedule = client.get_schedule(actuator_id).unwrap();

    println!("{:?}", schedule.timeslots.len());

    for ts in schedule.timeslots.values() {
        println!("{:?}", ts);
    }
}
