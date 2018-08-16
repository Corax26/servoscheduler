use std::sync::mpsc;
use std::thread;

use tarpc::sync;
use tarpc::sync::client::ClientExt;

use server::*;
use rpc::{RpcServer, SyncClient, SyncServiceExt};

pub fn test_client() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let rpc_server = RpcServer::new();

        let actuator = Actuator {
            name: "act".to_string(),
            actuator_type: ActuatorType::Toggle
        };
        rpc_server.server.write().unwrap().add_actuator(actuator, ActuatorState::Toggle(false)).unwrap();
        println!("Server added actuator");

        let handle = rpc_server.listen("localhost:0", sync::server::Options::default())
            .unwrap();
        tx.send(handle.addr()).unwrap();
        handle.run();
    });

    let client = SyncClient::connect(rx.recv().unwrap(), sync::client::Options::default())
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
                month: 5,
                day: 8,
            },
            end: Date {
                year: 2017,
                month: 5,
                day: 8,
            },
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
