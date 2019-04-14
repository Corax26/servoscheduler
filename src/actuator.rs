use std::collections::BTreeMap;
use std::fmt;
use std::num;
use std::result;
use std::str;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::thread;

use actuator_controller::*;
use schedule;
use time::*;
use time_slot::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

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

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ActuatorState {
    Toggle(bool),
    FloatValue(f64),
}

impl fmt::Display for ActuatorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActuatorState::Toggle(value) => write!(f, "{}", if *value { "On" } else { "Off" }),
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

pub struct Actuator {
    pub info: ActuatorInfo,

    timeslots: BTreeMap<u32, TimeSlot>,
    default_state: ActuatorState,

    next_timeslot_id: u32,
    // TODO: would be nice to be per-timeslot, but shouldn't be exposed via RPC either...
    next_override_id: u32,

    actuator_controller: ActuatorControllerHandle,

    thread_comm: Arc<Mutex<ThreadComm>>,
    thread_comm_cv: Arc<Condvar>,
}
pub type ActuatorHandle = Arc<RwLock<Actuator>>;

impl Actuator {
    pub fn new(info: ActuatorInfo,
               default_state: ActuatorState,
               actuator_controller: ActuatorControllerHandle) -> ActuatorHandle {
        let result_handle = Arc::new(RwLock::new(Actuator {
            info,
            timeslots: BTreeMap::new(),
            default_state: default_state.clone(),
            next_timeslot_id: 0,
            next_override_id: 0,
            actuator_controller,
            thread_comm: Arc::new(Mutex::new(ThreadComm {
                active_timeslot: ActiveTimeSlot::default_state(default_state),
                modified: false,
            })),
            thread_comm_cv: Arc::new(Condvar::new()),
        }));

        let thread_handle = result_handle.clone();

        thread::spawn(move || actuator_thread(thread_handle));

        result_handle
    }

    pub fn timeslots(&self) -> &BTreeMap<u32, TimeSlot> {
        &self.timeslots
    }

    pub fn default_state(&self) -> &ActuatorState {
        &self.default_state
    }

    pub fn set_default_state(&mut self, default_state: ActuatorState) -> Result<()> {
        if !self.valid_state(&default_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        self.default_state = default_state;

        self.update_active_timeslot_and_notify(|active_timeslot| {
            if let DefaultStateActive { .. } = active_timeslot.state {
                // The default state is active, update the actuator state.
                active_timeslot.actuator_state = self.default_state.clone();
            }
        });

        Ok(())
    }

    pub fn add_time_slot(&mut self,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        if !self.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        // Check for overlaps.
        for (id, ts) in self.timeslots.iter() {
            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        // All good, insert the timeslot.
        let id = self.next_timeslot_id;
        self.timeslots.insert(id, TimeSlot::new(enabled, actuator_state, time_period));
        self.next_timeslot_id += 1;

        self.update_active_timeslot_and_notify(|active_timeslot| {
            active_timeslot.update_timeslot_added(self.timeslots.get(&id).unwrap(), id);
        });

        println!("Added time slot, len = {:?}", self.timeslots.len());

        Ok(id)
    }

    pub fn remove_time_slot(&mut self, time_slot_id: u32) -> Result<()> {
        if self.timeslots.remove(&time_slot_id).is_none() {
            return Err(InvalidArgument(IAE::TimeSlotId))
        }

        self.update_active_timeslot_and_notify(|active_timeslot| {
            active_timeslot.update_timeslot_removed(time_slot_id,
                                                    &self.timeslots, &self.default_state);
        });

        Ok(())
    }

    // TODO: the timeslot management logic should be moved to TimeSlot itself (which would also
    // make reference management easier)
    pub fn time_slot_set_time_period(&mut self, time_slot_id: u32,
                                     time_period: TimePeriod) -> Result<()> {
        {
            // Find the matching timeslot and check for overlaps.
            let mut target_ts: Result<&mut TimeSlot> = Err(InvalidArgument(IAE::TimeSlotId));
            for (id, ts) in self.timeslots.iter_mut() {
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
            if time_period.date_range.start != Date::empty_date() {
                new_time_period.date_range.start = time_period.date_range.start;
            }
            if time_period.date_range.end != Date::empty_date() {
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
        };

        self.update_active_timeslot_and_notify(|active_timeslot| {
            // Get the modified timeslot (immutable reference this time).
            let ts = self.timeslots.get(&time_slot_id).unwrap();
            active_timeslot.update_timeslot_modified(ts, time_slot_id,
                                                     &self.timeslots, &self.default_state);
        });

        Ok(())
    }

    pub fn time_slot_set_enabled(&mut self, time_slot_id: u32,
                                 enabled: bool) -> Result<()> {
        let old_enabled = {
            let time_slot = self.timeslots.get_mut(&time_slot_id)
                .ok_or(InvalidArgument(IAE::TimeSlotId))?;

            let old_enabled = time_slot.enabled;
            time_slot.enabled = enabled;
            old_enabled
        };

        if old_enabled != enabled {
            self.update_active_timeslot_and_notify(|active_timeslot| {
                if enabled {
                    // Handle as if a new timeslot were added.
                    let ts = self.timeslots.get(&time_slot_id).unwrap();
                    active_timeslot.update_timeslot_added(ts, time_slot_id);
                } else {
                    // Handle as if the timeslot had been removed.
                    active_timeslot.update_timeslot_removed(time_slot_id,
                                                            &self.timeslots, &self.default_state);
                }
            });
        }

        Ok(())
    }

    pub fn time_slot_set_actuator_state(&mut self, time_slot_id: u32,
                                        actuator_state: ActuatorState) -> Result<()> {
        if !self.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        self.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?
            .actuator_state = actuator_state.clone();

        self.update_active_timeslot_and_notify(|active_timeslot| {
            match active_timeslot.state {
                TimeSlotActive { id, .. } if id == time_slot_id => {
                    // This timeslot is active, update the actuator state.
                    active_timeslot.actuator_state = actuator_state;
                },
                _ => (),
            }
        });

        Ok(())
    }

    pub fn time_slot_add_time_override(&mut self, time_slot_id: u32,
                                       time_period: TimePeriod) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        let new_override_id = self.next_override_id;

        {
            // Find the matching timeslot and check for overlaps.
            let mut target_ts: Option<&mut TimeSlot> = None;
            for (id, ts) in self.timeslots.iter_mut() {
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
                ts.time_override.insert(new_override_id, time_period);
                self.next_override_id += 1;
            } else {
                return Err(InvalidArgument(IAE::TimeSlotId))
            }
        }

        self.update_active_timeslot_and_notify(|active_timeslot| {
            // Same handling as set_time_period().
            let ts = self.timeslots.get(&time_slot_id).unwrap();
            active_timeslot.update_timeslot_modified(ts, time_slot_id,
                                                     &self.timeslots, &self.default_state);
        });

        Ok(new_override_id)
    }

    pub fn time_slot_remove_time_override(&mut self, time_slot_id: u32,
                                          time_override_id: u32) -> Result<()> {
        if self.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?
            .time_override.remove(&time_override_id).is_none()
        {
            return Err(InvalidArgument(IAE::TimeOverrideId))
        }

        self.update_active_timeslot_and_notify(|active_timeslot| {
            // Same handling as set_time_period().
            let ts = self.timeslots.get(&time_slot_id).unwrap();
            active_timeslot.update_timeslot_modified(ts, time_slot_id,
                                                     &self.timeslots, &self.default_state);
        });

        Ok(())
    }

    pub fn set_state(&self, state: ActuatorState) -> Result<()> {
        if !self.valid_state(&state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        self.actuator_controller.lock().unwrap().set_state(&state);

        Ok(())
    }

    fn valid_state(&self, state: &ActuatorState) -> bool {
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

    fn update_active_timeslot_and_notify<F>(&self, func: F)
    where
        F: FnOnce(&mut ActiveTimeSlot)
    {
        let mut thread_comm_guard = self.thread_comm.lock().unwrap();
        let ThreadComm { active_timeslot, modified } = &mut *thread_comm_guard;

        let mut new_active_ts = active_timeslot.clone();
        func(&mut new_active_ts);

        if &new_active_ts != active_timeslot {
            *active_timeslot = new_active_ts;
            *modified = true;
            self.thread_comm_cv.notify_one();
        }
    }
}

impl ValidCheck for Actuator {
    fn valid(&self) -> bool {
        self.info.valid() && self.valid_state(&self.default_state)
    }
}

#[derive(Clone, PartialEq)]
enum ActiveTimeSlotState {
    TimeSlotActive {
        id: u32,
        override_id: Option<u32>,
    },
    DefaultStateActive {
        next_id: Option<u32>,
        next_override_id: Option<u32>,
    },
}
use self::ActiveTimeSlotState::*;

#[derive(Clone, PartialEq)]
struct ActiveTimeSlot {
    state: ActiveTimeSlotState,
    end_time: Time,
    actuator_state: ActuatorState,
}

impl ActiveTimeSlot {
    fn timeslot(id: u32, override_id: Option<u32>,
                end_time: Time, actuator_state: ActuatorState) -> ActiveTimeSlot {
        ActiveTimeSlot {
            state: TimeSlotActive { id, override_id },
            end_time,
            actuator_state,
        }
    }

    fn default_state(actuator_state: ActuatorState) -> ActiveTimeSlot {
        ActiveTimeSlot {
            state: DefaultStateActive {
                next_id: None,
                next_override_id: None,
            },
            end_time: Time::MAX,
            actuator_state,
        }
    }

    fn default_state_until(next_id: u32, next_override_id: Option<u32>,
                           end_time: Time, actuator_state: ActuatorState) -> ActiveTimeSlot {
        ActiveTimeSlot {
            state: DefaultStateActive {
                next_id: Some(next_id),
                next_override_id,
            },
            end_time,
            actuator_state,
        }
    }

    fn compute(now: &DateTime, timeslots: &BTreeMap<u32, TimeSlot>, default_state: ActuatorState)
        -> ActiveTimeSlot
    {
        let next_slot = schedule::find_next_timeslot(timeslots, now);

        if let Some(slot) = next_slot {
            if slot.time_interval.start == now.time {
                Self::timeslot(slot.id, slot.override_id, slot.time_interval.end,
                               slot.actuator_state)
            } else {
                Self::default_state_until(slot.id, slot.override_id, slot.time_interval.start,
                                          default_state)
            }
        } else {
            Self::default_state(default_state)
        }
    }

    fn update_timeslot_added(&mut self, timeslot: &TimeSlot, id: u32) {
        let now = DateTime::now();

        if let DefaultStateActive { .. } = self.state {
            if let Some((time_interval_today, override_id))
                = timeslot.time_interval_on(now.date)
            {
                if time_interval_today.contains(&now.time) {
                    // The new timeslot is currently active.
                    *self = Self::timeslot(
                        id,
                        override_id,
                        time_interval_today.end,
                        timeslot.actuator_state.clone(),
                    );
                } else if now.time < time_interval_today.start &&
                    time_interval_today.start < self.end_time
                {
                    // The new timeslot will become active before any other.
                    *self = Self::default_state_until(
                        id,
                        override_id,
                        time_interval_today.start,
                        self.actuator_state.clone(),
                    );
                }
            }
        }
    }

    fn update_timeslot_removed(&mut self, timeslot_id: u32, timeslots: &BTreeMap<u32, TimeSlot>,
                               default_state: &ActuatorState) {
        let recompute = match self.state {
            // The removed timeslot was active, the default state becomes active.
            TimeSlotActive { id, .. } if id == timeslot_id => true,
            // The removed timeslot was the next timeslot, the next timeslot needs to be
            // recalculated.
            DefaultStateActive { next_id, .. } if next_id == Some(timeslot_id) => true,
            _ => false,
        };

        if recompute {
            *self = Self::compute(&DateTime::now(), &timeslots, default_state.clone());
        }
    }

    fn update_timeslot_modified(&mut self, timeslot: &TimeSlot, timeslot_id: u32,
                                timeslots: &BTreeMap<u32, TimeSlot>,
                                default_state: &ActuatorState) {
        // It would be possible to make a finer-grained analysis, based on exactly how the timeslot
        // was modified, to avoid recalculating today's next timeslot. However, handling this
        // becomes very complex and error-prone, so the focus here is on correctness.

        let mut recompute = false;
        let now = DateTime::now();

        if let Some((time_interval_today, override_id))
            = timeslot.time_interval_on(now.date)
        {
            if time_interval_today.contains(&now.time) {
                // The timeslot is active.
                *self = Self::timeslot(
                    timeslot_id,
                    override_id,
                    time_interval_today.end,
                    timeslot.actuator_state.clone(),
                );
            } else {
                match self.state {
                    TimeSlotActive { id, .. } if id == timeslot_id => {
                        // The timeslot was active and no longer is, the default state becomes
                        // active.
                        recompute = true;
                    },
                    DefaultStateActive { next_id, .. } => {
                        if now.time < time_interval_today.start &&
                            time_interval_today.start <= self.end_time
                        {
                            // The timeslot is the next to become active.
                            *self = Self::default_state_until(
                                timeslot_id,
                                override_id,
                                time_interval_today.start,
                                self.actuator_state.clone(),
                            );
                        } else if next_id == Some(timeslot_id) {
                            // The timeslot was the next to become active and its start time has
                            // been delayed, we need to recalculate the next timeslot.
                            recompute = true;
                        }
                    },
                    _ => (),
                }
            }
        } else {
            // The timeslot doesn't occur today. If it was either the active or the next timeslot,
            // the default state is now active and we need to (re)calculate the next timeslot.
            match self.state {
                TimeSlotActive { id, .. } if id == timeslot_id => {
                    recompute = true;
                },
                DefaultStateActive { next_id, .. } if next_id == Some(timeslot_id) => {
                    recompute = true;
                },
                _ => (),
            }
        }

        if recompute {
            *self = Self::compute(&now, &timeslots, default_state.clone());
        }
    }
}

#[derive(Clone)]
struct ThreadComm {
    active_timeslot: ActiveTimeSlot,
    // The bool is set to true when the active timeslot is modified (to be used with the condvar).
    modified: bool,
}

fn actuator_thread(actuator: ActuatorHandle) {
    let (thread_comm_lock, thread_comm_cv, actuator_controller) = {
        let guard = actuator.read().unwrap();
        (guard.thread_comm.clone(), guard.thread_comm_cv.clone(), guard.actuator_controller.clone())
    };

    let mut now = DateTime::now();

    loop {
        // Note: we never keep the lock. If the active timeslot has been modified, we don't need to
        // keep it (if it gets modified again later on, we will realise during the next iteration),
        // and if we have reached end_time, then we cannot keep it because we need to lock the
        // actuator (risk of deadlock).
        let ThreadComm { active_timeslot, modified } = {
            let mut thread_comm_guard = thread_comm_lock.lock().unwrap();

            // Wait until either end_time, or the active timeslot is modified.
            let end_time = thread_comm_guard.active_timeslot.end_time;
            // In case the timeslot lasts until the end of the day, wait until the start of the
            // next day (one more minute).
            let adjust_min = if end_time == Time::MAX { 1 } else { 0 };

            while !thread_comm_guard.modified {
                now.time = Time::now();
                let wait_sec = (end_time.sub_minute(now.time) + adjust_min) * 60;
                // Theoretically wait_sec can be negative (huge latency between the active timeslot
                // being modified and us being woken up), handle like wait_sec=0 (timeout).
                if wait_sec <= 0 {
                    break;
                }

                let res = thread_comm_cv.wait_timeout(
                    thread_comm_guard,
                    time::Duration::from_secs(wait_sec as u64),
                ).unwrap();
                thread_comm_guard = res.0;

                if res.1.timed_out() {
                    break;
                }
            }

            let thread_comm = thread_comm_guard.clone();
            if thread_comm_guard.modified {
                thread_comm_guard.modified = false;
            }
            thread_comm
        };

        if modified {
            // The active timeslot has been modified, read it.
            let state_str = match active_timeslot.state {
                TimeSlotActive { id, override_id } => format!("timeslot {:?}:{:?}", id, override_id),
                DefaultStateActive { next_id, next_override_id } => format!("default until {:?}:{:?}", next_id, next_override_id),
            };

            let actuator_guard = actuator.read().unwrap();

            println!(
                "[AT {}] {} {}: new state {} ({}) until {}",
                actuator_guard.info.name,
                now.date,
                now.time,
                active_timeslot.actuator_state,
                state_str,
                active_timeslot.end_time
            );

            actuator_controller.lock().unwrap().set_state(&active_timeslot.actuator_state);
        } else {
            // We have reached end_time. Find the new active timeslot.

            // First acquire read access to the Actuator data, to be able to inspect the timeslots.
            let actuator_guard = actuator.read().unwrap();
            // Also lock thread_comm, as we will need to access it in any case.
            let mut thread_comm_guard = thread_comm_lock.lock().unwrap();

            if thread_comm_guard.modified {
                // In the unlikely event that another operation modified thread_comm while we
                // yielded the lock, no need to do anything.
                continue;
            }

            if let DefaultStateActive { next_id: Some(next_id), next_override_id }
                = active_timeslot.state
            {
                // The next timeslot becomes the active one.
                let next_timeslot = actuator_guard.timeslots.get(&next_id).unwrap();
                thread_comm_guard.active_timeslot = ActiveTimeSlot::timeslot(
                    next_id,
                    next_override_id,
                    next_timeslot.time_interval_on(now.date).unwrap().0.end,
                    next_timeslot.actuator_state.clone(),
                );
            } else {
                if active_timeslot.end_time == Time::MAX {
                    // This was the last timeslot for today. Move to the next day.
                    now.date += 1;
                    now.time = Time::MIN;
                } else {
                    now.time = active_timeslot.end_time;
                }

                // Find the next timeslot.
                thread_comm_guard.active_timeslot = ActiveTimeSlot::compute(
                    &now,
                    &actuator_guard.timeslots,
                    actuator_guard.default_state.clone(),
                );
            }

            thread_comm_guard.modified = true;
        }
    }
}
