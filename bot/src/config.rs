use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub order_chat_id: i64,
    pub calc_chat_id: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            bot_token: require_env("BOT_TOKEN")?,
            order_chat_id: parse_chat_id("ORDER_CHAT_ID")?,
            calc_chat_id: parse_chat_id("CALC_CHAT_ID")?,
        })
    }
}

fn require_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("{name} env berilmagan"))
}

fn parse_chat_id(name: &str) -> Result<i64, String> {
    require_env(name)?
        .parse::<i64>()
        .map_err(|_| format!("{name} butun son bo'lishi kerak"))
}
