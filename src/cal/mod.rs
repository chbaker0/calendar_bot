use std::collections::BTreeSet;
use std::ops::Range;

use chrono::offset::Utc;
use chrono::DateTime;
use chrono::Duration;

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
            organizer: "".to_string(),
            description: "".to_string(),
            date: date,
            duration: Duration::zero(),
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

    #[test]
    fn event_ordering() {
        let event_a = Event {
            organizer: "zzzz".to_string(),
            description: "zzzz".to_string(),
            date: "2019-01-01T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::zero(),
        };
        let event_a = CmpEvent::from_event(event_a);
        let event_b = Event {
            organizer: "aaaa".to_string(),
            description: "aaaa".to_string(),
            date: "2020-12-31T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::zero(),
        };
        let event_b = CmpEvent::from_event(event_b);

        assert_eq!(event_a.cmp(&event_b), Ordering::Less)
    }

    #[test]
    fn test_overlap() {
        let event_a = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: "2019-01-01T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::hours(2),
        };
        let event_b = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: "2019-01-01T01:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::hours(1),
        };
        let event_c = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: "2020-12-31T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::hours(1),
        };
        let event_d = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: "2019-01-01T00:30:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::hours(1),
        };
        let event_e = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: "2019-01-01T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            duration: Duration::hours(2),
        };
        assert!(event_a.overlap(&event_b));
        assert!(!event_a.overlap(&event_c));
        assert!(event_a.overlap(&event_d));
        assert!(event_a.overlap(&event_e));
    }

    #[test]
    fn test_event_in() {
        let date = "2019-01-01T12:00:00Z"
            .to_string()
            .parse::<DateTime<Utc>>()
            .unwrap();
        let event = Event {
            organizer: "test".to_string(),
            description: "test".to_string(),
            date: date,
            duration: Duration::zero(),
        };

        let mut cal = Cal::new();
        cal.add_event(event.clone());
        let events = cal.events_in(date - Duration::days(1)..date + Duration::days(1));

        for e in events {
            assert_eq!(CmpEvent::from_event(e), CmpEvent::from_event(event.clone()));
        }
    }
}
