mod calc;
mod config;
mod formatter;
mod handlers;
mod order;
mod state;
mod telegram;

use config::Config;
use handlers::BotApp;
use telegram::TelegramClient;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("bot xatosi: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let telegram = TelegramClient::new(config.bot_token.clone());
    let mut app = BotApp::new(config, telegram);

    app.run().await
}
