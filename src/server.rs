use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::result;

use serde_yaml;

use actuator::*;
use actuator_controller::*;
use time_slot::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

// TODO: merge with RpcServer?
pub struct Server {
    actuators: Vec<ActuatorHandle>,
}

impl Server {
    pub fn new(config_file: impl Read) -> result::Result<Server, String> {
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum ConfigActuatorController {
            File { path: String },
        };
        // We can't modify ActuatorState's serde attributes directly, as otherwise tarpc would
        // complain, so as a workaround we create a mirror struct.
        #[derive(Deserialize)]
        #[serde(untagged)]
        pub enum ConfigActuatorState {
            Toggle(bool),
            FloatValue(f64),
        }
        #[derive(Deserialize)]
        struct ConfigActuator {
            name: String,
            actuator_type: ActuatorType,
            default_state: ConfigActuatorState,
            controller: ConfigActuatorController,
        }
        #[derive(Deserialize)]
        struct ConfigFile {
            actuators: Vec<ConfigActuator>,
        }

        let config: ConfigFile = serde_yaml::from_reader(config_file)
            .map_err(|e| format!("Reading config file failed: {}", e))?;

        let mut actuators = Vec::<ActuatorHandle>::new();

        for ca in config.actuators {
            let controller = match ca.controller {
                ConfigActuatorController::File { ref path } => {
                    FileActuatorController::new(Path::new(&path))
                },
            }.map_err(|e| format!("Failed to create controller for actuator {}: {}", ca.name, e))?;

            let default_state = match ca.default_state {
                ConfigActuatorState::Toggle(b) => ActuatorState::Toggle(b),
                ConfigActuatorState::FloatValue(f) => ActuatorState::FloatValue(f),
            };

            let actuator = Actuator::new(
                ActuatorInfo {
                    name: ca.name.clone(),
                    actuator_type: ca.actuator_type,
                },
                default_state,
                controller,
            );

            if !actuator.read().unwrap().valid() {
                return Err(format!("Invalid configuration for actuator {}", ca.name))
            }

            actuators.push(actuator);
        }

        Ok(Server {
            actuators,
        })
    }

    // Public API (exposed via RPC)

    pub fn list_actuators(&self) -> Vec<ActuatorInfo> {
        self.actuators.iter()
            .map(|a| a.read().unwrap().info.clone())
            .collect()
    }

    pub fn list_timeslots(&self, actuator_id: u32) -> Result<BTreeMap<u32, TimeSlot>> {
        self.read_actuator(actuator_id,
                           |a| Ok(a.timeslots().clone()))
    }

    pub fn get_default_state(&self, actuator_id: u32) -> Result<ActuatorState> {
        self.read_actuator(actuator_id,
                           |a| Ok(a.default_state().clone()))
    }

    pub fn set_default_state(&self,
                             actuator_id: u32,
                             default_state: ActuatorState) -> Result<()> {
        self.write_actuator(actuator_id,
                            |a| a.set_default_state(default_state))
    }

    pub fn add_time_slot(&self,
                         actuator_id: u32,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        self.write_actuator(actuator_id,
                            |a| a.add_time_slot(time_period, actuator_state, enabled))
    }

    pub fn remove_time_slot(&self, actuator_id: u32, time_slot_id: u32) -> Result<()> {
        self.write_actuator(actuator_id,
                            |a| a.remove_time_slot(time_slot_id))
    }

    pub fn time_slot_set_time_period(&self,
                                 actuator_id: u32,
                                 time_slot_id: u32,
                                 time_period: TimePeriod) -> Result<()> {
        self.write_actuator(actuator_id,
            |a| a.time_slot_set_time_period(time_slot_id, time_period))
    }

    pub fn time_slot_set_enabled(&self,
                             actuator_id: u32,
                             time_slot_id: u32,
                             enabled: bool) -> Result<()> {
        self.write_actuator(actuator_id,
            |a| a.time_slot_set_enabled(time_slot_id, enabled))
    }

    pub fn time_slot_set_actuator_state(&self,
                                        actuator_id: u32,
                                        time_slot_id: u32,
                                        actuator_state: ActuatorState) -> Result<()> {
        self.write_actuator(actuator_id,
            |a| a.time_slot_set_actuator_state(time_slot_id, actuator_state))
    }

    pub fn time_slot_add_time_override(&self,
                                       actuator_id: u32,
                                       time_slot_id: u32,
                                       time_period: TimePeriod) -> Result<u32> {
        self.write_actuator(actuator_id,
            |a| a.time_slot_add_time_override(time_slot_id, time_period))
    }

    pub fn time_slot_remove_time_override(&self,
                                          actuator_id: u32,
                                          time_slot_id: u32,
                                          time_override_id: u32) -> Result<()> {
        self.write_actuator(actuator_id,
            |a| a.time_slot_remove_time_override(time_slot_id, time_override_id))
    }

    pub fn set_state(&self, actuator_id: u32, state: ActuatorState) -> Result<()> {
        self.read_actuator(actuator_id, |a| a.set_state(state))
    }


    fn read_actuator<F, T>(&self, actuator_id: u32, func: F) -> Result<T>
    where
        F: FnOnce(&Actuator) -> Result<T>
    {
        let actuator_handle =
            self.actuators.get(actuator_id as usize).ok_or(InvalidArgument(IAE::ActuatorId))?;
        func(&actuator_handle.read().unwrap())
    }

    fn write_actuator<F, T>(&self, actuator_id: u32, func: F) -> Result<T>
    where
        F: FnOnce(&mut Actuator) -> Result<T>
    {
        let actuator_handle =
            self.actuators.get(actuator_id as usize).ok_or(InvalidArgument(IAE::ActuatorId))?;
        func(&mut *actuator_handle.write().unwrap())
    }
}
