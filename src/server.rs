use std::collections::BTreeMap;
use std::result;

use actuator::*;
use schedule::*;
use time::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

pub struct Server {
    actuators: BTreeMap<u32, Actuator>,
    next_actuator_id: u32,
    next_timeslot_id: u32,
    next_override_id: u32,
}

impl Server {
    pub fn new() -> Server {
        Server {
            actuators: BTreeMap::new(),
            next_actuator_id: 0,
            next_timeslot_id: 0,
            next_override_id: 0,
        }
    }

    // Public API (exposed via RPC)

    pub fn list_actuators(&self) -> BTreeMap<u32, ActuatorInfo> {
        self.actuators.iter().map(|(id, a)| (*id, a.info.clone())).collect()
    }

    pub fn get_schedule(&self, actuator_id: u32) -> Result<&Schedule> {
        match self.actuators.get(&actuator_id) {
            Some(actuator) => Ok(&actuator.schedule),
            None => Err(InvalidArgument(IAE::ActuatorId)),
        }
    }

    pub fn set_default_state(&mut self,
                             actuator_id: u32,
                             default_state: ActuatorState) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        if !actuator.valid_state(&default_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        actuator.schedule.default_state = default_state;
        Ok(())
    }

    pub fn add_time_slot(&mut self,
                         actuator_id: u32,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        if !actuator.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        let schedule = &mut actuator.schedule;

        // Check for overlaps.
        for (id, ts) in schedule.timeslots.iter() {
            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        // All good, insert the timeslot.
        let id = self.next_timeslot_id;
        schedule.timeslots.insert(id, TimeSlot::new(enabled, actuator_state, time_period));
        self.next_timeslot_id += 1;

        println!("Added time slot, len = {:?}", schedule.timeslots.len());

        Ok(id)
    }

    pub fn remove_time_slot(&mut self, actuator_id: u32, time_slot_id: u32) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        if actuator.schedule.timeslots.remove(&time_slot_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(IAE::TimeSlotId))
        }
    }

    pub fn time_slot_set_time_period(&mut self,
                                 actuator_id: u32,
                                 time_slot_id: u32,
                                 time_period: TimePeriod) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Result<&mut TimeSlot> = Err(InvalidArgument(IAE::TimeSlotId));
        for (id, ts) in actuator.schedule.timeslots.iter_mut() {
            if *id == time_slot_id {
                target_ts = Ok(ts);
                continue;
            }

            if ts.overlaps(&time_period) {
                target_ts = Err(TimeSlotOverlap(*id));
                break;
            }
        }

        let ts = target_ts?;

        // Update specified fields.
        let mut new_time_period = ts.time_period.clone();

        if time_period.time_interval.start != Time::EMPTY {
            new_time_period.time_interval.start = time_period.time_interval.start;
        }
        if time_period.time_interval.end != Time::EMPTY {
            new_time_period.time_interval.end = time_period.time_interval.end;
        }
        if time_period.date_range.start != Date::EMPTY {
            new_time_period.date_range.start = time_period.date_range.start;
        }
        if time_period.date_range.end != Date::EMPTY {
            new_time_period.date_range.end = time_period.date_range.end;
        }
        if !time_period.days.is_empty() {
            new_time_period.days = time_period.days;
        }

        // Check that the specified fields were valid.
        if !new_time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        // All good, modify the timeslot.
        ts.time_period = new_time_period;
        Ok(())
    }

    pub fn time_slot_set_enabled(&mut self,
                             actuator_id: u32,
                             time_slot_id: u32,
                             enabled: bool) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        let time_slot = actuator.schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        time_slot.enabled = enabled;
        Ok(())
    }

    pub fn time_slot_set_actuator_state(&mut self,
                                    actuator_id: u32,
                                    time_slot_id: u32,
                                    actuator_state: ActuatorState) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        if !actuator.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        let time_slot = actuator.schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        time_slot.actuator_state = actuator_state;
        Ok(())
    }

    pub fn time_slot_add_time_override(&mut self,
                                   actuator_id: u32,
                                   time_slot_id: u32,
                                   time_period: TimePeriod) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Option<&mut TimeSlot> = None;
        for (id, ts) in actuator.schedule.timeslots.iter_mut() {
            if *id == time_slot_id {
                target_ts = Some(ts);
                continue;
            }

            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        if let Some(ts) = target_ts {
            // Also check there is no overlap with other overrides. The requirement is stronger:
            // two overrides cannot apply to the same day (not just day and time).
            for (id, or) in ts.time_override.iter() {
                if or.overlaps_dates(&time_period) {
                    return Err(TimeOverrideOverlap(*id))
                }
            }

            // All good, add the override.
            let id = self.next_override_id;
            ts.time_override.insert(id, time_period);
            self.next_override_id += 1;

            Ok(id)
        } else {
            Err(InvalidArgument(IAE::TimeSlotId))
        }
    }

    pub fn time_slot_remove_time_override(&mut self,
                                      actuator_id: u32,
                                      time_slot_id: u32,
                                      time_override_id: u32) -> Result<()> {
        let actuator = self.actuators.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))?;

        let time_slot = actuator.schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        if time_slot.time_override.remove(&time_override_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(IAE::TimeOverrideId))
        }
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

    // Private
    // TODO: need to make this more borrow-friendly
    // Can probably be done by implementing these functions for actuators directly
    /* fn get_mut_schedule(&mut self, actuator_id: u32) -> Result<&mut Schedule> {
        self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(IAE::ActuatorId))
    }

    fn get_mut_time_slot(&mut self, actuator_id: u32, time_slot_id: u32) -> Result<&mut TimeSlot> {
        self.get_mut_schedule(actuator_id)?
            .timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))
    } */
}
