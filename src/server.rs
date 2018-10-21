use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::num;
use std::result;
use std::str;

use chrono::Datelike;
use regex::Regex;

use utils::*;

// Actuator.
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
            ActuatorState::Toggle(value) => write!(f, "{}", if *value { "On" } else { "Off "}),
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
pub struct Actuator {
    pub name: String,
    pub actuator_type: ActuatorType,
}

impl Actuator {
    fn valid_state(&self, state: &ActuatorState) -> bool {
        match self.actuator_type {
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
        match self.actuator_type {
            ActuatorType::Toggle => true,
            ActuatorType::FloatValue { min, max } => min < max,
        }
    }
}

// Time constructs.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Debug)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    // Use valid values because it's much easier to handle (no need to special-case).
    pub const MIN: Date = Date { year: 1970, month: 1, day: 1 };
    pub const MAX: Date = Date { year: u16::max_value(), month: 12, day: 31 };
    pub const EMPTY: Date = Date { year: 0, month: 0, day: 0 };

    fn to_chrono_naive_date(&self) -> Option<::chrono::naive::NaiveDate> {
        ::chrono::naive::NaiveDate::from_ymd_opt(self.year as i32,
                                               self.month as u32,
                                               self.day as u32)
    }

    // Must be a range of valid dates.
    fn weekday_set(range: &DateRange) -> WeekdaySet {
        let start_naive_date = range.start.to_chrono_naive_date().unwrap();
        let end_naive_date = range.end.to_chrono_naive_date().unwrap();

        let start_day = start_naive_date.weekday().num_days_from_monday();
        let num_day_diff = end_naive_date.signed_duration_since(start_naive_date).num_days() as u32;

        if num_day_diff >= 6 {
            WeekdaySet::all()
        } else if start_day + num_day_diff <= 6 {
            // No wrapping around, the end weekday index is greater than the start.
            WeekdaySet::from_bits(bit_range::<u8>(start_day, start_day + num_day_diff)).unwrap()
        } else {
            // Wrapping around (the range includes Sunday and Monday).
            let start_to_sunday = bit_range::<u8>(start_day, 6);
            let monday_to_end = bit_range::<u8>(0, (start_day + num_day_diff) % 7);

            WeekdaySet::from_bits(start_to_sunday | monday_to_end).unwrap()
        }
    }
}

impl ValidCheck for Date {
    fn valid(&self) -> bool {
        self.to_chrono_naive_date() != None
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Date::MIN | &Date::MAX => write!(f, "-"),
            _ => write!(f, "{:02}/{:02}/{}", self.day, self.month, self.year),
        }
    }
}

impl str::FromStr for Date {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let re = Regex::new(r"^(\d+)/(\d+)(?:/(\d+))?$").unwrap();
        match re.captures(s) {
            Some(caps) => Ok(Date {
                year: {
                    if let Some(year) = caps.get(3) {
                        u16::from_str(year.as_str()).unwrap()
                    } else {
                        ::chrono::offset::Local::now().year() as u16
                    }
                },
                month: u8::from_str(&caps[2]).unwrap(),
                day: u8::from_str(&caps[1]).unwrap(),
            }),
            None => Err(())
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

impl Time {
    const DAY_START_HOUR: u8 = 4;
    pub const EMPTY: Time = Time { hour: 25, minute: 0 };
}

impl ValidCheck for Time {
    fn valid(&self) -> bool {
        self.hour < 24 && self.minute < 60
    }
}

impl PartialOrd for Time {
    // Special order so that days start at DAY_START_HOUR (instead of midnight).
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        let shift = |h| (h + 24 - Self::DAY_START_HOUR) % 24;

        match shift(self.hour).partial_cmp(&shift(other.hour)) {
            Some(Ordering::Equal) => self.minute.partial_cmp(&other.minute),
            r => r
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct WeekdaySet: u8 {
        const MONDAY    = 0b0000001;
        const TUESDAY   = 0b0000010;
        const WEDNESDAY = 0b0000100;
        const THURSDAY  = 0b0001000;
        const FRIDAY    = 0b0010000;
        const SATURDAY  = 0b0100000;
        const SUNDAY    = 0b1000000;
    }
}

impl WeekdaySet {
    const TEXT_REPR: [char; 7] = ['M', 'T', 'W', 'T', 'F', 'S' ,'S'];
}

impl fmt::Display for WeekdaySet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = Self::TEXT_REPR.clone();
        let mut day_bits = self.bits();

        for i in 0..7 {
            if (day_bits & 1) == 0 {
                s[i] = '-';
            }
            day_bits >>= 1;
        }

        f.write_str(&s.into_iter().collect::<String>())
    }
}

impl str::FromStr for WeekdaySet {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        if s.len() != 7 {
            return Err(())
        }

        let mut day_bits = 0;
        for (i, c) in s.char_indices() {
            if c == Self::TEXT_REPR[i] {
                day_bits |= 1 << i;
            } else if c != '-' {
                return Err(());
            }
        }

        Ok(WeekdaySet::from_bits(day_bits).unwrap())
    }
}

pub type TimeInterval = ExclusiveRange<Time>;

impl str::FromStr for TimeInterval {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let re = Regex::new(r"^(\d+):(\d+)-(\d+):(\d+)$").unwrap();
        match re.captures(s) {
            Some(caps) => Ok(TimeInterval {
                start: Time {
                    hour: u8::from_str(&caps[1]).unwrap(),
                    minute: u8::from_str(&caps[2]).unwrap(),
                },
                end: Time {
                    hour: u8::from_str(&caps[3]).unwrap(),
                    minute: u8::from_str(&caps[4]).unwrap(),
                }
            }),
            None => Err(())
        }
    }
}

pub type DateRange = InclusiveRange<Date>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TimePeriod {
    pub time_interval: TimeInterval,
    pub date_range: DateRange,
    pub days: WeekdaySet,
}

impl TimePeriod {
    fn overlaps_dates(&self, other: &TimePeriod) -> bool {
        if let Some(intersection) = self.date_range.intersection(&other.date_range) {
            if self.days.is_all() && other.days.is_all() {
                // Fast path: both repeat every day, no need to check weekdays.
                true
            } else {
                // There must be at least one day included in the intersection and both of the time
                // periods.
                let intersect_weekdays = Date::weekday_set(&intersection);

                !(intersect_weekdays & self.days & other.days).is_empty()
            }
        } else {
            false
        }
    }

    fn overlaps(&self, other: &TimePeriod) -> bool {
        self.overlaps_dates(other) && self.time_interval.overlaps(&other.time_interval)
    }
}

impl ValidCheck for TimePeriod {
    fn valid(&self) -> bool {
        self.time_interval.valid() && self.date_range.valid() && !self.days.is_empty()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TimeSlot {
    pub enabled: bool,
    pub actuator_state: ActuatorState,
    pub time_period: TimePeriod,
    pub time_override: BTreeMap<u32, TimePeriod>,
}

impl TimeSlot {
    fn new(enabled: bool, actuator_state: ActuatorState, time_period: TimePeriod) -> TimeSlot {
        TimeSlot {
            enabled,
            actuator_state,
            time_period,
            time_override: BTreeMap::new(),
        }
    }

    fn overlaps(&self, time_period: &TimePeriod) -> bool {
        if self.time_period.overlaps_dates(&time_period) {
            if self.time_period.time_interval.overlaps(&time_period.time_interval) {
                return true
            }

            for or in self.time_override.values() {
                if or.overlaps(&time_period) {
                    return true
                }
            }
        }

        return false
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub timeslots: BTreeMap<u32, TimeSlot>,
    pub default_state: ActuatorState,
}

impl Schedule {
    fn new(default_state: ActuatorState) -> Schedule {
        Schedule {
            timeslots: BTreeMap::new(),
            default_state
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InvalArgError {
    ActuatorId,
    TimeSlotId,
    TimeOverrideId,
    TimePeriod,
    ActuatorState,
}
use self::InvalArgError::*;

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
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidArgument(ref arg) => write!(f, "invalid argument: {}", arg),
            Error::TimeSlotOverlap(id) => write!(f, "overlap with time slot (ID {})", id),
            Error::TimeOverrideOverlap(id) =>
                write!(f, "overlap with another time override in this slot(ID {})", id),
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

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    actuators: BTreeMap<u32, Actuator>,
    schedules: BTreeMap<u32, Schedule>,
    next_actuator_id: u32,
    next_timeslot_id: u32,
    next_override_id: u32,
}

impl Server {
    pub fn new() -> Server {
        Server {
            actuators: BTreeMap::new(),
            schedules: BTreeMap::new(),
            next_actuator_id: 0,
            next_timeslot_id: 0,
            next_override_id: 0,
        }
    }

    // Public API (exposed via RPC)

    pub fn list_actuators(&self) -> &BTreeMap<u32, Actuator> {
        &self.actuators
    }

    pub fn get_schedule(&self, actuator_id: u32) -> Result<&Schedule> {
        match self.schedules.get(&actuator_id) {
            Some(schedule) => Ok(&schedule),
            None => Err(InvalidArgument(ActuatorId)),
        }
    }

    pub fn set_default_state(&mut self,
                             actuator_id: u32,
                             default_state: ActuatorState) -> Result<()> {
        let actuator = self.actuators.get(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        if !actuator.valid_state(&default_state) {
            return Err(InvalidArgument(ActuatorState))
        }

        let schedule = self.schedules.get_mut(&actuator_id).unwrap();

        schedule.default_state = default_state;
        Ok(())
    }

    pub fn add_time_slot(&mut self,
                         actuator_id: u32,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(TimePeriod))
        }

        let actuator = self.actuators.get(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        if !actuator.valid_state(&actuator_state) {
            return Err(InvalidArgument(ActuatorState))
        }

        let schedule = self.schedules.get_mut(&actuator_id).unwrap();

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
        let schedule = self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        if schedule.timeslots.remove(&time_slot_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(TimeSlotId))
        }
    }

    pub fn time_slot_set_time_period(&mut self,
                                 actuator_id: u32,
                                 time_slot_id: u32,
                                 time_period: TimePeriod) -> Result<()> {
        let schedule = self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Result<&mut TimeSlot> = Err(InvalidArgument(TimeSlotId));
        for (id, ts) in schedule.timeslots.iter_mut() {
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
            return Err(InvalidArgument(TimePeriod))
        }

        // All good, modify the timeslot.
        ts.time_period = new_time_period;
        Ok(())
    }

    pub fn time_slot_set_enabled(&mut self,
                             actuator_id: u32,
                             time_slot_id: u32,
                             enabled: bool) -> Result<()> {
        let schedule = self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        let time_slot = schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(TimeSlotId))?;

        time_slot.enabled = enabled;
        Ok(())
    }

    pub fn time_slot_set_actuator_state(&mut self,
                                    actuator_id: u32,
                                    time_slot_id: u32,
                                    actuator_state: ActuatorState) -> Result<()> {
        let actuator = self.actuators.get(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        if !actuator.valid_state(&actuator_state) {
            return Err(InvalidArgument(ActuatorState))
        }

        let schedule = self.schedules.get_mut(&actuator_id).unwrap();

        let time_slot = schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(TimeSlotId))?;

        time_slot.actuator_state = actuator_state;
        Ok(())
    }

    pub fn time_slot_add_time_override(&mut self,
                                   actuator_id: u32,
                                   time_slot_id: u32,
                                   time_period: TimePeriod) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(TimePeriod))
        }

        let schedule = self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Option<&mut TimeSlot> = None;
        for (id, ts) in schedule.timeslots.iter_mut() {
            if *id == time_slot_id {
                target_ts = Some(ts);
                continue;
            }

            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        if let Some(ts) = target_ts {
            // Also check there is no overlap with other overrides.
            for (id, or) in ts.time_override.iter() {
                if or.overlaps(&time_period) {
                    return Err(TimeOverrideOverlap(*id))
                }
            }

            // All good, add the override.
            let id = self.next_override_id;
            ts.time_override.insert(id, time_period);
            self.next_override_id += 1;

            Ok(id)
        } else {
            Err(InvalidArgument(TimeSlotId))
        }
    }

    pub fn time_slot_remove_time_override(&mut self,
                                      actuator_id: u32,
                                      time_slot_id: u32,
                                      time_override_id: u32) -> Result<()> {
        let schedule = self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))?;

        let time_slot = schedule.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(TimeSlotId))?;

        if time_slot.time_override.remove(&time_override_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(TimeOverrideId))
        }
    }

    // Internal API (not exposed via RPC)

    pub fn add_actuator(&mut self, actuator: Actuator, default_state: ActuatorState) -> Result<u32> {
        if !(actuator.valid() && actuator.valid_state(&default_state)) {
            return Err(InvalidArgument(ActuatorState))
        }

        let id = self.next_actuator_id;
        self.actuators.insert(id, actuator);
        self.schedules.insert(id, Schedule::new(default_state));
        self.next_actuator_id += 1;

        Ok(id)
    }

    pub fn remove_actuator(&mut self, actuator_id: u32) -> Result<()> {
        if self.actuators.remove(&actuator_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(ActuatorId))
        }
    }

    // Private
    // TODO: need to make this more borrow-friendly (nested struct?)
    /* fn get_mut_schedule(&mut self, actuator_id: u32) -> Result<&mut Schedule> {
        self.schedules.get_mut(&actuator_id)
            .ok_or(InvalidArgument(ActuatorId))
    }

    fn get_mut_time_slot(&mut self, actuator_id: u32, time_slot_id: u32) -> Result<&mut TimeSlot> {
        self.get_mut_schedule(actuator_id)?
            .timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(TimeSlotId))
    } */
}
