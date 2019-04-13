use std::collections::BTreeSet;
use std::ops::Range;

use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;

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

    pub fn events_in(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = Event> + '_ {
        let event_range = Range {
            start: CmpEvent::from_date(range.start),
            end: CmpEvent::from_date(range.end),
        };
        self.events.range(event_range).map(|x| x.event.clone())
    }

    pub fn add_event(&mut self, event: Event) -> bool {
        self.events.insert(CmpEvent::from_event(event))
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub organizer: String,
    pub description: String,
    pub date: DateTime<Utc>,
    pub duration: Duration,
}

impl Default for Event {
    fn default() -> Event {
        Event {
            organizer: "".to_string(),
            description: "".to_string(),
            date: chrono::Utc::now(),
            duration: Duration::zero(),
        }
    }
}

#[allow(dead_code)]
impl Event {
    pub fn overlap(&self, other: &Event) -> bool {
        match self.date.cmp(&other.date) {
            Ordering::Less => self.date + self.duration >= other.date,
            Ordering::Greater => other.date + other.duration >= self.date,
            Ordering::Equal => true,
        }
    }
}

#[derive(Clone, Debug)]
struct CmpEvent {
    event: Event,
}

impl CmpEvent {
    fn from_date(date: DateTime<Utc>) -> CmpEvent {
        let event = Event {
            date: date,
            ..Default::default()
        };

        CmpEvent { event: event }
    }

    fn from_event(event: Event) -> CmpEvent {
        CmpEvent { event: event }
    }
}

impl Ord for CmpEvent {
    fn cmp(&self, other: &CmpEvent) -> Ordering {
        self.event.date.cmp(&other.event.date)
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

        assert_eq!(event_a.cmp(&event_b), Ordering::Less)
    }

    #[test]
    fn test_overlap() {
        let event_a = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            duration: Duration::hours(2),
            ..Default::default()
        };
        let event_b = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(1, 0, 0),
            duration: Duration::hours(1),
            ..Default::default()
        };
        let event_c = Event {
            date: Utc.ymd(2020, 12, 31).and_hms(0, 0, 0),
            duration: Duration::hours(1),
            ..Default::default()
        };
        let event_d = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 30, 0),
            duration: Duration::hours(1),
            ..Default::default()
        };
        let event_e = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
            duration: Duration::hours(2),
            ..Default::default()
        };
        assert!(event_a.overlap(&event_b));
        assert!(!event_a.overlap(&event_c));
        assert!(event_a.overlap(&event_d));
        assert!(event_a.overlap(&event_e));
    }

    #[test]
    fn test_event_in() {
        let event = Event {
            date: Utc.ymd(2019, 1, 1).and_hms(12, 0, 0),
            ..Default::default()
        };

        let mut cal = Cal::new();
        cal.add_event(event.clone());
        let events = cal.events_in(event.date - Duration::days(1)..event.date + Duration::days(1));

        for e in events {
            assert_eq!(CmpEvent::from_event(e), CmpEvent::from_event(event.clone()));
        }
    }
}
