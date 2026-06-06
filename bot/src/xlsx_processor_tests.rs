use crate::xlsx_processor::process_xlsx;
use calamine::{open_workbook_auto, Reader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use umya_spreadsheet::{new_file, writer, Color};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn processes_xlsx_and_returns_xlsx() {
    let input_path = temp_path("test_input", "xlsx");
    let mut book = new_file();
    let sheet = book.sheet_mut(0).unwrap();
    sheet.cell_mut("A1").set_value("KG");
    sheet.cell_mut("B1").set_value("RAZMER");
    sheet.cell_mut("C1").set_value("1 QAVAT");
    sheet.cell_mut("D1").set_value("1 MIKRON");
    sheet.cell_mut("E1").set_value("2 QAVAT");
    sheet.cell_mut("F1").set_value("2 MIKRON");
    sheet.cell_mut("A2").set_value("300");
    sheet.cell_mut("B2").set_value("530");
    sheet.cell_mut("C2").set_value("pet");
    sheet.cell_mut("D2").set_value("12");
    sheet.cell_mut("E2").set_value("pe pr");
    sheet.cell_mut("F2").set_value("30");
    sheet
        .style_mut("H1")
        .set_background_color(Color::COLOR_BLUE_STR);
    sheet
        .style_mut("F1")
        .set_background_color(Color::COLOR_BLUE_STR);
    sheet
        .style_mut("F2")
        .font_mut()
        .color_mut()
        .set_argb_str("00FF0000");
    writer::xlsx::write(&book, &input_path).unwrap();

    let input = std::fs::read(&input_path).unwrap();
    let report = process_xlsx(&input).unwrap();
    let output_path = temp_path("test_output", "xlsx");
    std::fs::write(&output_path, &report.output).unwrap();

    let mut workbook = open_workbook_auto(&output_path).unwrap();
    let range = workbook.worksheet_range_at(0).unwrap().unwrap();
    assert_eq!(
        range.get((0, 6)).unwrap().to_string(),
        "HISOBLANGAN_UZUNLIK"
    );
    assert_eq!(range.get((1, 6)).unwrap().to_string(), "12000");
    assert_eq!(range.get((1, 7)).unwrap().to_string(), "OK");
    assert_eq!(report.processed_count, 1);
    assert_eq!(report.ok_count, 1);
    assert_eq!(report.error_count, 0);

    let output_book = umya_spreadsheet::reader::xlsx::read(&output_path).unwrap();
    let output_sheet = output_book.sheet(0).unwrap();
    assert_eq!(output_sheet.style("G1"), output_sheet.style("F1"));
    assert_eq!(output_sheet.style("G2"), output_sheet.style("F2"));

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn processes_sheet_without_header_by_data_layout() {
    let input_path = temp_path("test_no_header_input", "xlsx");
    let mut book = new_file();
    let sheet = book.sheet_mut(0).unwrap();
    sheet.cell_mut("A1").set_value(" ");
    sheet.cell_mut("B1").set_value("dfa");
    sheet.cell_mut("C1").set_value("df");
    sheet.cell_mut("D1").set_value("fdasfda");
    sheet.cell_mut("F2").set_value("fdsa");

    for (row, kg) in [(4, "300"), (5, "600"), (6, "800")] {
        sheet.cell_mut((6, row)).set_value(kg);
        sheet.cell_mut((7, row)).set_value("pet");
        sheet.cell_mut((8, row)).set_value("pe pr");
        sheet.cell_mut((9, row)).set_value("530");
        sheet.cell_mut((10, row)).set_value("12");
        sheet.cell_mut((11, row)).set_value("30");
    }
    writer::xlsx::write(&book, &input_path).unwrap();

    let input = std::fs::read(&input_path).unwrap();
    let report = process_xlsx(&input).unwrap();

    assert_eq!(report.processed_count, 3);
    assert_eq!(report.ok_count, 3);

    let _ = std::fs::remove_file(input_path);
}

fn temp_path(name: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "formula_bot_{name}_{}_{}_{}.{}",
        std::process::id(),
        nanos,
        counter,
        extension
    ))
}
