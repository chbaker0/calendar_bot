use std::collections::BTreeSet;
use std::ops::Range;

use chrono::offset::Utc;
use chrono::DateTime;
use chrono::Duration;

use std::cmp::Ordering;

pub struct Cal {
    events: BTreeSet<Event>,
}

#[allow(dead_code)]
impl Cal {
    pub fn new() -> Cal {
        Cal {
            events: BTreeSet::new(),
        }
    }

    pub fn events_in(&self, range: Range<DateTime<Utc>>) -> impl Iterator<Item = &Event> {
        let event_range = Range {
            start: Event::from_date(range.start),
            end: Event::from_date(range.end),
        };
        self.events.range(event_range)
    }

    pub fn add_event(&mut self, event: Event) -> bool {
        self.events.insert(event)
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub organizer: String,
    pub description: String,
    pub date: DateTime<Utc>,
    pub duration: Duration,
}

impl Event {
    fn from_date(date: DateTime<Utc>) -> Event {
        Event {
            organizer: "".to_string(),
            description: "".to_string(),
            date: date,
            duration: Duration::zero(),
        }
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        self.date.cmp(&other.date)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Event {}

impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        self.date == other.date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_ordering() {
        let event_a = Event::new(
            "zzzz".to_string(),
            "zzzz".to_string(),
            "2019-01-01T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            Duration::zero(),
        );
        let event_b = Event::new(
            "aaaa".to_string(),
            "aaaa".to_string(),
            "2020-12-31T00:00:00Z"
                .to_string()
                .parse::<DateTime<Utc>>()
                .unwrap(),
            Duration::zero(),
        );

        assert_eq!(event_a.cmp(&event_b), Ordering::Less)
    }

    #[test]
    fn test_event_in() {
        let date = "2019-01-01T12:00:00Z"
            .to_string()
            .parse::<DateTime<Utc>>()
            .unwrap();
        let event = Event::new(
            "test".to_string(),
            "test".to_string(),
            date,
            Duration::zero(),
        );

        let mut cal = Cal::new();
        cal.add_event(event.clone());
        let events = cal.events_in(date - Duration::days(1)..date + Duration::days(1));

        for e in events {
            assert_eq!(e, &event);
        }
    }
}
