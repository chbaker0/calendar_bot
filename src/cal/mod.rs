use std::collections::BTreeSet;
use std::collections::VecDeque;

use std::ops::Range;

use std::iter::Iterator;

use chrono::Date;
use chrono::DateTime;
use chrono::Duration;
use chrono::NaiveTime;
use chrono::Utc;

use std::cmp::max;
use std::cmp::min;
use std::cmp::Ordering;

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

    /// Gets all free time within a range.
    pub fn free_time_in(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = Event> {
        self.free_time_in_cmp(range).map(|x| x.event)
    }

    fn events_in_cmp(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = &CmpEvent> {
        let event_range = Range {
            start: CmpEvent::from_date(range.start),
            end: CmpEvent::from_date(range.end),
        };
        self.events.range(event_range)
    }

    fn free_time_in_cmp(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = CmpEvent> {
        let busy_times = self.events_in(range.clone());
        let mut vector = VecDeque::new();
        let mut start = range.start;

        for event in busy_times {
            vector.push_back(CmpEvent::from_event(Event {
                date: start,
                duration: event.date.signed_duration_since(start),
                ..Event::event_default()
            }));
            start = event.date + event.duration;
        }
        vector.push_back(CmpEvent::from_event(Event {
            date: start,
            duration: range.end.signed_duration_since(start),
            ..Event::event_default()
        }));
        vector
            .into_iter()
            .filter(|x| x.event.duration > Duration::zero())
    }

    pub fn add_event(&mut self, event: Event) -> bool {
        self.events.insert(CmpEvent::from_event(event))
    }

    pub fn find_time(
        &self,
        days: Range<Date<Utc>>,
        times: Range<NaiveTime>,
        duration: Duration,
    ) -> Option<DateTime<Utc>> {
        let mut proposed_times = Cal::get_proposed_times(&days, &times);
        let mut proposed_time = proposed_times.next()?;
        let mut free_times = self.free_time_in_cmp(Range {
            start: days.start.and_time(times.start).unwrap(),
            end: days.end.and_time(times.end).unwrap(),
        });
        let mut free_time = free_times.next()?;

        loop {
            match proposed_time.overlap(&free_time) {
                Some(x) => {
                    if x.event.duration >= duration {
                        return Some(x.event.date);
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

    fn get_proposed_times(
        days: &Range<Date<Utc>>,
        times: &Range<NaiveTime>,
    ) -> impl Iterator<Item = CmpEvent> {
        let mut vector: VecDeque<CmpEvent> = VecDeque::new();
        let mut day = days.start;

        while day <= days.end {
            // This unwrap will panic if an invalid date time in constructed
            vector.push_back(CmpEvent::from_event(Event {
                date: day.and_time(times.start).unwrap(),
                duration: times.end.signed_duration_since(times.start),
                ..Event::event_default()
            }));
            day = day.succ();
        }

        vector.into_iter()
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Event {
    pub organizer: String,
    pub description: String,
    pub date: DateTime<Utc>,
    pub duration: Duration,
}

impl Event {
    pub fn overlap(&self, other: &Event) -> Option<Event> {
        let start = max(self.date, other.date);
        let duration = min(self.date + self.duration, other.date + other.duration)
            .signed_duration_since(start);

        if duration > Duration::zero() {
            Some(Event {
                date: start,
                duration: duration,
                ..Event::event_default()
            })
        } else {
            None
        }
    }

    /// There is really no event default, this is a convieneince method, hence why its private
    fn event_default() -> Event {
        Event {
            organizer: "".to_string(),
            description: "".to_string(),
            date: chrono::Utc::now(),
            duration: Duration::zero(),
        }
    }
}

#[derive(Clone, Debug)]
struct CmpEvent {
    event: Event,
}

impl CmpEvent {
    fn overlap(&self, other: &CmpEvent) -> Option<CmpEvent> {
        self.event
            .overlap(&other.event)
            .map(|x| CmpEvent::from_event(x))
    }

    fn from_date(date: DateTime<Utc>) -> CmpEvent {
        let event = Event {
            date: date,
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
        let comp = self.event.date.cmp(&other.event.date);
        if comp == Ordering::Equal {
            self.event.duration.cmp(&other.event.duration)
        } else {
            comp
        }
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
        self.event.date == other.event.date && self.event.duration == other.event.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_find_dates() {
        let event_a = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(12, 0, 0),
            duration: Duration::hours(1),
            ..Event::event_default()
        };
        let event_b = Event {
            date: Utc.ymd(2019, 1, 2).and_hms(12, 0, 0),
            duration: Duration::hours(1),
            ..Event::event_default()
        };

        let mut cal = Cal::new();
        cal.add_event(event_a.clone());
        cal.add_event(event_b.clone());

        let date_range = Range {
            start: Utc.ymd(2019, 1, 1),
            end: Utc.ymd(2019, 1, 2),
        };
        let time_range = Range {
            start: NaiveTime::from_hms(12, 0, 0),
            end: NaiveTime::from_hms(14, 0, 0),
        };
        let free_event = Event {
            date: cal
                .find_time(date_range, time_range, Duration::hours(1))
                .unwrap(),
            duration: Duration::hours(1),
            ..Event::event_default()
        };

        println!("{:?}", free_event);

        assert!(free_event.overlap(&event_a).is_none());
        assert!(free_event.overlap(&event_b).is_none());
    }

    #[test]
    fn event_ordering() {
        let event_a = CmpEvent::from_event(Event {
            organizer: "zzzz".to_string(),
            description: "zzzz".to_string(),
            date: Utc.ymd(2019, 01, 01).and_hms(0, 0, 0),
            duration: Duration::hours(1),
        });
        let event_b = CmpEvent::from_event(Event {
            organizer: "aaaa".to_string(),
            description: "aaaa".to_string(),
            date: Utc.ymd(2020, 12, 31).and_hms(0, 0, 0),
            duration: Duration::zero(),
        });
        let event_c = CmpEvent::from_event(Event {
            organizer: "aaaa".to_string(),
            description: "aaaa".to_string(),
            date: Utc.ymd(2019, 01, 01).and_hms(0, 0, 0),
            duration: Duration::hours(2),
        });

        assert_eq!(event_a.cmp(&event_b), Ordering::Less);
        assert_eq!(event_a.cmp(&event_c), Ordering::Less);
    }

    #[test]
    fn test_overlap() {
        let event_a = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            duration: Duration::hours(2),
            ..Event::event_default()
        };
        let event_b = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
            duration: Duration::hours(1),
            ..Event::event_default()
        };
        let event_c = Event {
            date: Utc.ymd(2020, 12, 31).and_hms(0, 0, 0),
            duration: Duration::hours(1),
            ..Event::event_default()
        };
        let event_d = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 30, 0),
            duration: Duration::hours(1),
            ..Event::event_default()
        };
        let event_e = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            duration: Duration::hours(2),
            ..Event::event_default()
        };
        assert!(event_a.overlap(&event_b).is_some());
        assert!(event_a.overlap(&event_c).is_none());
        assert!(event_a.overlap(&event_d).is_some());
        assert!(event_a.overlap(&event_e).is_some());
    }

    #[test]
    fn test_event_in() {
        let event = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(12, 0, 0),
            ..Event::event_default()
        };

        let mut cal = Cal::new();
        cal.add_event(event.clone());
        let e = cal
            .events_in(event.date - Duration::days(1)..event.date + Duration::days(1))
            .next()
            .unwrap();

        assert_eq!(*e, event);
    }

    #[test]
    fn test_free_time_in() {
        let event = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            ..Event::event_default()
        };
        let mut cal = Cal::new();
        cal.add_event(event.clone());

        let free_times =
            cal.free_time_in(event.date - Duration::days(1)..event.date + Duration::days(1));

        let mut num_times = 0;
        for free_time in free_times {
            assert!(free_time.overlap(&event).is_none());
            num_times += 1;
        }
        assert_eq!(num_times, 2);

        let free_times = cal.free_time_in(event.date..event.date + event.duration);
        assert_eq!(free_times.count(), 0);
    }
}
