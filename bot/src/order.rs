#[derive(Debug, Clone, Default)]
pub struct OrderDraft {
    pub order_number: Option<String>,
    pub date: Option<String>,
    pub customer: Option<String>,
    pub product: Option<String>,
    pub status: Option<String>,
    pub material_display: Option<String>,
    pub color: Option<String>,
    pub kg: Option<f64>,
    pub width_mm: Option<f64>,
    pub roll_count: Option<f64>,
    pub first_material: Option<String>,
    pub first_micron: Option<String>,
    pub second_material: Option<String>,
    pub second_micron: Option<String>,
    pub third_material: Option<String>,
    pub third_micron: Option<String>,
    pub note: Option<String>,
    pub photo_file_id: Option<String>,
}

impl OrderDraft {
    pub fn require_text(value: &Option<String>, name: &str) -> Result<String, String> {
        value
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or_else(|| format!("{name} to'ldirilmagan"))
    }

    pub fn require_number(value: Option<f64>, name: &str) -> Result<f64, String> {
        value.ok_or_else(|| format!("{name} to'ldirilmagan"))
    }
}
