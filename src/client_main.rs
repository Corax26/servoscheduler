#![feature(plugin, use_extern_macros, proc_macro_path_invoc)]
#![plugin(tarpc_plugins)]

#[macro_use]
extern crate tarpc;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate bitflags;
extern crate chrono;
extern crate num;

#[macro_use]
extern crate clap;
#[macro_use]
extern crate prettytable;
extern crate regex;

mod actuator;
mod rpc;
mod schedule;
mod time;
mod time_slot;
mod utils;

use std::process;
use std::result;
use std::str;
use std::str::FromStr;

use tarpc::sync;
use tarpc::sync::client::ClientExt;

use actuator::*;
use time_slot::*;
use time::*;
use rpc::{SyncClient};

type RpcResult = result::Result<(), tarpc::Error<rpc::Error>>;

fn parse_colon_specifier(s: &str, expected_num: usize) -> Option<Vec<u32>> {
    let ids: Vec<&str> = s.split(':').collect();
    if ids.len() != expected_num {
        return None
    }

    let vals: Vec<u32> = ids.iter().filter_map(|s| u32::from_str(s).ok()).collect();
    if vals.len() == expected_num {
        Some(vals)
    } else {
        None
    }
}

struct TimeslotSpecifier {
    actuator_id: u32,
    timeslot_id: u32,
}

impl str::FromStr for TimeslotSpecifier {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let vals = parse_colon_specifier(s, 2).ok_or(())?;

        Ok(TimeslotSpecifier {
            actuator_id: vals[0],
            timeslot_id: vals[1],
        })
    }
}

struct TimeslotOverrideSpecifier {
    actuator_id: u32,
    timeslot_id: u32,
    timeslot_override_id: u32,
}

impl str::FromStr for TimeslotOverrideSpecifier {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let vals = parse_colon_specifier(s, 3).ok_or(())?;

        Ok(TimeslotOverrideSpecifier {
            actuator_id: vals[0],
            timeslot_id: vals[1],
            timeslot_override_id: vals[2],
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
            start: Date::from_ymd(2017, 11, 6).unwrap(),
            end: Date::from_ymd(2017, 11, 6).unwrap(),
            // end: Date::MAX,
        },
        days: WeekdaySet::MONDAY | WeekdaySet::SATURDAY,
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

fn list_time_slots(args: &clap::ArgMatches) -> RpcResult {
    use prettytable::{Table, format};

    fn time_interval_str(time_period: &TimePeriod) -> String {
        format!("{} - {}", time_period.time_interval.start, time_period.time_interval.end)
    }

    let actuator_id = value_t_or_exit!(args, "actuator", u32);

    let timeslots = get_client().list_timeslots(actuator_id)?;

    if timeslots.is_empty() {
        println!("No timeslot configured");
        return Ok(())
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);
    table.set_titles(row![b => "Timeslot ID", "Enabled", "Actuator state", "Time range",
                          "Start date", "End date", "Days"]);

    for (slot_id, slot) in timeslots.iter() {
        let time_period = &slot.time_period;
        let enabled = if slot.enabled { "Yes" } else { "No" };
        let time_range = time_interval_str(time_period);

        table.add_row(row![slot_id, enabled, slot.actuator_state, time_range,
                           time_period.date_range.start, time_period.date_range.end,
                           time_period.days]);

        for (time_override_id, time_period) in slot.time_override.iter() {
            let id = format!("{} > {}", slot_id, time_override_id);
            let time_range = time_interval_str(time_period);

            table.add_row(row![id, "-", "-", time_range,
                               time_period.date_range.start, time_period.date_range.end,
                               time_period.days]);
        }
    }

    table.printstd();

    Ok(())
}

fn add_time_slot(args: &clap::ArgMatches) -> RpcResult {
    let actuator_id = value_t_or_exit!(args, "actuator", u32);
    let time_interval = value_t_or_exit!(args, "time-interval", TimeInterval);
    let actuator_state = value_t_or_exit!(args, "state", ActuatorState);
    // TODO: macro value_t_default_or_exit, or just set value using .default_value()
    let start_date = if args.is_present("start-date") {
        value_t_or_exit!(args, "start-date", Date)
    } else {
        // TODO: maybe actually use today, to make it more consistent with the doc? It might also
        // make it possible to get rid of Date::MIN.
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
        Date::empty_date()
    };
    let end_date = if args.is_present("end-date") {
        value_t_or_exit!(args, "end-date", Date)
    } else {
        Date::empty_date()
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

fn time_slot_set_actuator_state(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);
    let actuator_state = value_t_or_exit!(args, "state", ActuatorState);

    get_client().time_slot_set_actuator_state(specifier.actuator_id, specifier.timeslot_id,
                                              actuator_state).and(Ok(()))
}

fn time_slot_set_enabled(args: &clap::ArgMatches, enabled: bool) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);

    get_client().time_slot_set_enabled(specifier.actuator_id, specifier.timeslot_id,
                                       enabled).and(Ok(()))
}

fn time_slot_add_time_override(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotSpecifier);
    let time_interval = value_t_or_exit!(args, "time-interval", TimeInterval);
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

    get_client().time_slot_add_time_override(specifier.actuator_id, specifier.timeslot_id,
                                             time_period).and(Ok(()))
}

fn time_slot_remove_time_override(args: &clap::ArgMatches) -> RpcResult {
    let specifier = value_t_or_exit!(args, "specifier", TimeslotOverrideSpecifier);

    get_client().time_slot_remove_time_override(specifier.actuator_id, specifier.timeslot_id,
                                                specifier.timeslot_override_id).and(Ok(()))
}

fn time_slot(args: &clap::ArgMatches) -> RpcResult {
    match args.subcommand() {
        ("list", Some(sub)) => list_time_slots(sub),
        ("add", Some(sub)) => add_time_slot(sub),
        ("remove", Some(sub)) => remove_time_slot(sub),
        ("set-time", Some(sub)) => time_slot_set_time_period(sub),
        ("set-state", Some(sub)) => time_slot_set_actuator_state(sub),
        ("disable", Some(sub)) => time_slot_set_enabled(sub, false),
        ("enable", Some(sub)) => time_slot_set_enabled(sub, true),
        ("add-override", Some(sub)) => time_slot_add_time_override(sub),
        ("remove-override", Some(sub)) => time_slot_remove_time_override(sub),
        _ => unreachable!(),
    }
}

fn default_state(args: &clap::ArgMatches) -> RpcResult {
    let sub = match args.subcommand() {
        ("get", Some(sub)) => sub,
        ("set", Some(sub)) => sub,
        _ => unreachable!(),
    };

    let actuator_id = value_t_or_exit!(sub, "actuator", u32);

    if sub.is_present("state") {
        let actuator_state = value_t_or_exit!(sub, "state", ActuatorState);
        get_client().set_default_state(actuator_id, actuator_state).and(Ok(()))
    } else {
        println!("{}", get_client().get_default_state(actuator_id)?);
        Ok(())
    }
}

fn schedule(args: &clap::ArgMatches) -> RpcResult {
    use prettytable::{Table, Row, format};

    let actuator_id = value_t_or_exit!(args, "actuator", u32);
    let start_date = if args.is_present("start-date") {
        value_t_or_exit!(args, "start-date", Date)
    } else {
        DateTime::now().date
    };
    let nb_days = value_t_or_exit!(args, "day-number", u32);

    let timeslots = get_client().list_timeslots(actuator_id)?;
    let default_state = get_client().get_default_state(actuator_id)?;

    let schedule = schedule::compute_schedule(&timeslots, start_date, nb_days);

    let mut schedule_table = Table::new();
    schedule_table.set_titles(Row::new(schedule.keys().map(|d| cell!(b->d)).collect()));
    let mut days_row = Row::empty();

    for slots in schedule.values() {
        let mut day_table = Table::new();
        day_table.set_format(*format::consts::FORMAT_CLEAN);

        let mut previous_end_time = Time { hour: Time::DAY_START_HOUR, minute: 0 };

        for slot in slots.iter() {
            let id_string = if let Some(oid) = slot.override_id {
                format!("{} > {}", slot.id, oid)
            } else {
                format!("{}", slot.id)
            };

            if slot.time_interval.start != previous_end_time {
                day_table.add_row(row!["", default_state]);
                day_table.add_row(row![slot.time_interval.start, ""]);
            }

            day_table.add_row(row!["  |  ", format!("{} (TS {})", slot.actuator_state, id_string)]);
            day_table.add_row(row![slot.time_interval.end, ""]);

            previous_end_time = slot.time_interval.end;
        }

        day_table.add_row(row!["", default_state]);

        days_row.add_cell(cell!(day_table));
    }

    schedule_table.add_row(days_row);
    schedule_table.printstd();

    Ok(())
}

fn main() {
    use clap::{Arg, ArgGroup, App, AppSettings, SubCommand};

    let actuator_arg = Arg::with_name("actuator")
        .help("Actuator ID");
    let actuator_state_arg = Arg::with_name("state")
        .help("Default actuator state");

    let timeslot_specifier_arg = Arg::with_name("specifier")
        .help("Timeslot specifier, specified as <actuator ID>:<timeslot ID>");
    let timeslot_override_specifier_arg = Arg::with_name("specifier")
        .help("Timeslot override specifier, specified as <actuator ID>:<timeslot ID>:<override ID>");

    let time_interval_arg = Arg::with_name("time-interval")
        .takes_value(true)
        .help("Time interval, specified as hh:mm-hh:mm");
    let start_date_arg = Arg::with_name("start-date")
        .takes_value(true)
        .help("Start date, specified as DD/MM[/YYYY] (default: today)");
    let end_date_arg = Arg::with_name("end-date")
        .takes_value(true)
        .help("End date, specified as DD/MM[/YYYY] (default: none)");
    let weekdays_arg = Arg::with_name("weekdays")
        .takes_value(true).allow_hyphen_values(true)
        .help("Enable only on certain weekdays, e.g. M----S- for Monday and Saturday (default: all)");

    let args = App::new("servoctl")
        .about("CLI for ServoScheduler")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("list-actuators")
        ).subcommand(SubCommand::with_name("default-state")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("get")
                .arg(actuator_arg.clone()
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("set")
                .arg(actuator_arg.clone()
                    .required(true)
                ).arg(actuator_state_arg.clone()
                    .required(true)
                )
            )
        ).subcommand(SubCommand::with_name("timeslot")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("list")
                .arg(actuator_arg.clone()
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("add")
                .arg(actuator_arg.clone()
                    .required(true)
                ).arg(time_interval_arg.clone()
                    .required(true)
                ).arg(actuator_state_arg.clone()
                    .required(true)
                ).arg(start_date_arg.clone()
                    .long("--start-date").short("-s")
                ).arg(end_date_arg.clone()
                    .long("--end-date").short("-e")
                ).arg(weekdays_arg.clone()
                    .long("--weekdays").short("-w")
                )
            ).subcommand(SubCommand::with_name("remove")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("set-time")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                ).group(ArgGroup::with_name("fields")
                    .multiple(true)
                    .required(true)
                ).arg(time_interval_arg.clone()
                    .long("--time-interval").short("-t")
                    .group("fields")
                ).arg(start_date_arg.clone()
                    .long("--start-date").short("-s")
                    .group("fields")
                ).arg(end_date_arg.clone()
                    .long("--end-date").short("-e")
                    .group("fields")
                ).arg(weekdays_arg.clone()
                    .long("--weekdays").short("-w")
                    .group("fields")
                )
            ).subcommand(SubCommand::with_name("set-state")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                )
                .arg(&actuator_state_arg)
            ).subcommand(SubCommand::with_name("disable")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("enable")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                )
            ).subcommand(SubCommand::with_name("add-override")
                .arg(timeslot_specifier_arg.clone()
                    .required(true)
                ).arg(time_interval_arg.clone()
                    .required(true)
                // Require at least one date restriction, otherwise the override would always take
                // over the normal settings.
                ).group(ArgGroup::with_name("date-fields")
                    .multiple(true)
                    .required(true)
                ).arg(start_date_arg.clone()
                    .long("--start-date").short("-s")
                    .group("date-fields")
                ).arg(end_date_arg.clone()
                    .long("--end-date").short("-e")
                    .group("date-fields")
                ).arg(weekdays_arg.clone()
                    .long("--weekdays").short("-w")
                    .group("date-fields")
                )
            ).subcommand(SubCommand::with_name("remove-override")
                .arg(timeslot_override_specifier_arg.clone()
                    .required(true)
                )
            )
        ).subcommand(SubCommand::with_name("schedule")
            .arg(actuator_arg.clone()
                .required(true)
            ).arg(start_date_arg.clone()
                .long("--start-date").short("-s")
            ).arg(Arg::with_name("day-number")
                .takes_value(true)
                .default_value("7")
                .help("Number of days to show")
                .long("--day-number").short("-n")
            )
        ).subcommand(SubCommand::with_name("test")
        ).get_matches();

    let res = match args.subcommand() {
        ("list-actuators", Some(_)) => list_actuators(),
        ("timeslot", Some(sub)) => time_slot(sub),
        ("default-state", Some(sub)) => default_state(sub),
        ("schedule", Some(sub)) => schedule(sub),
        ("test", Some(_)) => test(),
        _ => unreachable!(),
    };

    if let Err(error) = res {
        eprintln!("RPC failed: {}", error);
    }
}
