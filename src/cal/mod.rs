pub mod interval;

use crate::cal::interval::Interval;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Range;

use std::iter::Iterator;

use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cal {
    events: BTreeSet<CmpEvent>,
}

#[allow(dead_code)]
impl Cal {
    pub fn new() -> Cal {
        Cal {
            events: BTreeSet::new(),
        }
    }

    /// Gets all events within a range
    pub fn events_in<'a>(
        &'a self,
        range: Range<DateTime<Utc>>,
    ) -> impl Iterator<Item = &Event> + 'a {
        self.events_in_cmp(range).map(|x| &x.event)
    }

    fn events_in_cmp(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = &CmpEvent> {
        let event_range = Range {
            start: CmpEvent::from_date(range.start),
            end: CmpEvent::from_date(range.end),
        };
        self.events.range(event_range)
    }

    fn free_time_in<'a>(
        &'a self,
        range: Range<DateTime<Utc>>,
    ) -> impl Iterator<Item = Interval<DateTime<Utc>>> + 'a {
        struct FreeTime<T> {
            busy_times: T,
            interval: Interval<DateTime<Utc>>,
        }

        impl<T> Iterator for FreeTime<T>
        where
            T: Iterator<Item = Interval<DateTime<Utc>>>,
        {
            type Item = Interval<DateTime<Utc>>;

            fn next(&mut self) -> Option<Interval<DateTime<Utc>>> {
                if self.interval.start >= self.interval.end {
                    return None;
                }

                let busy_time = match self.busy_times.next() {
                    Some(x) => x,
                    None => {
                        let ret = self.interval;
                        self.interval.start = self.interval.end;
                        return Some(ret);
                    }
                };

                let ret = Some(Interval {
                    start: self.interval.start,
                    end: busy_time.start,
                });
                self.interval.start = busy_time.end;

                ret
            }
        }

        FreeTime {
            interval: Interval {
                start: range.start,
                end: range.end,
            },
            busy_times: self.events_in(range).map(|x| x.interval),
        }
        .into_iter()
    }

    pub fn add_event(&mut self, event: Event) -> bool {
        self.events.insert(CmpEvent::from_event(event))
    }

    pub fn find_time<T>(
        &self,
        proposed_times: &mut T,
        range: Range<DateTime<Utc>>,
        duration: Duration,
    ) -> Option<Interval<DateTime<Utc>>>
    where
        T: Iterator<Item = Interval<DateTime<Utc>>>,
    {
        let mut proposed_time = proposed_times.next()?;
        let mut free_times = self.free_time_in(range);
        let mut free_time = free_times.next()?;

        loop {
            match proposed_time.intersection(&free_time) {
                Some(x) => {
                    if x.end.signed_duration_since(x.start) >= duration {
                        return Some(x);
                    } else {
                        match proposed_time.cmp(&free_time) {
                            Ordering::Less => proposed_time = proposed_times.next()?,
                            Ordering::Greater => free_time = free_times.next()?,
                            Ordering::Equal => {
                                free_time = free_times.next()?;
                                proposed_time = proposed_times.next()?;
                            }
                        }
                    }
                }
                None => {
                    free_time = free_times.next()?;
                    proposed_time = proposed_times.next()?;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Event {
    pub organizer: String,
    pub description: String,
    pub interval: Interval<DateTime<Utc>>,
}

#[allow(dead_code)]
impl Event {
    pub fn overlap(&self, other: &Event) -> Option<Interval<DateTime<Utc>>> {
        self.interval.intersection(&other.interval)
    }

    fn from_datetime_duration(start: DateTime<Utc>, hours: i64) -> Event {
        Event {
            organizer: "".to_string(),
            description: "".to_string(),
            interval: Interval {
                start: start,
                end: start + Duration::hours(hours),
            },
        }
    }

    fn from_date(start: DateTime<Utc>) -> Event {
        Event::from_datetime_duration(start, 0)
    }

    /// There is really no event default, this is a convieneince method, hence why its private
    fn event_default() -> Event {
        use chrono::TimeZone;
        Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(0, 0, 0), 1)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CmpEvent {
    event: Event,
}

#[allow(dead_code)]
impl CmpEvent {
    fn overlap(&self, other: &CmpEvent) -> Option<Interval<DateTime<Utc>>> {
        self.event.overlap(&other.event)
    }

    fn from_date(date: DateTime<Utc>) -> CmpEvent {
        CmpEvent::from_event(Event::from_date(date))
    }

    fn from_interval(interval: Interval<DateTime<Utc>>) -> CmpEvent {
        let event = Event {
            interval: interval,
            ..Event::event_default()
        };

        CmpEvent { event: event }
    }

    fn from_event(event: Event) -> CmpEvent {
        CmpEvent { event: event }
    }
}

impl Ord for CmpEvent {
    fn cmp(&self, other: &CmpEvent) -> Ordering {
        self.event.interval.cmp(&other.event.interval)
    }
}

impl PartialOrd for CmpEvent {
    fn partial_cmp(&self, other: &CmpEvent) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CmpEvent {}

impl PartialEq for CmpEvent {
    fn eq(&self, other: &CmpEvent) -> bool {
        self.event.interval.eq(&other.event.interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::vec::Vec;

    #[test]
    fn test_find_intervals() {
        let event_a = Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(12, 0, 0), 1);
        let event_b = Event::from_datetime_duration(Utc.ymd(2019, 2, 1).and_hms(12, 0, 0), 1);
        let mut free_times = Vec::new();
        free_times.push(Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(12, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(13, 0, 0),
        });
        free_times.push(Interval {
            start: Utc.ymd(2019, 1, 1).and_hms(13, 0, 0),
            end: Utc.ymd(2019, 1, 1).and_hms(14, 0, 0),
        });

        let mut cal = Cal::new();
        cal.add_event(event_a.clone());
        cal.add_event(event_b.clone());

        let range = Range {
            start: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: Utc.ymd(2019, 1, 2).and_hms(15, 0, 0),
        };
        assert_eq!(
            free_times[1].clone(),
            cal.find_time(free_times.into_iter().by_ref(), range, Duration::hours(1))
                .unwrap()
        );
    }

    #[test]
    fn event_ordering() {
        let event_a = CmpEvent::from_event(Event {
            organizer: "zzzz".to_string(),
            description: "zzzz".to_string(),
            interval: Interval {
                start: Utc.ymd(2019, 01, 01).and_hms(0, 0, 0),
                end: Utc.ymd(2019, 01, 01).and_hms(1, 0, 0),
            },
        });
        let event_b = CmpEvent::from_event(Event {
            organizer: "aaaa".to_string(),
            description: "aaaa".to_string(),
            interval: Interval {
                start: Utc.ymd(2020, 12, 31).and_hms(0, 0, 0),
                end: Utc.ymd(2020, 12, 31).and_hms(0, 0, 0),
            },
        });
        let event_c = CmpEvent::from_event(Event {
            organizer: "aaaa".to_string(),
            description: "aaaa".to_string(),
            interval: Interval {
                start: Utc.ymd(2019, 01, 01).and_hms(0, 0, 0),
                end: Utc.ymd(2019, 01, 01).and_hms(2, 0, 0),
            },
        });

        assert_eq!(event_a.cmp(&event_b), Ordering::Less);
        assert_eq!(event_a.cmp(&event_c), Ordering::Less);
    }

    #[test]
    fn test_overlap() {
        let event_a = Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(0, 0, 0), 2);
        let event_b = Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(1, 0, 0), 1);
        let event_c = Event::from_datetime_duration(Utc.ymd(2020, 12, 31).and_hms(0, 0, 0), 1);
        let event_d = Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(0, 30, 0), 1);
        let event_e = Event::from_datetime_duration(Utc.ymd(2019, 1, 1).and_hms(0, 0, 0), 2);
        assert!(event_a.overlap(&event_b).is_some());
        assert!(event_a.overlap(&event_c).is_none());
        assert!(event_a.overlap(&event_d).is_some());
        assert!(event_a.overlap(&event_e).is_some());
    }

    #[test]
    fn test_event_in() {
        let event = Event::from_date(Utc.ymd(2019, 1, 1).and_hms(12, 0, 0));

        let mut cal = Cal::new();
        cal.add_event(event.clone());
        let e = cal
            .events_in(
                event.interval.start - Duration::days(1)..event.interval.end + Duration::days(1),
            )
            .next()
            .unwrap();

        assert_eq!(*e, event);
    }

    #[test]
    fn test_free_time_in() {
        let event = Event::from_date(Utc.ymd(2019, 1, 1).and_hms(0, 0, 0));
        let mut cal = Cal::new();
        cal.add_event(event.clone());

        let free_times = cal.free_time_in(
            event.interval.start - Duration::days(1)..event.interval.end + Duration::days(1),
        );

        assert_eq!(free_times.count(), 2);

        let free_times = cal.free_time_in(event.interval.start..event.interval.end);
        assert_eq!(free_times.count(), 0);
    }
}
