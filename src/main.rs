extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod tg;

use std::ops::FnOnce;
use std::string::String;

use futures::future;
use futures::Future;
use serde::de::DeserializeOwned;

const TOKEN_ENV_VAR: &'static str = "TG_BOT_TOKEN";

/// Fires off a request to the Telegram Bot API.
///
/// A function must be passed in which returns a string future given a
/// URL string. Using futures here allows us to abstract over
/// synchronicity; that is, this function doesn't care whether the
/// request is done synchronously or asynchronously.
///
/// * `send`: Function that sends a request to the passed URL and
/// returns a future for the result.
///
/// * `token`: The bot token given by the Botfather.
///
/// * `method`: The bot API method, e.g. "getUpdates" or
/// "sendMessage".
fn request<S, F, E, T>(send: S, token: &str, method: &str) -> impl Future<Item = T, Error = E>
where
    S: FnOnce(&str) -> F,
    F: Future<Item = String, Error = E>,
    T: DeserializeOwned,
{
    const BASE_URL: &'static str = "https://api.telegram.org/";

    let mut url_str = String::from(BASE_URL);
    url_str.push_str("bot");
    url_str.push_str(token);
    url_str.push('/');
    url_str.push_str(method);

    send(&url_str).map(|s| serde_json::from_str(&s).expect("Received invalid JSON response"))
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

fn main() {
    let token = std::env::var(TOKEN_ENV_VAR).expect("Missing TG_BOT_TOKEN env var");
    let client = reqwest::Client::new();

    let result = request(|url| synchronous_send(&client, url), &token, "getMe").wait();
    let me: tg::Response<tg::User> = result.unwrap();
    println!("{:?}", me);
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::Deserialize;
    use serde::Serialize;

    #[test]
    /// Tests that `request` formats its request correctly.
    fn request_format() {
        const TOKEN: &'static str = "123:abc-xyz";
        const METHOD: &'static str = "fooBar";
        const EXPECTED_URL: &'static str = "https://api.telegram.org/bot123:abc-xyz/fooBar";

        // Our `send` implementation that will verify what `request`
        // sends.
        let mock_send = |url: &str| {
            assert_eq!(url, EXPECTED_URL);

            future::ok::<String, ()>(serde_json::to_string(&()).unwrap())
        };

        request::<_, _, (), ()>(mock_send, TOKEN, METHOD)
            .wait()
            .unwrap();
    }

    #[test]
    /// Tests that `request` correctly returns the result it receives.
    fn request_result() {
        #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
        struct Fromble {
            n: u32,
            b: bool,
        }

        let expected_result = Fromble { n: 1, b: true };

        let stub_send =
            |_: &str| future::ok::<String, ()>(serde_json::to_string(&expected_result).unwrap());

        let result: Fromble = request(stub_send, "", "").wait().unwrap();

        assert_eq!(result, expected_result);
    }
}
