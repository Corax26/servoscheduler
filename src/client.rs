use server::*;

pub fn test_client() {
    let mut server = Server::new();

    let actuator = Actuator {
        name: "act".to_string(),
        actuator_type: ActuatorType::Toggle
    };

    let actuator_id = server.add_actuator(actuator, ActuatorState::Toggle(false)).unwrap();

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

    let _time_slot_id = server.add_time_slot(actuator_id, time_period.clone(),
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

    server.add_time_slot(actuator_id, time_period, ActuatorState::Toggle(true), true).unwrap();

    let schedule = server.get_schedule(actuator_id).unwrap();

    for ts in schedule.timeslots.values() {
        println!("{:?}", ts);
    }
}
