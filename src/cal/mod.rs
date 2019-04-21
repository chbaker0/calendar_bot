use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Range;

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

    pub fn events_in<'a>(
        &'a self,
        range: Range<DateTime<Utc>>,
    ) -> impl Iterator<Item = &Event> + 'a {
        let event_range = Range {
            start: CmpEvent::from_date(range.start),
            end: CmpEvent::from_date(range.end),
        };
        self.events.range(event_range).map(|x| &x.event)
    }

    pub fn add_event(&mut self, event: Event) -> bool {
        self.events.insert(CmpEvent::from_event(event))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Event {
    pub organizer: String,
    pub description: String,
    pub date: DateTime<Utc>,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
}

mod duration_serde {
    use chrono::Duration;
    use serde::de::Deserializer;
    use serde::de::Error;
    use serde::de::MapAccess;
    use serde::de::SeqAccess;
    use serde::de::Visitor;
    use serde::ser::SerializeStruct;
    use serde::ser::Serializer;

    const STRUCT_NAME: &'static str = "Duration";
    const FIELD_NAME: &'static str = "_";

    pub fn serialize<S>(dur: &Duration, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ss = s.serialize_struct(STRUCT_NAME, 1)?;
        ss.serialize_field(FIELD_NAME, &dur.to_std().unwrap())?;
        ss.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl<'de> Visitor<'de> for DurationVisitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(STRUCT_NAME)
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Duration, V::Error>
            where
                V: SeqAccess<'de>,
            {
                match seq.next_element()? {
                    Some(dur) => Ok(Duration::from_std(dur).unwrap()),
                    None => Err(V::Error::missing_field(FIELD_NAME)),
                }
            }

            fn visit_map<V>(self, mut map: V) -> Result<Duration, V::Error>
            where
                V: MapAccess<'de>,
            {
                while let Some((k, v)) = map.next_entry::<String, _>()? {
                    if k == FIELD_NAME {
                        return Ok(Duration::from_std(v).unwrap());
                    }
                }

                Err(V::Error::missing_field(FIELD_NAME))
            }
        }

        d.deserialize_struct(STRUCT_NAME, &[FIELD_NAME], DurationVisitor)
    }
}

#[allow(dead_code)]
impl Event {
    pub fn overlap(&self, other: &Event) -> bool {
        match self.date.cmp(&other.date) {
            Ordering::Less => self.date + self.duration > other.date,
            Ordering::Greater => other.date + other.duration > self.date,
            Ordering::Equal => true,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CmpEvent {
    event: Event,
}

impl CmpEvent {
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
        assert!(event_a.overlap(&event_b));
        assert!(!event_a.overlap(&event_c));
        assert!(event_a.overlap(&event_d));
        assert!(event_a.overlap(&event_e));
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
}
