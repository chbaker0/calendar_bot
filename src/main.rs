extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod tg;

use std::string::String;

use futures::future;
use futures::Future;

fn main() {
    let token = std::env::var(TOKEN_ENV_VAR).expect("Missing TG_BOT_TOKEN env var");
    let http_client = reqwest::Client::new();
    let tg_client = tg::Client::new(token, |url| synchronous_send(&http_client, url));
    let me = tg_client.get_me().wait().unwrap();
    println!("{:?}", me);
}

/// Adapter for using reqwest with futures.
fn synchronous_send(
    client: &reqwest::Client,
    url: &str,
) -> impl Future<Item = String, Error = reqwest::Error> {
    future::result::<String, reqwest::Error>(
        client.get(url).send().and_then(|mut resp| resp.text()),
    )
}

const TOKEN_ENV_VAR: &'static str = "TG_BOT_TOKEN";
