use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::result;
use std::str;

use chrono;
use chrono::Datelike;
use regex::Regex;

use utils::*;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Date {
    // Use chrono's representation, because it makes it much easier to manipulate the date and
    // provides fast access to metadata (like weekday).
    chrono_date: chrono::NaiveDate,
}
pub type DateRange = InclusiveRange<Date>;

impl Date {
    // Use valid values because it's much easier to handle (no need to special-case).
    pub const MIN: Date = Date { chrono_date: chrono::naive::MIN_DATE };
    pub const MAX: Date = Date { chrono_date: chrono::naive::MAX_DATE };
    // This is rather arbitrary. Ideally it would be an invalid value, but chrono does not allow
    // this. It also cannot be implemented as a const member, because ::from_yo() is not a constant
    // function.
    pub fn empty_date() -> Date {
        Date::from(chrono::NaiveDate::from_yo(1, 1))
    }

    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Date> {
        chrono::NaiveDate::from_ymd_opt(year, month, day).map(|cd| Date::from(cd))
    }

    pub fn today() -> Date {
        Date::from(chrono::offset::Local::today().naive_local())
    }

    pub fn year(&self) -> i32 {
        self.chrono_date.year()
    }

    pub fn month(&self) -> u32 {
        self.chrono_date.month()
    }

    pub fn day(&self) -> u32 {
        self.chrono_date.day()
    }

    pub fn weekday(&self) -> WeekdaySet {
        let idx = self.chrono_date.weekday().num_days_from_monday();
        WeekdaySet::from_bits(1 << idx).unwrap()
    }
}

impl From<chrono::NaiveDate> for Date {
    fn from(chrono_date: chrono::NaiveDate) -> Self {
        Date { chrono_date }
    }
}

impl ValidCheck for Date {
    fn valid(&self) -> bool {
        *self != Date::empty_date()
    }
}

impl Add<i64> for Date {
    type Output = Date;

    fn add(self, rhs: i64) -> Date {
        Date::from(self.chrono_date + chrono::Duration::days(rhs))
    }
}

impl AddAssign<i64> for Date {
    fn add_assign(&mut self, rhs: i64) {
        self.chrono_date += chrono::Duration::days(rhs);
    }
}

impl Sub<i64> for Date {
    type Output = Date;

    fn sub(self, rhs: i64) -> Date {
        Date::from(self.chrono_date - chrono::Duration::days(rhs))
    }
}

impl SubAssign<i64> for Date {
    fn sub_assign(&mut self, rhs: i64) {
        self.chrono_date -= chrono::Duration::days(rhs);
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Date::MIN | &Date::MAX => write!(f, "-"),
            _ => write!(f, "{:02}/{:02}/{}", self.day(), self.month(), self.year()),
        }
    }
}

impl str::FromStr for Date {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let re = Regex::new(r"^(\d+)/(\d+)(?:/(\d+))?$").unwrap();
        match re.captures(s) {
            Some(caps) => Date::from_ymd(
                {
                    if let Some(year) = caps.get(3) {
                        // We need to handle the error case, because although the regex validates that
                        // the capture is an integer, it may not be representable as u8.
                        i32::from_str(year.as_str()).or(Err(()))?
                    } else {
                        Date::today().year()
                    }
                },
                u32::from_str(&caps[2]).or(Err(()))?,
                u32::from_str(&caps[1]).or(Err(()))?,
            ).ok_or(()),
            None => Err(())
        }
    }
}

impl DateRange {
    // Must be a range of valid dates.
    pub fn weekday_set(&self) -> WeekdaySet {
        let start_day = self.start.chrono_date.weekday().num_days_from_monday();
        let num_day_diff = self.end.chrono_date.signed_duration_since(self.start.chrono_date).num_days() as u32;

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

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}
pub type TimeInterval = ExclusiveRange<Time>;

impl Time {
    // Used to define a special order so that days start at DAY_START_HOUR (instead of midnight).
    pub const DAY_START_HOUR: u8 = 4;
    pub const EMPTY: Time = Time { hour: 25, minute: 0 };

    fn shifted_hour(&self) -> u8 {
        (self.hour + 24 - Self::DAY_START_HOUR) % 24
    }
}

impl ValidCheck for Time {
    fn valid(&self) -> bool {
        self.hour < 24 && self.minute < 60
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        match self.shifted_hour().partial_cmp(&other.shifted_hour()) {
            Some(Ordering::Equal) => self.minute.partial_cmp(&other.minute),
            r => r
        }
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Time) -> Ordering {
        match self.shifted_hour().cmp(&other.shifted_hour()) {
            Ordering::Equal => self.minute.cmp(&other.minute),
            r => r
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

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

