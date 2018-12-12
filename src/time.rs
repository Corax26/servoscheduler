use std::cmp::Ordering;
use std::fmt;
use std::result;
use std::str;

use chrono::Datelike;
use regex::Regex;

use utils::*;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Debug)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}
pub type DateRange = InclusiveRange<Date>;

impl Date {
    // Use valid values because it's much easier to handle (no need to special-case).
    pub const MIN: Date = Date { year: 1970, month: 1, day: 1 };
    pub const MAX: Date = Date { year: u16::max_value(), month: 12, day: 31 };
    // Not a valid value (self.valid() == false).
    pub const EMPTY: Date = Date { year: 0, month: 0, day: 0 };

    fn to_chrono_naive_date(&self) -> Option<::chrono::naive::NaiveDate> {
        ::chrono::naive::NaiveDate::from_ymd_opt(self.year as i32,
                                               self.month as u32,
                                               self.day as u32)
    }

    // Must be a range of valid dates.
    pub fn weekday_set(range: &DateRange) -> WeekdaySet {
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
                        // We need to handle the error case, because although the regex validates that
                        // the capture is an integer, it may not be representable as u8.
                        u16::from_str(year.as_str()).or(Err(()))?
                    } else {
                        ::chrono::offset::Local::now().year() as u16
                    }
                },
                month: u8::from_str(&caps[2]).or(Err(()))?,
                day: u8::from_str(&caps[1]).or(Err(()))?,
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
pub type TimeInterval = ExclusiveRange<Time>;

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

