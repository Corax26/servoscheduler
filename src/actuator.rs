use std::fmt;
use std::num;
use std::result;
use std::str;

use schedule::Schedule;
use utils::*;

#[derive(Clone, Serialize, Deserialize)]
pub enum ActuatorType {
    Toggle,
    FloatValue { min: f64, max: f64 },
}

impl fmt::Display for ActuatorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActuatorType::Toggle => write!(f, "Toggle"),
            ActuatorType::FloatValue { min, max } => write!(f, "Float [{}, {}]", min, max),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ActuatorState {
    Toggle(bool),
    FloatValue(f64),
}

impl fmt::Display for ActuatorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActuatorState::Toggle(value) => write!(f, "{}", if *value { "On" } else { "Off " }),
            ActuatorState::FloatValue(value) => write!(f, "{}", value),
        }
    }
}

impl str::FromStr for ActuatorState {
    type Err = num::ParseFloatError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "on" => Ok(ActuatorState::Toggle(true)),
            "off" => Ok(ActuatorState::Toggle(false)),
            _ => f64::from_str(s).map(|f| ActuatorState::FloatValue(f))
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActuatorInfo {
    pub name: String,
    pub actuator_type: ActuatorType,
}

impl ValidCheck for ActuatorInfo {
    fn valid(&self) -> bool {
        match self.actuator_type {
            ActuatorType::Toggle => true,
            ActuatorType::FloatValue { min, max } => min < max,
        }
    }
}

pub struct Actuator{
    pub info: ActuatorInfo,
    // TODO: make private, and move the implementation of methods from Server to here (allowing
    // to modify the actuator's internal state)
    pub schedule: Schedule,
    // rest: internals
}

impl Actuator {
    pub fn new(info: ActuatorInfo, default_state: ActuatorState) -> Actuator {
        Actuator {
            info,
            schedule: Schedule::new(default_state),
        }
    }

    pub fn valid_state(&self, state: &ActuatorState) -> bool {
        match self.info.actuator_type {
            ActuatorType::Toggle => match state {
                &ActuatorState::Toggle(_) => true,
                _ => false,
            },
            ActuatorType::FloatValue { min, max } => match state {
                &ActuatorState::FloatValue(value) => (min <= value && value <= max),
                _ => false
            },
        }
    }
}

impl ValidCheck for Actuator {
    fn valid(&self) -> bool {
        self.info.valid() && self.valid_state(&self.schedule.default_state)
    }
}
