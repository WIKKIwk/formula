use crate::calc::CalcResult;
use crate::order::OrderDraft;

pub fn order_message(order: &OrderDraft) -> Result<String, String> {
    let order_number = OrderDraft::require_text(&order.order_number, "Buyurtma raqami")?;
    let date = OrderDraft::require_text(&order.date, "Sana")?;
    let customer = OrderDraft::require_text(&order.customer, "Mijoz")?;
    let product = OrderDraft::require_text(&order.product, "Mahsulot")?;
    let status = OrderDraft::require_text(&order.status, "Holat")?;
    let material = OrderDraft::require_text(&order.material_display, "Material")?;
    let color = OrderDraft::require_text(&order.color, "Rang")?;
    let kg = OrderDraft::require_number(order.kg, "Tiraj")?;
    let width = OrderDraft::require_number(order.width_mm, "Uzunligi")?;
    let roll_count = OrderDraft::require_number(order.roll_count, "Val soni")?;
    let note = order.note.as_deref().unwrap_or("-");

    Ok(format!(
        "Buyurtma raqami: №{}  {}\nMijoz: {}\nMahsulot: {}\nHolat: {}\n\n1. Material: {}\n2. Rang: {}\n3. Tiraj: {:.0} kg\n4. uzunligi: {:.0}mm\n5. Rang soni: {:.0}\n\n<b>Eslatm:</b> {}",
        esc(&order_number),
        esc(&date),
        esc(&customer),
        esc(&product),
        esc(&status),
        esc(&material),
        esc(&color),
        kg,
        width,
        roll_count,
        esc(note)
    ))
}

pub fn draft_form_message(order: &OrderDraft, prompt: &str) -> String {
    format!(
        "<b>DMBO</b>\nBuyurtma raqami: {}\nMijoz: {}\nMahsulot: {}\nHolat: {}\n\n1. Material: {}\n2. Rang: {}\n3. Tiraj: {}\n4. uzunligi: {}\n5. Val soni: {}\n\n<b>Eslatm:</b> {}\n\n{}",
        esc(display_text(&order.order_number)),
        esc(display_text(&order.customer)),
        esc(display_text(&order.product)),
        esc(display_text(&order.status)),
        esc(display_text(&order.material_display)),
        esc(display_text(&order.color)),
        esc(&display_kg(order.kg)),
        esc(&display_width(order.width_mm)),
        esc(&display_number(order.roll_count)),
        esc(order.note.as_deref().unwrap_or("")),
        esc(prompt)
    )
}

pub fn calc_message(order: &OrderDraft, result: &CalcResult) -> Result<String, String> {
    let order_number = OrderDraft::require_text(&order.order_number, "Buyurtma raqami")?;
    let kg = OrderDraft::require_number(order.kg, "KG")?;
    let width = OrderDraft::require_number(order.width_mm, "RAZMER")?;
    let roll_count = OrderDraft::require_number(order.roll_count, "Val soni")?;
    let q1 = OrderDraft::require_text(&order.first_material, "1-qavat")?;
    let m1 = OrderDraft::require_text(&order.first_micron, "1-mikron")?;
    let q2 = order.second_material.as_deref().unwrap_or("--");
    let m2 = order.second_micron.as_deref().unwrap_or("--");
    let q3 = order.third_material.as_deref().unwrap_or("--");
    let m3 = order.third_micron.as_deref().unwrap_or("--");

    Ok(format!(
        "<b>Hisob-kitob</b>\nBuyurtma: №{}\nKG: {:.0}\nRAZMER: {:.0} mm\nVal soni: {:.0}\n\n1-qavat: {} {} => {}\n2-qavat: {} {}{}\n\nKoeff: {} + {} = {}\nRazmer: {:.0} mm = {} sm\nBase: {:.0}\nAtxod 5%: {:.0}\n\n<b>Yakuniy uzunlik: {:.0}</b>",
        esc(&order_number),
        kg,
        width,
        roll_count,
        esc(&q1),
        esc(&m1),
        result.first_coeff,
        esc(q2),
        esc(m2),
        third_line(q3, m3),
        result.first_coeff,
        result.other_coeff,
        result.coeff_sum,
        width,
        result.width_sm,
        result.base_length,
        result.waste_length,
        result.rounded_length
    ))
}

fn display_text(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("")
}

fn display_kg(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0} kg"))
        .unwrap_or_default()
}

fn display_width(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}mm"))
        .unwrap_or_default()
}

fn display_number(value: Option<f64>) -> String {
    value.map(|value| format!("{value:.0}")).unwrap_or_default()
}

fn third_line(material: &str, micron: &str) -> String {
    if material.trim().is_empty() || material == "--" {
        String::new()
    } else {
        format!("\n3-qavat: {} {}", esc(material), esc(micron))
    }
}

fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
