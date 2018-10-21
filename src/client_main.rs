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

#[macro_use]
extern crate clap;
#[macro_use]
extern crate prettytable;
extern crate regex;

mod server;
mod utils;
mod rpc;

use std::process;
use std::result;
use std::str;

use tarpc::sync;
use tarpc::sync::client::ClientExt;

use server::*;
use rpc::{SyncClient};

type RpcResult = ::std::result::Result<(), tarpc::Error<Error>>;

struct TimeslotSpecifier {
    actuator_id: u32,
    timeslot_id: u32,
}

impl str::FromStr for TimeslotSpecifier {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let ids: Vec<&str> = s.split(':').collect();
        if ids.len() != 2 {
            return Err(());
        }

        let vals: Vec<u32> = ids.iter().filter_map(|s| u32::from_str(s).ok()).collect();
        if vals.len() != 2 {
            return Err(());
        }

        Ok(TimeslotSpecifier {
            actuator_id: vals[0],
            timeslot_id: vals[1],
        })
    }
}

fn get_client() -> SyncClient {
    match SyncClient::connect("localhost:4242", sync::client::Options::default()) {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Failed to connect: {}", err);
            process::exit(1)
        }
    }
}

// TODO: remove, replace with shell script
fn test() -> RpcResult {
    let client = get_client();

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
                day: 6,
            },
            end: Date {
                year: 2017,
                month: 11,
                day: 6,
            },
            // end: Date::MAX,
        },
        days: WeekdaySet::TUESDAY | WeekdaySet::SATURDAY,
    };

    let _time_slot_id = client.add_time_slot(actuator_id, time_period.clone(),
                                             ActuatorState::Toggle(true), true)?;

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

    client.add_time_slot(actuator_id, time_period, ActuatorState::Toggle(true), true)?;

    // let schedule = client.get_schedule(actuator_id).unwrap();

    // println!("{:?}", schedule.timeslots.len());

    // for ts in schedule.timeslots.values() {
        // println!("{:?}", ts);
    // }

    Ok(())
}

fn list_actuators() -> RpcResult {
    let actuators = get_client().list_actuators()?;

    println!("{:>5}  {:10} {:5}", "Index", "Name", "Type");
    for (id, actuator) in actuators.iter() {
        println!("{:5}  {:10} {:5}", id, actuator.name, actuator.actuator_type);
    }

    Ok(())
}

fn set_default_state(args: &clap::ArgMatches) -> RpcResult {
    let actuator_id = value_t_or_exit!(args, "actuator", u32);
    let actuator_state = value_t_or_exit!(args, "state", ActuatorState);

    get_client().set_default_state(actuator_id, actuator_state).and(Ok(()))
}

fn show_schedule(args: &clap::ArgMatches) -> RpcResult {
    use prettytable::{Table,format};

    let actuator_id = value_t_or_exit!(args, "actuator", u32);

    let schedule = get_client().get_schedule(actuator_id)?;

    println!("Default state: {}", schedule.default_state);

    if schedule.timeslots.is_empty() {
        println!("No timeslot configured");
        return Ok(())
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);
    table.set_titles(row![b => "Timeslot ID", "Enabled", "Actuator state", "Time range",
                          "Start date", "End date", "Days"]);

    for (slot_id, slot) in schedule.timeslots.iter() {
        let time_period = &slot.time_period;
        let enabled = if slot.enabled { "Yes" } else { "No" };
        let time_range = format!("{} - {}", time_period.time_interval.start,
                                 time_period.time_interval.end);

        // TODO: override
        table.add_row(row![slot_id, enabled, slot.actuator_state, time_range,
                           time_period.date_range.start, time_period.date_range.end,
                           time_period.days]);
    }

    table.printstd();

    Ok(())
}

fn add_time_slot(args: &clap::ArgMatches) -> RpcResult {
    let actuator_id = value_t_or_exit!(args, "actuator", u32);
    let time_interval = value_t_or_exit!(args, "time-interval", TimeInterval);
    let actuator_state = value_t_or_exit!(args, "state", ActuatorState);
    let start_date = if args.is_present("start-date") {
        value_t_or_exit!(args, "start-date", Date)
    } else {
        Date::MIN
    };
    let end_date = if args.is_present("end-date") {
        value_t_or_exit!(args, "end-date", Date)
    } else {
        Date::MAX
    };
    let weekdays = if args.is_present("weekdays") {
        value_t_or_exit!(args, "weekdays", WeekdaySet)
    } else {
        WeekdaySet::all()
    };

    let time_period = TimePeriod {
        time_interval: time_interval,
        date_range: DateRange {
            start: start_date,
            end: end_date,
        },
        days: weekdays,
    };

    get_client().add_time_slot(actuator_id, time_period, actuator_state, true).and(Ok(()))
}

fn remove_time_slot(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);

    get_client().remove_time_slot(specifier.actuator_id, specifier.timeslot_id).and(Ok(()))
}

fn time_slot_set_time_period(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);
    let time_interval = if args.is_present("time-interval") {
        value_t_or_exit!(args, "time-interval", TimeInterval)
    } else {
        TimeInterval { start: Time::EMPTY, end: Time::EMPTY }
    };
    let start_date = if args.is_present("start-date") {
        value_t_or_exit!(args, "start-date", Date)
    } else {
        Date::EMPTY
    };
    let end_date = if args.is_present("end-date") {
        value_t_or_exit!(args, "end-date", Date)
    } else {
        Date::EMPTY
    };
    let weekdays = if args.is_present("weekdays") {
        value_t_or_exit!(args, "weekdays", WeekdaySet)
    } else {
        WeekdaySet::empty()
    };

    let time_period = TimePeriod {
        time_interval: time_interval,
        date_range: DateRange {
            start: start_date,
            end: end_date,
        },
        days: weekdays,
    };

    get_client().time_slot_set_time_period(specifier.actuator_id, specifier.timeslot_id,
                                           time_period).and(Ok(()))
}

fn time_slot_set_enabled(args: &clap::ArgMatches, enabled: bool) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);

    get_client().time_slot_set_enabled(specifier.actuator_id, specifier.timeslot_id,
                                       enabled).and(Ok(()))
}

fn time_slot_set_actuator_state(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);
    let actuator_state = value_t_or_exit!(args, "state", ActuatorState);

    get_client().time_slot_set_actuator_state(specifier.actuator_id, specifier.timeslot_id,
                                              actuator_state).and(Ok(()))
}

fn time_slot(args: &clap::ArgMatches) -> RpcResult {
    match args.subcommand() {
        ("add", Some(sub)) => add_time_slot(sub),
        ("remove", Some(sub)) => remove_time_slot(sub),
        ("set-time", Some(sub)) => time_slot_set_time_period(sub),
        ("set-state", Some(sub)) => time_slot_set_actuator_state(sub),
        ("disable", Some(sub)) => time_slot_set_enabled(sub, false),
        ("enable", Some(sub)) => time_slot_set_enabled(sub, true),
        _ => unreachable!(),
    }
}

fn main() {
    use clap::{Arg, ArgGroup, App, AppSettings, SubCommand};

    let actuator_arg = Arg::with_name("actuator")
        .required(true)
        .help("Actuator ID");
    let timeslot_specifier_arg = Arg::with_name("specifier")
        .required(true)
        .help("Timeslot specifier, specified as <actuator ID>:<timeslot ID>");

    let args = App::new("servoctl")
        .about("CLI for ServoScheduler")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("list-actuators")
        ).subcommand(SubCommand::with_name("show-schedule")
            .arg(&actuator_arg)
        ).subcommand(SubCommand::with_name("set-default-state")
            .arg(&actuator_arg)
            .arg(Arg::with_name("state")
                .required(true)
                .help("Default actuator state")
            )
        ).subcommand(SubCommand::with_name("timeslot")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("add")
                .arg(&actuator_arg)
                .arg(Arg::with_name("time-interval")
                    .required(true)
                    .help("Time interval, specified as hh:mm-hh:mm")
                ).arg(Arg::with_name("state")
                    .required(true)
                    .help("Actuator state")
                ).arg(Arg::with_name("start-date")
                    .long("--start-date").short("-s")
                    .takes_value(true)
                    .help("Start date, specified as [YYYY-]MM-DD (default: now)")
                ).arg(Arg::with_name("end-date")
                    .long("--end-date").short("-e")
                    .takes_value(true)
                    .help("End date, specified as [YYYY-]MM-DD (default: none)")
                ).arg(Arg::with_name("weekdays")
                    .long("--weekdays").short("-w")
                    .takes_value(true).allow_hyphen_values(true)
                    .help("Enable only on certain weekdays, e.g. M----S- for Monday and Saturday (default: all)")
                )
            ).subcommand(SubCommand::with_name("remove")
                .arg(&timeslot_specifier_arg)
            ).subcommand(SubCommand::with_name("set-time")
                .arg(&timeslot_specifier_arg)
                .arg(Arg::with_name("time-interval")
                    .long("--time-interval").short("-t")
                    .help("Time interval, specified as hh:mm-hh:mm")
                ).arg(Arg::with_name("start-date")
                    .long("--start-date").short("-s")
                    .takes_value(true)
                    .help("Start date, specified as [YYYY-]MM-DD (default: now)")
                ).arg(Arg::with_name("end-date")
                    .long("--end-date").short("-e")
                    .takes_value(true)
                    .help("End date, specified as [YYYY-]MM-DD (default: none)")
                ).arg(Arg::with_name("weekdays")
                    .long("--weekdays").short("-w")
                    .takes_value(true).allow_hyphen_values(true)
                    .help("Enable only on certain weekdays, e.g. M----S- for Monday and Saturday (default: all)")
                ).group(ArgGroup::with_name("fields")
                    .args(&["time-interval", "start-date", "end-date","weekdays"])
                    .multiple(true)
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("set-state")
                .arg(&timeslot_specifier_arg)
                .arg(Arg::with_name("state")
                    .required(true)
                    .help("Actuator state")
                )
            ).subcommand(SubCommand::with_name("disable")
                .arg(&timeslot_specifier_arg)
            ).subcommand(SubCommand::with_name("enable")
                .arg(&timeslot_specifier_arg)
            )
        ).subcommand(SubCommand::with_name("test")
        ).get_matches();

    let res = match args.subcommand() {
        ("list-actuators", Some(_)) => list_actuators(),
        ("show-schedule", Some(sub)) => show_schedule(sub),
        ("set-default-state", Some(sub)) => set_default_state(sub),
        ("timeslot", Some(sub)) => time_slot(sub),
        ("test", Some(_)) => test(),
        _ => unreachable!(),
    };

    if let Err(error) = res {
        eprintln!("RPC failed: {}", error);
    }
}
