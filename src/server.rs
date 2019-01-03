use std::collections::BTreeMap;
use std::result;

use actuator::*;
use schedule::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

// TODO: merge with RpcServer, this layer is not useful and doesn't allow for fine-grained
// locks.
pub struct Server {
    actuators: BTreeMap<u32, Actuator>,
    next_actuator_id: u32,
}

impl Server {
    pub fn new() -> Server {
        Server {
            actuators: BTreeMap::new(),
            next_actuator_id: 0,
        }
    }

    // Public API (exposed via RPC)

    pub fn list_actuators(&self) -> BTreeMap<u32, ActuatorInfo> {
        self.actuators.iter().map(|(id, a)| (*id, a.info.clone())).collect()
    }

    pub fn list_timeslots(&self, actuator_id: u32) -> Result<&BTreeMap<u32, TimeSlot>> {
        self.actuator(actuator_id)
            .map(|a| a.timeslots())
    }

    pub fn get_default_state(&self, actuator_id: u32) -> Result<&ActuatorState> {
        self.actuator(actuator_id)
            .map(|a| a.default_state())
    }

    pub fn set_default_state(&mut self,
                             actuator_id: u32,
                             default_state: ActuatorState) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .set_default_state(default_state)
    }

    pub fn add_time_slot(&mut self,
                         actuator_id: u32,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        self.mut_actuator(actuator_id)?
            .add_time_slot(time_period, actuator_state, enabled)
    }

    pub fn remove_time_slot(&mut self, actuator_id: u32, time_slot_id: u32) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .remove_time_slot(time_slot_id)
    }

    pub fn time_slot_set_time_period(&mut self,
                                 actuator_id: u32,
                                 time_slot_id: u32,
                                 time_period: TimePeriod) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .time_slot_set_time_period(time_slot_id, time_period)
    }

    pub fn time_slot_set_enabled(&mut self,
                             actuator_id: u32,
                             time_slot_id: u32,
                             enabled: bool) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .time_slot_set_enabled(time_slot_id, enabled)
    }

    pub fn time_slot_set_actuator_state(&mut self,
                                        actuator_id: u32,
                                        time_slot_id: u32,
                                        actuator_state: ActuatorState) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .time_slot_set_actuator_state(time_slot_id, actuator_state)
    }

    pub fn time_slot_add_time_override(&mut self,
                                       actuator_id: u32,
                                       time_slot_id: u32,
                                       time_period: TimePeriod) -> Result<u32> {
        self.mut_actuator(actuator_id)?
            .time_slot_add_time_override(time_slot_id, time_period)
    }

    pub fn time_slot_remove_time_override(&mut self,
                                          actuator_id: u32,
                                          time_slot_id: u32,
                                          time_override_id: u32) -> Result<()> {
        self.mut_actuator(actuator_id)?
            .time_slot_remove_time_override(time_slot_id, time_override_id)
    }

    // Internal API (not exposed via RPC)

    pub fn add_actuator(&mut self, actuator: Actuator) -> Result<u32> {
        if !(actuator.valid()) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        let id = self.next_actuator_id;
        self.actuators.insert(id, actuator);
        self.next_actuator_id += 1;

        Ok(id)
    }

    pub fn remove_actuator(&mut self, actuator_id: u32) -> Result<()> {
        if self.actuators.remove(&actuator_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(IAE::ActuatorId))
        }
    }

    fn actuator(&self, actuator_id: u32) -> Result<&Actuator> {
        self.actuators.get(&actuator_id).ok_or(InvalidArgument(IAE::ActuatorId))
    }

    fn mut_actuator(&mut self, actuator_id: u32) -> Result<&mut Actuator> {
        self.actuators.get_mut(&actuator_id).ok_or(InvalidArgument(IAE::ActuatorId))
    }
}
