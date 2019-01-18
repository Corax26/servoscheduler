use std::collections::BTreeMap;

use actuator::ActuatorState;
use time::*;
use time_slot::*;

pub struct ScheduleSlot {
    pub time_interval: TimeInterval,
    pub actuator_state: ActuatorState,
    pub timeslot_id: u32,
    pub override_id: Option<u32>,
}

pub type Schedule = BTreeMap<Date, Vec<ScheduleSlot>>;

pub fn compute_schedule(timeslots: &BTreeMap<u32, TimeSlot>,
                        start_date: Date, nb_days: u32) -> Schedule {
    let mut day = start_date.clone();
    let mut schedule = Schedule::new();

    for _ in 0..nb_days {
        let mut slots = Vec::<ScheduleSlot>::new();

        for (id, ts) in timeslots.iter() {
            if let Some((time_interval, override_id)) = ts.time_interval_on(day) {
                slots.push(ScheduleSlot {
                    time_interval,
                    actuator_state: ts.actuator_state.clone(),
                    timeslot_id: *id,
                    override_id,
                });
            }
        }

        // Sort slots by time.
        slots.sort_unstable_by_key(|s| s.time_interval.start);

        schedule.insert(day, slots);
        day += 1;
    }

    schedule
}
