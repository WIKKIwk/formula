use crate::calc::calculate_order;
use crate::config::Config;
use crate::formatter::{calc_message, order_message};
use crate::material_parser::parse_material_layers;
use crate::order::OrderDraft;
use crate::registry::{ChatRole, RegistryStore};
use crate::state::{LoginSessions, LoginStep, Sessions, Step};
use crate::telegram::{TelegramClient, Update};
use tokio::time::{sleep, Duration};

pub struct BotApp {
    telegram: TelegramClient,
    sessions: Sessions,
    logins: LoginSessions,
    registry: RegistryStore,
}

impl BotApp {
    pub fn new(config: Config, telegram: TelegramClient) -> Self {
        let registry = RegistryStore::load(config.state_file.clone())
            .unwrap_or_else(|error| panic!("registry yuklanmadi: {error}"));
        Self {
            telegram,
            sessions: Sessions::default(),
            logins: LoginSessions::default(),
            registry,
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
        let message_id = message.message_id;
        let photo_file_id = largest_photo_file_id(&message.photo);
        let text = message.text.unwrap_or_default();
        let trimmed = text.trim();

        if matches!(trimmed, "/login" | "login") {
            let _ = self.telegram.delete_message(chat_id, message_id).await;
            let prompt_message_id = self.telegram.send_message(chat_id, "Login yozing").await?;
            self.logins.start(chat_id, prompt_message_id);
            return Ok(());
        }

        if self.logins.get_mut(chat_id).is_some() {
            self.handle_login_answer(chat_id, message_id, trimmed)
                .await?;
            return Ok(());
        }

        if matches!(trimmed, "/chatid" | "chatid") {
            self.telegram
                .send_message(chat_id, &format!("Bu chat ID: <code>{chat_id}</code>"))
                .await?;
            return Ok(());
        }

        if !self.registry.value().is_ready() {
            self.telegram
                .send_message(chat_id, &setup_message(chat_id, self.registry.value()))
                .await?;
            return Ok(());
        }

        if matches!(trimmed, "/start" | "/new" | "new" | "yangi") {
            if !self.is_admin_chat(chat_id) {
                self.telegram
                    .send_message(
                        chat_id,
                        "Buyurtma faqat admin chatdan qabul qilinadi. Admin chatni /login orqali ulang.",
                    )
                    .await?;
                return Ok(());
            }
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
            if !self.is_admin_chat(chat_id) {
                return Ok(());
            }
            let prompt = self.sessions.start(chat_id);
            self.telegram.send_message(chat_id, prompt).await?;
            return Ok(());
        }

        let outcome = {
            let session = self.sessions.get_mut(chat_id).expect("session exists");
            apply_answer(
                &mut session.draft,
                &mut session.step,
                trimmed,
                photo_file_id,
            )
        };

        match outcome {
            Flow::Ask(prompt) => {
                let _ = self.telegram.send_message(chat_id, prompt).await?;
            }
            Flow::Done(order) => {
                self.sessions.remove(chat_id);
                self.finish_order(chat_id, order).await?;
            }
            Flow::Error(message) => {
                let _ = self.telegram.send_message(chat_id, &message).await?;
            }
        }
        Ok(())
    }

    async fn handle_login_answer(
        &mut self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.telegram.delete_message(chat_id, message_id).await;
        let Some(session) = self.logins.get_mut(chat_id) else {
            return Ok(());
        };

        match session.step {
            LoginStep::Login => {
                session.login = Some(text.to_string());
                session.step = LoginStep::Password;
                self.telegram
                    .edit_message(chat_id, session.prompt_message_id, "Parol yozing")
                    .await?;
            }
            LoginStep::Password => {
                let login = session.login.clone().unwrap_or_default();
                let prompt_message_id = session.prompt_message_id;
                self.logins.remove(chat_id);

                match ChatRole::from_credentials(&login, text) {
                    Some(role) => {
                        self.registry
                            .set_role(role, chat_id)
                            .map_err(|error| format!("role saqlanmadi: {error}"))?;
                        self.telegram
                            .edit_message(
                                chat_id,
                                prompt_message_id,
                                &format!("Qabul qilindi: <b>{}</b>", role.label()),
                            )
                            .await?;
                    }
                    None => {
                        self.telegram
                            .edit_message(chat_id, prompt_message_id, "Login yoki parol noto'g'ri.")
                            .await?;
                    }
                }
            }
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
                let order_chat_id = self
                    .registry
                    .value()
                    .order_chat_id
                    .expect("checked by setup mode");
                let calc_chat_id = self
                    .registry
                    .value()
                    .calc_chat_id
                    .expect("checked by setup mode");
                let order_text = order_message(&order)?;
                if let Some(photo_file_id) = order.photo_file_id.as_deref() {
                    self.telegram
                        .send_photo(order_chat_id, photo_file_id, &order_text)
                        .await?;
                } else {
                    self.telegram
                        .send_message(order_chat_id, &order_text)
                        .await?;
                }
                self.telegram
                    .send_message(calc_chat_id, &calc_message(&order, &result)?)
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

    fn is_admin_chat(&self, chat_id: i64) -> bool {
        self.registry.value().admin_chat_id == Some(chat_id)
    }
}

fn setup_message(chat_id: i64, registry: &crate::registry::BotRegistry) -> String {
    let mut missing = Vec::new();
    if registry.order_chat_id.is_none() {
        missing.push("ma'lumot guruhi");
    }
    if registry.calc_chat_id.is_none() {
        missing.push("hisob-kitob guruhi");
    }
    if registry.admin_chat_id.is_none() {
        missing.push("admin chat");
    }
    let missing = if missing.is_empty() {
        "hammasi ulangan".to_string()
    } else {
        missing.join(", ")
    };

    format!(
        "Bot setup rejimida.\nBu chat ID: <code>{chat_id}</code>\nYetishmayapti: <b>{missing}</b>\n\nMa'lumot guruhi, hisob guruhi va admin chatni /login orqali ulang."
    )
}

enum Flow {
    Ask(&'static str),
    Done(OrderDraft),
    Error(String),
}

fn apply_answer(
    draft: &mut OrderDraft,
    step: &mut Step,
    text: &str,
    photo_file_id: Option<String>,
) -> Flow {
    let value = normalize_empty(text);
    let result = match *step {
        Step::OrderNumber => set_text(&mut draft.order_number, value),
        Step::Customer => set_text(&mut draft.customer, value),
        Step::Product => set_text(&mut draft.product, value),
        Step::Status => set_text(&mut draft.status, value),
        Step::MaterialDisplay => {
            if let Err(error) = set_text(&mut draft.material_display, value.clone()) {
                return Flow::Error(error);
            }
            if let Some(value) = value {
                apply_material_display(draft, &value);
            }
            Ok(())
        }
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
            advance_step(draft, step);
            return Flow::Ask(step.next_prompt());
        }
        Step::Photo => {
            if let Some(photo_file_id) = photo_file_id {
                draft.photo_file_id = Some(photo_file_id);
                return Flow::Done(draft.clone());
            }
            if value.is_none() {
                return Flow::Done(draft.clone());
            }
            return Flow::Error("Rasm yuboring yoki '-' yozing.".to_string());
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
        Step::Width if draft.first_material.is_some() => Step::Note,
        Step::Width => Step::FirstMaterial,
        Step::FirstMaterial => Step::FirstMicron,
        Step::FirstMicron => Step::SecondMaterial,
        Step::SecondMaterial if draft.second_material.is_none() => Step::ThirdMaterial,
        Step::SecondMaterial => Step::SecondMicron,
        Step::SecondMicron => Step::ThirdMaterial,
        Step::ThirdMaterial if draft.third_material.is_none() => Step::Note,
        Step::ThirdMaterial => Step::ThirdMicron,
        Step::ThirdMicron => Step::Note,
        Step::Note => Step::Photo,
        Step::Photo => Step::Photo,
    };
}

fn largest_photo_file_id(photos: &Option<Vec<crate::telegram::PhotoSize>>) -> Option<String> {
    photos
        .as_ref()?
        .iter()
        .max_by_key(|photo| photo.file_size.unwrap_or(photo.width * photo.height))
        .map(|photo| photo.file_id.clone())
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
    let number_text = value
        .chars()
        .filter(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ','))
        .collect::<String>()
        .replace(',', ".");
    let number = number_text
        .parse::<f64>()
        .map_err(|_| format!("{name} raqam bo'lishi kerak."))?;
    if number <= 0.0 {
        return Err(format!("{name} 0 dan katta bo'lishi kerak."));
    }
    *slot = Some(number);
    Ok(())
}

fn apply_material_display(draft: &mut OrderDraft, value: &str) {
    let layers = parse_material_layers(value);
    if layers.is_empty() {
        return;
    }
    draft.first_material = Some(layers[0].0.clone());
    draft.first_micron = Some(layers[0].1.clone());

    if let Some((material, micron)) = layers.get(1) {
        draft.second_material = Some(material.clone());
        draft.second_micron = Some(micron.clone());
    } else {
        draft.second_material = None;
        draft.second_micron = None;
    }

    if let Some((material, micron)) = layers.get(2) {
        draft.third_material = Some(material.clone());
        draft.third_micron = Some(micron.clone());
    } else {
        draft.third_material = None;
        draft.third_micron = None;
    }
}

fn normalize_empty(text: &str) -> Option<String> {
    let value = text.trim();
    if value.is_empty() || matches!(value, "-" | "--") {
        None
    } else {
        Some(value.to_string())
    }
}
