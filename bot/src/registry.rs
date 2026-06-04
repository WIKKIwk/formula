use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BotRegistry {
    pub order_chat_id: Option<i64>,
    pub calc_chat_id: Option<i64>,
    pub admin_chat_id: Option<i64>,
}

impl BotRegistry {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("state o'qib bo'lmadi: {error}"))?;
        serde_json::from_str(&content).map_err(|error| format!("state parse xatosi: {error}"))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("state papka yaratilmadi: {error}"))?;
            }
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("state serialize xatosi: {error}"))?;
        std::fs::write(path, content).map_err(|error| format!("state yozilmadi: {error}"))
    }

    pub fn is_ready(&self) -> bool {
        self.order_chat_id.is_some() && self.calc_chat_id.is_some() && self.admin_chat_id.is_some()
    }

    pub fn set_role(&mut self, role: ChatRole, chat_id: i64) {
        match role {
            ChatRole::Order => self.order_chat_id = Some(chat_id),
            ChatRole::Calc => self.calc_chat_id = Some(chat_id),
            ChatRole::Admin => self.admin_chat_id = Some(chat_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    Order,
    Calc,
    Admin,
}

impl ChatRole {
    pub fn from_credentials(login: &str, password: &str) -> Option<Self> {
        let login = login.trim().to_lowercase();
        match (login.as_str(), password.trim()) {
            ("guruh", "@#12asn") => Some(Self::Order),
            ("hisob", "@#12hsb") => Some(Self::Calc),
            ("chat", "@#12") => Some(Self::Admin),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Order => "ma'lumot tashlaydigan guruh",
            Self::Calc => "hisob-kitob guruhi",
            Self::Admin => "admin chat",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStore {
    path: PathBuf,
    value: BotRegistry,
}

impl RegistryStore {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let value = BotRegistry::load(&path)?;
        Ok(Self { path, value })
    }

    pub fn value(&self) -> &BotRegistry {
        &self.value
    }

    pub fn set_role(&mut self, role: ChatRole, chat_id: i64) -> Result<(), String> {
        self.value.set_role(role, chat_id);
        self.value.save(&self.path)
    }
}
