use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct TelegramClient {
    token: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub result: T,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub text: Option<String>,
    pub photo: Option<Vec<PhotoSize>>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct PhotoSize {
    pub file_id: String,
    pub width: i64,
    pub height: i64,
    pub file_size: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SendMessage<'a> {
    chat_id: i64,
    text: &'a str,
    parse_mode: &'a str,
}

#[derive(Debug, Serialize)]
struct SendPhoto<'a> {
    chat_id: i64,
    photo: &'a str,
    caption: &'a str,
    parse_mode: &'a str,
}

#[derive(Debug, Serialize)]
struct EditMessage<'a> {
    chat_id: i64,
    message_id: i64,
    text: &'a str,
    parse_mode: &'a str,
}

#[derive(Debug, Serialize)]
struct DeleteMessage {
    chat_id: i64,
    message_id: i64,
}

#[derive(Debug, Deserialize)]
struct SentMessage {
    message_id: i64,
}

impl TelegramClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: Client::new(),
        }
    }

    pub async fn get_updates(
        &self,
        offset: Option<i64>,
    ) -> Result<Vec<Update>, Box<dyn std::error::Error>> {
        let url = self.url("getUpdates");
        let mut request = self.client.get(url).query(&[("timeout", "30")]);
        if let Some(offset) = offset {
            request = request.query(&[("offset", offset.to_string())]);
        }
        let http_response = request.send().await?;
        let status = http_response.status();
        if !status.is_success() {
            return Err(format!("Telegram getUpdates HTTP status: {status}").into());
        }
        let response = http_response.json::<ApiResponse<Vec<Update>>>().await?;
        if !response.ok {
            return Err("Telegram getUpdates ok=false".into());
        }
        Ok(response.result)
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let payload = SendMessage {
            chat_id,
            text,
            parse_mode: "HTML",
        };
        let response = self
            .client
            .post(self.url("sendMessage"))
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Telegram sendMessage HTTP status: {status}").into());
        }
        let response = response.json::<ApiResponse<SentMessage>>().await?;
        if !response.ok {
            return Err("Telegram sendMessage ok=false".into());
        }
        Ok(response.result.message_id)
    }

    pub async fn send_photo(
        &self,
        chat_id: i64,
        photo: &str,
        caption: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let payload = SendPhoto {
            chat_id,
            photo,
            caption,
            parse_mode: "HTML",
        };
        let response = self
            .client
            .post(self.url("sendPhoto"))
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Telegram sendPhoto HTTP status: {status}").into());
        }
        let response = response.json::<ApiResponse<SentMessage>>().await?;
        if !response.ok {
            return Err("Telegram sendPhoto ok=false".into());
        }
        Ok(response.result.message_id)
    }

    pub async fn edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = EditMessage {
            chat_id,
            message_id,
            text,
            parse_mode: "HTML",
        };
        let response = self
            .client
            .post(self.url("editMessageText"))
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Telegram editMessage HTTP status: {status}").into());
        }
        let response = response.json::<serde_json::Value>().await?;
        if response.get("ok").and_then(|value| value.as_bool()) != Some(true) {
            return Err(format!("Telegram editMessage xato: {response}").into());
        }
        Ok(())
    }

    pub async fn delete_message(
        &self,
        chat_id: i64,
        message_id: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = DeleteMessage {
            chat_id,
            message_id,
        };
        let response = self
            .client
            .post(self.url("deleteMessage"))
            .json(&payload)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Ok(());
        }
        Ok(())
    }

    fn url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }
}
