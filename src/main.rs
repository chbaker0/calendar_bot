extern crate chrono;
extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod cal;
mod tg;

use std::string::String;

use futures::future;
use futures::Future;
use futures::Stream;

fn main() {
    let token = std::env::var(TOKEN_ENV_VAR).expect("Missing TG_BOT_TOKEN env var");
    let http_client = reqwest::Client::new();
    let tg_client = tg::Client::new(token, |url, body| synchronous_send(&http_client, url, body));
    let me = tg_client.get_me().wait().unwrap().unwrap();
    println!("{:?}", me);

    tg::update_stream(&tg_client, 10)
        .filter_map(|update| update.message)
        .for_each(|recv_msg| {
            let (command, body) =
                parse_command(recv_msg.text.as_ref().map(String::as_str).unwrap_or(""));

            if command == "echo" && !body.is_empty() {
                let send_msg = tg::SendMessage {
                    chat_id: recv_msg.chat.id,
                    text: String::from(body),
                };
                future::Either::A(tg_client.send_message(send_msg).map(|r| {
                    r.unwrap();
                    ()
                }))
            } else {
                future::Either::B(future::ok(()))
            }
        })
        .wait()
        .unwrap();
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

const TOKEN_ENV_VAR: &'static str = "TG_BOT_TOKEN";

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
}
