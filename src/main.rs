extern crate chrono;
extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod cal;
mod tg;

use std::default::Default;
use std::string::String;

use futures::future;
use futures::Future;

fn main() {
    let token = std::env::var(TOKEN_ENV_VAR).expect("Missing TG_BOT_TOKEN env var");
    let http_client = reqwest::Client::new();
    let tg_client = tg::Client::new(token, |url, body| synchronous_send(&http_client, url, body));
    let me = tg_client.get_me().wait().unwrap();
    println!("{:?}", me);
    let updates = tg_client.get_updates(Default::default()).wait().unwrap();
    println!("{:?}", updates);
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
