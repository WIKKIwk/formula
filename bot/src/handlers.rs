use crate::calc::calculate_order;
use crate::config::Config;
use crate::formatter::{calc_message, order_message};
use crate::order::OrderDraft;
use crate::state::{Sessions, Step};
use crate::telegram::{TelegramClient, Update};
use tokio::time::{sleep, Duration};

pub struct BotApp {
    config: Config,
    telegram: TelegramClient,
    sessions: Sessions,
}

impl BotApp {
    pub fn new(config: Config, telegram: TelegramClient) -> Self {
        Self {
            config,
            telegram,
            sessions: Sessions::default(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut offset = None;
        loop {
            match self.telegram.get_updates(offset).await {
                Ok(updates) => {
                    for update in updates {
                        offset = Some(update.update_id + 1);
                        self.handle_update(update).await?;
                    }
                }
                Err(error) => {
                    eprintln!("polling xatosi: {error}");
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
    }

    async fn handle_update(&mut self, update: Update) -> Result<(), Box<dyn std::error::Error>> {
        let Some(message) = update.message else {
            return Ok(());
        };
        let chat_id = message.chat.id;
        let text = message.text.unwrap_or_default();
        let trimmed = text.trim();

        if matches!(trimmed, "/start" | "/new" | "new" | "yangi") {
            let prompt = self.sessions.start(chat_id);
            self.telegram.send_message(chat_id, prompt).await?;
            return Ok(());
        }
        if matches!(trimmed, "/cancel" | "cancel" | "bekor") {
            self.sessions.remove(chat_id);
            self.telegram
                .send_message(chat_id, "Bekor qilindi. Yangi buyurtma uchun /new yozing.")
                .await?;
            return Ok(());
        }

        if self.sessions.get_mut(chat_id).is_none() {
            let prompt = self.sessions.start(chat_id);
            self.telegram.send_message(chat_id, prompt).await?;
            return Ok(());
        }

        let outcome = {
            let session = self.sessions.get_mut(chat_id).expect("session exists");
            apply_answer(&mut session.draft, &mut session.step, trimmed)
        };

        match outcome {
            Flow::Ask(prompt) => self.telegram.send_message(chat_id, prompt).await?,
            Flow::Done(order) => {
                self.sessions.remove(chat_id);
                self.finish_order(chat_id, order).await?;
            }
            Flow::Error(message) => self.telegram.send_message(chat_id, &message).await?,
        }
        Ok(())
    }

    async fn finish_order(
        &self,
        chat_id: i64,
        order: OrderDraft,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match calculate_order(&order) {
            Ok(result) => {
                self.telegram
                    .send_message(self.config.order_chat_id, &order_message(&order)?)
                    .await?;
                self.telegram
                    .send_message(self.config.calc_chat_id, &calc_message(&order, &result)?)
                    .await?;
                self.telegram
                    .send_message(
                        chat_id,
                        "Buyurtma yuborildi. Keyingi buyurtma uchun /new yozing.",
                    )
                    .await?;
            }
            Err(error) => {
                self.telegram
                    .send_message(
                        chat_id,
                        &format!("Hisoblashda xato: {error}\nQaytadan boshlash uchun /new yozing."),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

enum Flow {
    Ask(&'static str),
    Done(OrderDraft),
    Error(String),
}

fn apply_answer(draft: &mut OrderDraft, step: &mut Step, text: &str) -> Flow {
    let value = normalize_empty(text);
    let result = match *step {
        Step::OrderNumber => set_text(&mut draft.order_number, value),
        Step::Customer => set_text(&mut draft.customer, value),
        Step::Product => set_text(&mut draft.product, value),
        Step::Status => set_text(&mut draft.status, value),
        Step::MaterialDisplay => set_text(&mut draft.material_display, value),
        Step::Color => set_text(&mut draft.color, value),
        Step::Kg => set_number(&mut draft.kg, value, "Tiraj kg"),
        Step::Width => set_number(&mut draft.width_mm, value, "Uzunligi mm"),
        Step::FirstMaterial => set_text(&mut draft.first_material, value),
        Step::FirstMicron => set_text(&mut draft.first_micron, value),
        Step::SecondMaterial => set_optional_text(&mut draft.second_material, value),
        Step::SecondMicron => set_text(&mut draft.second_micron, value),
        Step::ThirdMaterial => set_optional_text(&mut draft.third_material, value),
        Step::ThirdMicron => set_text(&mut draft.third_micron, value),
        Step::Note => {
            if let Err(error) = set_optional_text(&mut draft.note, value) {
                return Flow::Error(error);
            }
            return Flow::Done(draft.clone());
        }
    };

    if let Err(error) = result {
        return Flow::Error(error);
    }
    advance_step(draft, step);
    Flow::Ask(step.next_prompt())
}

fn advance_step(draft: &OrderDraft, step: &mut Step) {
    *step = match *step {
        Step::OrderNumber => Step::Customer,
        Step::Customer => Step::Product,
        Step::Product => Step::Status,
        Step::Status => Step::MaterialDisplay,
        Step::MaterialDisplay => Step::Color,
        Step::Color => Step::Kg,
        Step::Kg => Step::Width,
        Step::Width => Step::FirstMaterial,
        Step::FirstMaterial => Step::FirstMicron,
        Step::FirstMicron => Step::SecondMaterial,
        Step::SecondMaterial if draft.second_material.is_none() => Step::ThirdMaterial,
        Step::SecondMaterial => Step::SecondMicron,
        Step::SecondMicron => Step::ThirdMaterial,
        Step::ThirdMaterial if draft.third_material.is_none() => Step::Note,
        Step::ThirdMaterial => Step::ThirdMicron,
        Step::ThirdMicron => Step::Note,
        Step::Note => Step::Note,
    };
}

fn set_text(slot: &mut Option<String>, value: Option<String>) -> Result<(), String> {
    let Some(value) = value else {
        return Err("Bo'sh bo'lmasligi kerak.".to_string());
    };
    *slot = Some(value);
    Ok(())
}

fn set_optional_text(slot: &mut Option<String>, value: Option<String>) -> Result<(), String> {
    *slot = value;
    Ok(())
}

fn set_number(slot: &mut Option<f64>, value: Option<String>, name: &str) -> Result<(), String> {
    let Some(value) = value else {
        return Err(format!("{name} bo'sh bo'lmasligi kerak."));
    };
    let number = value
        .replace(' ', "")
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("{name} raqam bo'lishi kerak."))?;
    if number <= 0.0 {
        return Err(format!("{name} 0 dan katta bo'lishi kerak."));
    }
    *slot = Some(number);
    Ok(())
}

fn normalize_empty(text: &str) -> Option<String> {
    let value = text.trim();
    if value.is_empty() || matches!(value, "-" | "--") {
        None
    } else {
        Some(value.to_string())
    }
}
