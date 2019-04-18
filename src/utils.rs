use num::Num;
use std::cmp::{min,max};
use std::ops::Shl;

pub trait ValidCheck {
    fn valid(&self) -> bool;
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct InclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: Ord + Copy> InclusiveRange<T> {
    pub fn overlaps(&self, other: &InclusiveRange<T>) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    pub fn intersection(&self, other: &InclusiveRange<T>) -> Option<InclusiveRange<T>> {
        let start = max(self.start, other.start);
        let end = min(self.end, other.end);
        if start <= end {
            return Some(InclusiveRange { start, end })
        } else {
            return None
        }
    }

    pub fn contains(&self, elem: &T) -> bool {
        self.start <= *elem && *elem <= self.end
    }
}

impl<T: ValidCheck + Ord> ValidCheck for InclusiveRange<T> {
    fn valid(&self) -> bool {
        self.start.valid() && self.end.valid() && self.start <= self.end
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ExclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: Ord + Copy> ExclusiveRange<T> {
    pub fn overlaps(&self, other: &ExclusiveRange<T>) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn contains(&self, elem: &T) -> bool {
        self.start <= *elem && *elem < self.end
    }
}

impl<T: ValidCheck + Ord> ValidCheck for ExclusiveRange<T> {
    fn valid(&self) -> bool {
        self.start.valid() && self.end.valid() && self.start < self.end
    }
}

// Set bits <start> to <end> (inclusive), clearing the others, and return the result.
pub fn bit_range<T: Num + Shl<u32, Output=T>>(start: u32, end: u32) -> T {
    ((T::one() << (end - start + 1)) - T::one()) << start
}
