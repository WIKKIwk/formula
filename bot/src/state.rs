use crate::order::OrderDraft;
use chrono::Local;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Session {
    pub step: Step,
    pub draft: OrderDraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    OrderNumber,
    Customer,
    Product,
    Status,
    MaterialDisplay,
    Color,
    Kg,
    Width,
    FirstMaterial,
    FirstMicron,
    SecondMaterial,
    SecondMicron,
    ThirdMaterial,
    ThirdMicron,
    Note,
}

#[derive(Default)]
pub struct Sessions {
    values: HashMap<i64, Session>,
}

impl Sessions {
    pub fn start(&mut self, chat_id: i64) -> &'static str {
        self.values.insert(
            chat_id,
            Session {
                step: Step::OrderNumber,
                draft: OrderDraft {
                    date: Some(Local::now().format("%d/%m/%y").to_string()),
                    ..OrderDraft::default()
                },
            },
        );
        "Buyurtma raqamini yozing. Masalan: T2437"
    }

    pub fn remove(&mut self, chat_id: i64) {
        self.values.remove(&chat_id);
    }

    pub fn get_mut(&mut self, chat_id: i64) -> Option<&mut Session> {
        self.values.get_mut(&chat_id)
    }
}

impl Step {
    pub fn next_prompt(self) -> &'static str {
        match self {
            Step::Customer => "Mijoz nomi?",
            Step::Product => "Mahsulot nomi?",
            Step::Status => "Holat? Masalan: rulon",
            Step::MaterialDisplay => "Material matni? Masalan: BOPP 20 + Metall BOPP 30",
            Step::Color => "Rang?",
            Step::Kg => "Tiraj, kg?",
            Step::Width => "Uzunligi/razmer, mm?",
            Step::FirstMaterial => "1-qavat material?",
            Step::FirstMicron => "1-qavat mikron?",
            Step::SecondMaterial => "2-qavat material? Bo'sh bo'lsa '-' yozing",
            Step::SecondMicron => "2-qavat mikron?",
            Step::ThirdMaterial => "3-qavat material? Bo'sh bo'lsa '-' yozing",
            Step::ThirdMicron => "3-qavat mikron?",
            Step::Note => "Eslatma? Bo'sh bo'lsa '-' yozing",
            Step::OrderNumber => "Buyurtma raqamini yozing.",
        }
    }
}
