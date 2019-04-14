use std::collections::BTreeMap;
use std::error;
use std::fmt;

use actuator::{ActuatorInfo, ActuatorState};
use time_slot::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum InvalArgError {
    ActuatorId,
    TimeSlotId,
    TimeOverrideId,
    TimePeriod,
    ActuatorState,
}

impl fmt::Display for InvalArgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let desc = match *self {
            InvalArgError::ActuatorId => "actuator ID",
            InvalArgError::TimeSlotId => "time slot ID",
            InvalArgError::TimeOverrideId => "time override ID",
            InvalArgError::TimePeriod => "time period",
            InvalArgError::ActuatorState => "actuator state",
        };
        f.write_str(desc)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Error {
    InvalidArgument(InvalArgError),
    TimeSlotOverlap(u32),
    TimeOverrideOverlap(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidArgument(ref arg) => write!(f, "invalid argument: {}", arg),
            Error::TimeSlotOverlap(id) => write!(f, "overlap with time slot (ID {})", id),
            Error::TimeOverrideOverlap(id) =>
                write!(f, "overlap with another time override in this slot (ID {})", id),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<InvalArgError> for Error {
    fn from(error: InvalArgError) -> Self {
        Error::InvalidArgument(error)
    }
}

service! {
    // Specifying | Error anyway, because tarpc::util::Never is a pain to handle.
    rpc list_actuators() -> Vec<ActuatorInfo> | Error;
    rpc list_timeslots(actuator_id: u32) -> BTreeMap<u32, TimeSlot> | Error;

    rpc get_default_state(actuator_id: u32) -> ActuatorState | Error;
    rpc set_default_state(actuator_id: u32, default_state: ActuatorState) -> () | Error;

    rpc add_time_slot(actuator_id: u32, time_period: TimePeriod, actuator_state: ActuatorState, enabled: bool) -> u32 | Error;
    // TODO: choose one spelling: time_slot or timeslot
    rpc remove_time_slot(actuator_id: u32, time_slot_id: u32) -> () | Error;
    // Allows time_period's fields to be empty.
    rpc time_slot_set_time_period(actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> () | Error;
    rpc time_slot_set_enabled(actuator_id: u32, time_slot_id: u32, enabled: bool) -> () | Error;
    rpc time_slot_set_actuator_state(actuator_id: u32, time_slot_id: u32, actuator_state: ActuatorState) -> () | Error;
    rpc time_slot_add_time_override(actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> u32 | Error;
    rpc time_slot_remove_time_override(actuator_id: u32, time_slot_id: u32, time_override_id: u32) -> () | Error;

    rpc set_state(actuator_id: u32, state: ActuatorState) -> () | Error;
}
