use num::Num;
use std::ops::Shl;

pub trait ValidCheck {
    fn valid(&self) -> bool;
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct InclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd + Copy> InclusiveRange<T> {
    pub fn overlaps(&self, other: &InclusiveRange<T>) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    pub fn intersection(&self, other: &InclusiveRange<T>) -> Option<InclusiveRange<T>> {
        if self.overlaps(&other) {
            if self.start <= other.start {
                return Some(InclusiveRange { start: other.start, end: self.end })
            } else {
                return Some(InclusiveRange { start: self.start, end: other.end })
            }
        } else {
            return None
        }
    }
}

impl<T: ValidCheck + PartialOrd> ValidCheck for InclusiveRange<T> {
    fn valid(&self) -> bool {
        self.start.valid() && self.end.valid() && self.start <= self.end
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ExclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd + Copy> ExclusiveRange<T> {
    pub fn overlaps(&self, other: &ExclusiveRange<T>) -> bool {
        self.start < other.end && other.start < self.end
    }
}

impl<T: ValidCheck + PartialOrd> ValidCheck for ExclusiveRange<T> {
    fn valid(&self) -> bool {
        self.start.valid() && self.end.valid() && self.start < self.end
    }
}

// Set bits <start> to <end> (inclusive), clearing the others, and return the result.
pub fn bit_range<T: Num + Shl<u32, Output=T>>(start: u32, end: u32) -> T {
    ((T::one() << (end - start + 1)) - T::one()) << start
}
