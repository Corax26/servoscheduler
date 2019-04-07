use std::collections::BTreeMap;
use std::sync::Arc;

use actuator::{ActuatorInfo, ActuatorState};
use rpc::SyncService;
use time_slot::*;
use server::*;

pub struct RpcServer {
    pub server: Arc<Server>,
}

impl RpcServer {
    pub fn new(server: Server) -> RpcServer {
        RpcServer {
            server: Arc::new(server),
        }
    }
}

// Implement Clone manually because #[derive] does not use the right bounds and requires Server
// itself to be clonable (which we don't want to allow here), see:
// https://github.com/rust-lang/rust/issues/26925
impl Clone for RpcServer {
    fn clone(&self) -> Self {
        RpcServer {
            server: self.server.clone()
        }
    }
}

impl SyncService for RpcServer {
    fn list_actuators(&self) -> Result<BTreeMap<u32, ActuatorInfo>> {
        Ok(self.server.list_actuators())
    }

    fn list_timeslots(&self, actuator_id: u32) -> Result<BTreeMap<u32, TimeSlot>> {
        self.server.list_timeslots(actuator_id)
    }

    fn get_default_state(&self, actuator_id: u32) -> Result<ActuatorState> {
        self.server.get_default_state(actuator_id)
    }

    fn set_default_state(&self, actuator_id: u32, default_state: ActuatorState) -> Result<()> {
        self.server.set_default_state(actuator_id, default_state)
    }

    fn add_time_slot(&self, actuator_id: u32, time_period: TimePeriod, actuator_state: ActuatorState, enabled: bool) -> Result<u32> {
        self.server.add_time_slot(actuator_id, time_period, actuator_state, enabled)
    }

    fn remove_time_slot(&self, actuator_id: u32, time_slot_id: u32) -> Result<()> {
        self.server.remove_time_slot(actuator_id, time_slot_id)
    }

    fn time_slot_set_time_period(&self, actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> Result<()> {
        self.server.time_slot_set_time_period(actuator_id, time_slot_id, time_period)
    }

    fn time_slot_set_enabled(&self, actuator_id: u32, time_slot_id: u32, enabled: bool) -> Result<()> {
        self.server.time_slot_set_enabled(actuator_id, time_slot_id, enabled)
    }

    fn time_slot_set_actuator_state(&self, actuator_id: u32, time_slot_id: u32, actuator_state: ActuatorState) -> Result<()> {
        self.server.time_slot_set_actuator_state(actuator_id, time_slot_id, actuator_state)
    }

    fn time_slot_add_time_override(&self, actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> Result<u32> {
        self.server.time_slot_add_time_override(actuator_id, time_slot_id, time_period)
    }

    fn time_slot_remove_time_override(&self, actuator_id: u32, time_slot_id: u32, time_override_id: u32) -> Result<()> {
        self.server.time_slot_remove_time_override(actuator_id, time_slot_id, time_override_id)
    }

    fn set_state(&self, actuator_id: u32, state: ActuatorState) -> Result<()> {
        self.server.set_state(actuator_id, state)
    }
}

/* impl FutureService for RpcServer {
    type GetScheduleFut = Result<Schedule>;
    fn get_schedule(&self, actuator_id: u32) -> Self::GetScheduleFut {
        self.server.read().unwrap().get_schedule(actuator_id)
    }

    type SetDefaultStateFut = Result<()>;
    fn set_default_state(&self, actuator_id: u32, default_state: ActuatorState) -> Self::SetDefaultStateFut {
        self.server.write().unwrap().set_default_state(actuator_id, default_state)
    }

    type AddTimeSlotFut = Result<u32>;
    fn add_time_slot(&self,
                     actuator_id: u32,
                     time_period: TimePeriod,
                     actuator_state: ActuatorState,
                     enabled: bool) -> Self::AddTimeSlotFut {
        self.server.write().unwrap().add_time_slot(actuator_id, time_period, actuator_state, enabled)
    }
} */
