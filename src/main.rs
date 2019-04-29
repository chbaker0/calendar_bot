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

    let mut cal = load_cal();

    tg::update_stream(&tg_client, 10)
        .filter_map(|update| update.message)
        .filter_map(|recv_msg| {
            let (command, body) =
                parse_command(recv_msg.text.as_ref().map(String::as_str).unwrap_or(""));

            if command == "echo" && !body.is_empty() {
                let send_msg = tg::SendMessage {
                    chat_id: recv_msg.chat.id,
                    text: String::from(body),
                };
                Some(
                    tg_client
                        .send_message(send_msg)
                        .map(Result::unwrap)
                        .map(drop),
                )
            } else if command == "add_event" {
                let response = match parse_event(body) {
                    Ok(event) => {
                        cal.add_event(event);
                        save_cal(&cal);
                        String::from("Added event successfully")
                    }
                    Err(err) => String::from(err),
                };

                let send_msg = tg::SendMessage {
                    chat_id: recv_msg.chat.id,
                    text: response,
                };
                Some(
                    tg_client
                        .send_message(send_msg)
                        .map(Result::unwrap)
                        .map(drop),
                )
            } else if command == "today" {
                let today_local = Utc::now().with_timezone(&*TIMEZONE).date();
                let range = Range {
                    start: today_local.and_hms(0, 0, 0).with_timezone(&Utc),
                    end: today_local.and_hms(23, 59, 59).with_timezone(&Utc),
                };
                let mut response =
                    itertools::join(cal.events_in(range).map(pretty_print_event), "\n\n");
                if response == "" {
                    response = String::from("No events today");
                }

                let send_msg = tg::SendMessage {
                    chat_id: recv_msg.chat.id,
                    text: response,
                };
                Some(
                    tg_client
                        .send_message(send_msg)
                        .map(Result::unwrap)
                        .map(drop),
                )
            } else {
                None
            }
        })
        .map(Future::into_stream)
        .flatten()
        .wait()
        .map(Result::unwrap)
        .for_each(|_| ());
}

/// Given the body of a message, parse out the command from the rest
/// of the message.
fn parse_command(text: &str) -> (&str, &str) {
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

fn load_cal() -> cal::Cal {
    match std::fs::File::open(CAL_FILE) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            serde_json::de::from_reader(reader).unwrap()
        }
        Err(_) => cal::Cal::new(),
    }
}

fn save_cal(cal: &cal::Cal) {
    let writer = std::io::BufWriter::new(std::fs::File::create(CAL_FILE).unwrap());
    serde_json::ser::to_writer(writer, cal).unwrap();
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
    fn parse_command_tests() {
        assert_eq!(parse_command("/foo"), ("foo", ""));
        assert_eq!(parse_command("/foo body test"), ("foo", "body test"));
        assert_eq!(
            parse_command("/foo@bar_bot body test"),
            ("foo", "body test")
        );
        assert_eq!(parse_command("  /foo"), ("foo", ""));
        assert_eq!(parse_command("/"), ("", ""));
        assert_eq!(parse_command("/@"), ("", ""));
        assert_eq!(parse_command("help me"), ("", "help me"));
        assert_eq!(parse_command(""), ("", ""));
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
