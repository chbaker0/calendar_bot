use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Response<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetUpdates {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub timeout: Option<i64>,
    pub allowed_updates: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SendMessage {
    pub chat_id: i64,
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub text: Option<String>,
}
