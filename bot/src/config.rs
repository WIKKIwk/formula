use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub state_file: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            bot_token: require_env("BOT_TOKEN")?,
            state_file: env::var("BOT_STATE_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("bot_state.json")),
        })
    }
}

fn require_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("{name} env berilmagan"))
}
