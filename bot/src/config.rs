use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub order_chat_id: Option<i64>,
    pub calc_chat_id: Option<i64>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            bot_token: require_env("BOT_TOKEN")?,
            order_chat_id: parse_optional_chat_id("ORDER_CHAT_ID")?,
            calc_chat_id: parse_optional_chat_id("CALC_CHAT_ID")?,
        })
    }

    pub fn is_ready(&self) -> bool {
        self.order_chat_id.is_some() && self.calc_chat_id.is_some()
    }
}

fn require_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("{name} env berilmagan"))
}

fn parse_optional_chat_id(name: &str) -> Result<Option<i64>, String> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|_| format!("{name} butun son bo'lishi kerak")),
        Err(_) => Ok(None),
    }
}
