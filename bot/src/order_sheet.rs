use crate::calc::CalcResult;
use crate::order::OrderDraft;
use chrono::Local;
use std::error::Error;
use umya_spreadsheet::{new_file, writer, Border};

pub fn build_order_sheet(
    order: &OrderDraft,
    result: &CalcResult,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut book = new_file();
    let sheet = book.sheet_mut(0)?;
    sheet.set_name("Buyurtma");

    let headers = [
        ("oym", "sana"),
        ("tushgan", "vaqti"),
        ("ish", "kod"),
        ("buyurtmalar nomi", ""),
        ("tushgan", "kg"),
        ("1-", "qavat"),
        ("2-", "qavat"),
        ("material", "razmeri"),
        ("1-q", "micron"),
        ("2-q", "micron"),
        ("umumiy", "metri"),
        ("val soni", ""),
        ("rezina", "razmeri"),
    ];

    for (index, (top, bottom)) in headers.iter().enumerate() {
        let column = index as u32 + 1;
        sheet.cell_mut((column, 1)).set_value(*top);
        sheet.cell_mut((column, 2)).set_value(*bottom);
        style_header(sheet, column, 1);
        style_header(sheet, column, 2);
    }

    let row = 3;
    for (column, value) in order_row(order, result)?.iter().enumerate() {
        sheet.cell_mut((column as u32 + 1, row)).set_value(value);
        style_body(sheet, column as u32 + 1, row);
    }

    let mut bytes = Vec::new();
    writer::xlsx::write_writer(&book, &mut bytes)?;
    Ok(bytes)
}

pub fn rubber_size(width_mm: f64) -> u32 {
    ((width_mm / 50.0).ceil() as u32 * 50).clamp(50, 1300)
}

pub fn sheet_filename(order: &OrderDraft) -> String {
    let number = order
        .order_number
        .as_deref()
        .unwrap_or("buyurtma")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect::<String>();
    format!(
        "buyurtma_{}.xlsx",
        if number.is_empty() { "jadval" } else { &number }
    )
}

fn order_row(order: &OrderDraft, result: &CalcResult) -> Result<Vec<String>, String> {
    let width = OrderDraft::require_number(order.width_mm, "RAZMER")?;
    Ok(vec![
        order.date.clone().unwrap_or_default(),
        Local::now().format("%H:%M").to_string(),
        OrderDraft::require_text(&order.order_number, "Buyurtma raqami")?,
        OrderDraft::require_text(&order.product, "Mahsulot")?,
        format_number(OrderDraft::require_number(order.kg, "KG")?),
        OrderDraft::require_text(&order.first_material, "1-qavat")?,
        second_material(order),
        format_number(width),
        OrderDraft::require_text(&order.first_micron, "1-mikron")?,
        second_micron(order),
        format_number(result.rounded_length),
        format_number(OrderDraft::require_number(order.roll_count, "Val soni")?),
        rubber_size(width).to_string(),
    ])
}

fn second_material(order: &OrderDraft) -> String {
    join_layers(
        order.second_material.as_deref().unwrap_or("--"),
        order.third_material.as_deref(),
    )
}

fn second_micron(order: &OrderDraft) -> String {
    join_layers(
        order.second_micron.as_deref().unwrap_or("--"),
        order.third_micron.as_deref(),
    )
}

fn join_layers(first: &str, second: Option<&str>) -> String {
    let Some(second) = second.filter(|value| !value.trim().is_empty()) else {
        return first.to_string();
    };
    if first.trim().is_empty() || first == "--" {
        second.to_string()
    } else {
        format!("{first}/{second}")
    }
}

fn format_number(value: f64) -> String {
    format!("{value:.0}")
}

fn style_header(sheet: &mut umya_spreadsheet::Worksheet, column: u32, row: u32) {
    let style = sheet.style_mut((column, row));
    style.set_background_color("FF9BE4EA");
    style.font_mut().set_bold(true);
    set_border(style);
}

fn style_body(sheet: &mut umya_spreadsheet::Worksheet, column: u32, row: u32) {
    let style = sheet.style_mut((column, row));
    style.set_background_color("FF9BE4EA");
    style.font_mut().set_bold(true);
    set_border(style);
}

fn set_border(style: &mut umya_spreadsheet::Style) {
    let borders = style.borders_mut();
    borders.left_mut().set_border_style(Border::BORDER_THIN);
    borders.right_mut().set_border_style(Border::BORDER_THIN);
    borders.top_mut().set_border_style(Border::BORDER_THIN);
    borders.bottom_mut().set_border_style(Border::BORDER_THIN);
}

#[cfg(test)]
mod tests {
    use super::{build_order_sheet, rubber_size};
    use crate::calc::CalcResult;
    use crate::order::OrderDraft;

    #[test]
    fn rounds_rubber_size_to_next_50() {
        assert_eq!(rubber_size(645.0), 650);
        assert_eq!(rubber_size(670.0), 700);
        assert_eq!(rubber_size(50.0), 50);
    }

    #[test]
    fn builds_order_sheet_bytes() {
        let order = OrderDraft {
            order_number: Some("004".to_string()),
            date: Some("05/01".to_string()),
            product: Some("zizi chups".to_string()),
            kg: Some(600.0),
            width_mm: Some(645.0),
            roll_count: Some(5.0),
            first_material: Some("pff".to_string()),
            first_micron: Some("18".to_string()),
            second_material: Some("--".to_string()),
            second_micron: Some("--".to_string()),
            ..OrderDraft::default()
        };
        let result = CalcResult {
            first_coeff: 1.0,
            other_coeff: 0.0,
            coeff_sum: 1.0,
            width_sm: 64.5,
            base_length: 0.0,
            waste_length: 0.0,
            rounded_length: 50_000.0,
        };

        let bytes = build_order_sheet(&order, &result).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }
}
