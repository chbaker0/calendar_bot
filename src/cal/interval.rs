use chrono::DateTime;
use chrono::Utc;

use std::cmp::max;
use std::cmp::min;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub struct Interval<T> {
    start: T,
    end: T,
}

impl<T> Interval<T>
where
    T: Eq,
    T: Ord,
    T: Copy,
    T: std::fmt::Debug,
{
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Interval<DateTime<Utc>> {
        assert!(start < end);

        Interval {
            start: start,
            end: end,
        }
    }

    pub fn intersection(&self, other: &Interval<T>) -> Option<Interval<T>> {
        let end = min(self.end, other.end);
        let start = max(self.start, other.start);

        if start < end {
            Some(Interval {
                start: start,
                end: end,
            })
        } else {
            None
        }
    }

    pub fn difference(&self, other: &Interval<T>) -> impl Iterator<Item = Interval<T>> {
        struct Difference<T> {
            interval: Interval<T>,
            intersection: Option<Interval<T>>,
        }

        impl<T> Iterator for Difference<T>
        where
            T: Eq,
            T: Ord,
            T: Copy,
            T: std::fmt::Debug,
        {
            type Item = Interval<T>;

            fn next(&mut self) -> Option<Interval<T>> {
                let i = match self.intersection {
                    Some(x) => x,
                    None => return Some(self.interval),
                };
                let start;
                let end;

                if self.interval.start < i.start {
                    start = self.interval.start;
                    end = i.start;
                    self.interval.start = i.start;
                } else if self.interval.end > i.end {
                    start = i.end;
                    end = self.interval.end;
                    self.interval.end = i.end;
                } else {
                    return None;
                }

                Some(Interval {
                    start: start,
                    end: end,
                })
            }
        }

        Difference {
            interval: (*self).clone(),
            intersection: self.intersection(other),
        }
        .into_iter()
    }
}

impl<T> Ord for Interval<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Interval<T>) -> Ordering {
        let comp = self.start.cmp(&other.start);
        if comp == Ordering::Equal {
            self.end.cmp(&other.end)
        } else {
            comp
        }
    }
}

impl<T> PartialOrd for Interval<T>
where
    T: Ord,
{
    fn partial_cmp(&self, other: &Interval<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Eq for Interval<T> where T: Eq {}

impl<T> PartialEq for Interval<T>
where
    T: Eq,
{
    fn eq(&self, other: &Interval<T>) -> bool {
        self.start == other.start && self.end == other.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn difference_no_overlap() {
        let interval_a = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
        };
        let interval_b = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(3, 0, 0),
        };
        assert_eq!(
            interval_a.difference(&interval_b).next().unwrap(),
            interval_a
        );
    }

    #[test]
    fn difference_contained() {
        let interval_a = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(3, 0, 0),
        };
        let interval_b = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
        };
        let interval_c = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(4, 0, 0),
        };

        let mut int_a = interval_a.difference(&interval_b);
        assert!(
            int_a.next().unwrap()
                == Interval {
                    start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
                    end: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
                }
        );
        assert!(
            int_a.next().unwrap()
                == Interval {
                    start: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
                    end: Utc.ymd(2019, 1, 1).and_hms(3, 0, 0),
                }
        );
        assert!(int_a.next().is_none());
        let mut int_a = interval_a.difference(&interval_c);
        assert!(
            int_a.next().unwrap()
                == Interval {
                    start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
                    end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
                }
        );
        assert!(int_a.next().is_none());
    }

    #[test]
    fn no_intersection() {
        let interval_a = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
        };
        let interval_b = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(3, 0, 0),
        };
        let interval_c = Interval {
            start: Utc.ymd(2019, 2, 1).and_hms(1, 0, 0),
            end: Utc.ymd(2019, 2, 1).and_hms(2, 0, 0),
        };
        assert!(interval_a.intersection(&interval_b).is_none());
        assert!(interval_a.intersection(&interval_c).is_none());
    }

    #[test]
    fn test_intersection() {
        let interval_a = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
        };
        let interval_b = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(3, 0, 0),
        };
        let interval_c = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 30, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(1, 30, 0),
        };
        let interval_d = Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
        };
        assert_eq!(
            interval_a.intersection(&interval_b).unwrap(),
            Interval {
                start: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
                end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
            }
        );
        assert_eq!(
            interval_a.intersection(&interval_c).unwrap(),
            Interval {
                start: Utc.ymd(2019, 1, 1).and_hms(0, 30, 0),
                end: Utc.ymd(2019, 1, 1).and_hms(1, 30, 0),
            }
        );
        assert_eq!(
            interval_a.intersection(&interval_d).unwrap(),
            Interval {
                start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
                end: Utc.ymd(2019, 1, 1).and_hms(2, 0, 0),
            }
        );
    }
}
