use std::collections::BTreeMap;

use actuator::ActuatorState;
use time::*;
use utils::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TimePeriod {
    pub time_interval: TimeInterval,
    pub date_range: DateRange,
    pub days: WeekdaySet,
}

impl TimePeriod {
    pub fn overlaps_dates(&self, other: &TimePeriod) -> bool {
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

    pub fn overlaps(&self, other: &TimePeriod) -> bool {
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
    pub fn new(enabled: bool, actuator_state: ActuatorState, time_period: TimePeriod) -> TimeSlot {
        TimeSlot {
            enabled,
            actuator_state,
            time_period,
            time_override: BTreeMap::new(),
        }
    }

    pub fn overlaps(&self, time_period: &TimePeriod) -> bool {
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
