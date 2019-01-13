use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::result;

use actuator::*;
use time_slot::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

// TODO: merge with RpcServer?
pub struct Server {
    actuators: BTreeMap<u32, ActuatorHandle>,
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
        self.actuators.iter()
            .map(|(id, a)| (*id, a.read().unwrap().info.clone())).collect()
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

    // Internal API (not exposed via RPC)

    pub fn add_actuator(&mut self, actuator: ActuatorHandle) -> Result<u32> {
        if !(actuator.read().unwrap().valid()) {
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

    fn read_actuator<F, T>(&self, actuator_id: u32, func: F) -> Result<T>
    where
        F: FnOnce(&Actuator) -> Result<T>
    {
        let actuator_handle =
            self.actuators.get(&actuator_id).ok_or(InvalidArgument(IAE::ActuatorId))?;
        func(&actuator_handle.read().unwrap())
    }

    fn write_actuator<F, T>(&self, actuator_id: u32, func: F) -> Result<T>
    where
        F: FnOnce(&mut Actuator) -> Result<T>
    {
        let actuator_handle =
            self.actuators.get(&actuator_id).ok_or(InvalidArgument(IAE::ActuatorId))?;
        // Explicit call to .deref_mut() needed because of
        // https://github.com/rust-lang/rust/issues/26186
        func(actuator_handle.write().unwrap().deref_mut())
    }
}
