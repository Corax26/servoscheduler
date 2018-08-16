use num::Num;
use std::ops::Shl;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd + Copy> InclusiveRange<T> {
    pub fn valid(&self) -> bool {
        self.start <= self.end
    }

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

#[derive(Clone, Serialize ,Deserialize, Debug)]
pub struct ExclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd + Copy> ExclusiveRange<T> {
    pub fn valid(&self) -> bool {
        self.start < self.end
    }

    pub fn overlaps(&self, other: &ExclusiveRange<T>) -> bool {
        self.start < other.end && other.start < self.end
    }
}

// Set bits <start> to <end> (inclusive), clearing the others, and return the result.
pub fn bit_range<T: Num + Shl<u32, Output=T>>(start: u32, end: u32) -> T {
    ((T::one() << (end - start + 1)) - T::one()) << start
}
