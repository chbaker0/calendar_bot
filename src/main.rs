extern crate bincode;
extern crate chrono;
extern crate futures;
extern crate itertools;
extern crate lazy_static;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod cal;
mod tg;

use std::mem::drop;
use std::ops::Range;
use std::string::String;

use chrono::prelude::*;
use futures::future;
use futures::Future;
use futures::Stream;
use lazy_static::lazy_static;

use crate::cal::interval::Interval;

fn main() {
    let token = std::env::var(TOKEN_ENV_VAR).expect("Missing TG_BOT_TOKEN env var");
    let http_client = reqwest::Client::new();
    let tg_client = tg::Client::new(token, |url, body| synchronous_send(&http_client, url, body));
    let me = tg_client.get_me().wait().unwrap().unwrap();
    println!("{:?}", me);

    let mut cal =
        cal::PersistentCal::open_or_create(CAL_FILE).expect("Couldn't open calendar file");

    let send_response = |chat_id, text| {
        let msg = tg::SendMessage {
            chat_id: chat_id,
            text: text,
        };

        tg_client.send_message(msg).map(Result::unwrap).map(drop)
    };

    tg::update_stream(&tg_client, 10)
        .filter_map(|update| update.message)
        .filter_map(|recv_msg| {
            let command = parse_command(recv_msg.text.as_ref().map(String::as_str).unwrap_or(""));
            if command.is_err() {
                return Some(send_response(
                    recv_msg.chat.id,
                    command.unwrap_err().to_string(),
                ));
            }

            let command = command.unwrap();
            Some(match command {
                Command::AddEvent(event) => {
                    // TODO: handle errors here
                    cal.add_event(event).unwrap();
                    send_response(recv_msg.chat.id, "Added event successfully".to_string())
                }
                Command::GetEvents(date_window) => {
                    let interval = date_window.resolve(&Utc::now().with_timezone(&*TIMEZONE));
                    if interval.is_none() {
                        return Some(send_response(
                            recv_msg.chat.id,
                            "Invalid date window specified".to_string(),
                        ));
                    }

                    let interval = interval.unwrap();
                    // TODO: change Cal to take intervals directly
                    let range = Range {
                        start: interval.start.with_timezone(&Utc),
                        end: interval.end.with_timezone(&Utc),
                    };
                    let mut response = itertools::join(
                        cal.get_cal().events_in(range).map(pretty_print_event),
                        "\n\n",
                    );
                    if response == "" {
                        response = String::from("No events today");
                    }
                    send_response(recv_msg.chat.id, response)
                }
            })
        })
        .map(Future::into_stream)
        .flatten()
        .wait()
        .map(Result::unwrap)
        .for_each(|_| ());
}

/// A user command sent to this bot.
#[derive(Eq, Debug, PartialEq)]
enum Command {
    /// Add the contained event to the calendar.
    AddEvent(cal::Event),
    /// Get events in some window described by the `DateWindow`.
    GetEvents(DateWindow),
}

/// Describes which dates and times the caller is interested for
/// queries.
#[derive(Eq, Debug, PartialEq)]
enum DateWindow {
    /// Events happening today.
    Today,
}

impl DateWindow {
    fn resolve<Tz: TimeZone>(&self, now: &DateTime<Tz>) -> Option<Interval<DateTime<Tz>>> {
        match self {
            DateWindow::Today => Some(Interval {
                start: now.date().and_hms(0, 0, 0),
                end: now.date().succ_opt()?.and_hms(0, 0, 0),
            }),
        }
    }
}

fn parse_command(text: &str) -> Result<Command, &'static str> {
    let (cmd_name, body) = split_command(text);
    if cmd_name == "add_event" {
        Ok(Command::AddEvent(parse_event(body)?))
    } else if cmd_name == "today" {
        Ok(Command::GetEvents(DateWindow::Today))
    } else {
        Err("Unsupported command")
    }
}

/// Given the body of a message, split out the command from the rest
/// of the message.
fn split_command(text: &str) -> (&str, &str) {
    let mut chars = text.chars();
    if let Some(_) = chars.find(|c| c == &'/') {
        let chars_after_slash = chars.clone();
        let text_after_slash = chars_after_slash.as_str();

        let maybe_at_ndx = chars_after_slash.clone().position(|c| c == '@');
        let maybe_cmd_end = chars_after_slash.clone().position(|c| c == ' ');

        let (mut command, rest) = if let Some(cmd_end) = maybe_cmd_end {
            (
                &text_after_slash[0..cmd_end],
                text_after_slash.get(cmd_end + 1..).unwrap_or(""),
            )
        } else {
            (text_after_slash, "")
        };

        if let Some(at_ndx) = maybe_at_ndx {
            if at_ndx < command.chars().count() {
                command = &command[0..at_ndx];
            }
        }

        (command, rest)
    } else {
        ("", text)
    }
}

/// Parses out a date, time, duration, and event description from the
/// message body.
fn parse_event(text: &str) -> Result<cal::Event, &'static str> {
    use chrono::Duration;

    const ERROR_MESSAGE: &'static str = "wrong";
    let mut pieces = text.splitn(3, char::is_whitespace);
    let date_text = pieces.next().ok_or(ERROR_MESSAGE)?;
    let time_text = pieces.next().ok_or(ERROR_MESSAGE)?;
    let description = pieces.next().unwrap_or("");

    let date = NaiveDate::parse_from_str(date_text, "%m/%d/%Y").map_err(|_| ERROR_MESSAGE)?;
    let time = NaiveTime::parse_from_str(time_text, "%H:%M:%S").map_err(|_| ERROR_MESSAGE)?;

    let tz_datetime = TIMEZONE
        .from_local_datetime(&NaiveDateTime::new(date, time))
        .earliest()
        .ok_or(ERROR_MESSAGE)?;
    let utc_datetime = Utc.from_utc_datetime(&tz_datetime.naive_utc());

    Ok(cal::Event {
        organizer: String::new(),
        description: String::from(description),
        interval: Interval {
            start: utc_datetime,
            end: utc_datetime + Duration::hours(1),
        },
    })
}

fn pretty_print_event(event: &cal::Event) -> String {
    let mut result = String::new();
    result.push_str("On ");
    result.push_str(
        &event
            .interval
            .start
            .with_timezone(&*TIMEZONE)
            .format("%-m/%-d/%Y at %H:%M:%S")
            .to_string(),
    );
    result.push_str(":\n");
    result.push_str(&event.description);
    result
}

/// Adapter for using reqwest with futures.
fn synchronous_send(
    client: &reqwest::Client,
    url: String,
    body: Option<String>,
) -> impl Future<Item = String, Error = reqwest::Error> {
    let mut req = client.get(&url);
    if let Some(b) = body {
        req = req
            .body(b)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
    }
    future::result::<String, reqwest::Error>(req.send().and_then(|mut resp| resp.text()))
}

const CAL_FILE: &'static str = "cal";

const TOKEN_ENV_VAR: &'static str = "TG_BOT_TOKEN";

lazy_static! {
    static ref TIMEZONE: chrono::offset::FixedOffset = chrono::offset::FixedOffset::west(7 * 3600);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_window_resolve_today() {
        let now = TIMEZONE.ymd(2019, 1, 1).and_hms(13, 10, 30);
        let expected = Interval {
            start: TIMEZONE.ymd(2019, 1, 1).and_hms(0, 0, 0),
            end: TIMEZONE.ymd(2019, 1, 2).and_hms(0, 0, 0),
        };
        assert_eq!(DateWindow::Today.resolve(&now).unwrap(), expected);
    }

    #[test]
    fn parse_command_add_event() {
        assert!(parse_command("/add_event@bot").is_err());
        let parsed = parse_command("/add_event@bot 01/01/2019 1:00:00 test description").unwrap();
        if let Command::AddEvent(event) = parsed {
            assert_eq!(event.description, "test description");
            assert_eq!(
                event.interval.start,
                TIMEZONE
                    .ymd(2019, 1, 1)
                    .and_hms(1, 0, 0)
                    .with_timezone(&Utc)
            );
            assert_eq!(
                event.interval.end,
                TIMEZONE
                    .ymd(2019, 1, 1)
                    .and_hms(2, 0, 0)
                    .with_timezone(&Utc)
            )
        } else {
            panic!("Returned value is not correct variant: {:?}", parsed);
        }
    }

    #[test]
    fn parse_command_today() {
        assert_eq!(
            parse_command("/today@bot").unwrap(),
            Command::GetEvents(DateWindow::Today)
        );
    }

    #[test]
    fn parse_command_invalid() {
        assert!(parse_command("/trombloni@bot").is_err());
    }

    #[test]
    fn split_command_tests() {
        assert_eq!(split_command("/foo"), ("foo", ""));
        assert_eq!(split_command("/foo body test"), ("foo", "body test"));
        assert_eq!(
            split_command("/foo@bar_bot body test"),
            ("foo", "body test")
        );
        assert_eq!(split_command("  /foo"), ("foo", ""));
        assert_eq!(split_command("/"), ("", ""));
        assert_eq!(split_command("/@"), ("", ""));
        assert_eq!(split_command("help me"), ("", "help me"));
        assert_eq!(split_command(""), ("", ""));
    }

    #[test]
    fn parse_event_correct_datetime() {
        let body = "1/15/2024 7:53:29 hello world";
        let event = parse_event(body).unwrap();
        assert_eq!(
            event.interval.start,
            TIMEZONE.ymd(2024, 1, 15).and_hms(7, 53, 29)
        );
    }

    #[test]
    fn parse_event_description() {
        let body = "1/1/1 1:1:1 god is dead";
        let event = parse_event(body).unwrap();
        assert_eq!(event.description, "god is dead");
    }

    #[test]
    fn parse_event_no_description() {
        let body = "1/1/1 1:1:1";
        let event = parse_event(body).unwrap();
        assert_eq!(event.description, "");
    }

    #[test]
    fn parse_event_errors() {
        assert!(parse_event("1/1/ 1:1:1").is_err());
        assert!(parse_event("1/1/1 1:67:1").is_err());
        assert!(parse_event("1/1/11:1:1").is_err());
        assert!(parse_event("1/1/1 i forgot the time").is_err());
    }

    #[test]
    fn pretty_print_event_test() {
        let event = cal::Event {
            organizer: String::from(""),
            description: String::from("test description"),
            interval: Interval {
                start: TIMEZONE
                    .ymd(2000, 1, 15)
                    .and_hms(13, 1, 2)
                    .with_timezone(&Utc),
                end: TIMEZONE
                    .ymd(2000, 1, 15)
                    .and_hms(13, 1, 2)
                    .with_timezone(&Utc),
            },
        };
        assert_eq!(
            pretty_print_event(&event),
            String::from("On 1/15/2000 at 13:01:02:\ntest description")
        );
    }
}
