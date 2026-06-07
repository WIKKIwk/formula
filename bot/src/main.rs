mod calc;
mod config;
mod csv_processor;
mod file_handler;
mod formatter;
mod handlers;
mod material_parser;
mod order;
mod order_sheet;
mod registry;
mod state;
mod telegram;
mod xlsx_processor;
#[cfg(test)]
mod xlsx_processor_tests;

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
