use crate::calc::calculate_order;
use crate::order::OrderDraft;
use calamine::{open_workbook_auto, Data, Reader};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use umya_spreadsheet::{reader, writer};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct XlsxProcessReport {
    pub output: Vec<u8>,
    pub processed_count: usize,
    pub ok_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct ColumnIndexes {
    kg: usize,
    width: usize,
    first_material: usize,
    first_micron: usize,
    second_material: usize,
    second_micron: usize,
}

#[derive(Debug)]
struct RowResult {
    row_index: usize,
    value: Result<f64, String>,
}

pub fn process_xlsx(input: &[u8]) -> Result<XlsxProcessReport, Box<dyn Error>> {
    let input_path = temp_path("input", "xlsx");
    let output_path = temp_path("output", "xlsx");
    std::fs::write(&input_path, input)?;

    let result = process_xlsx_paths(&input_path, &output_path);
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    result
}

fn process_xlsx_paths(input: &Path, output: &Path) -> Result<XlsxProcessReport, Box<dyn Error>> {
    let rows = read_xlsx_rows(input)?;
    let (header_index, indexes) = find_header_row(&rows)
        .ok_or("kerakli ustunlar topilmadi: KG, RAZMER, 1 QAVAT, 1 MIKRON, 2 QAVAT, 2 MIKRON")?;

    let mut results = Vec::new();
    let mut ok_count = 0;
    let mut error_count = 0;
    for (row_index, row) in rows.iter().enumerate().skip(header_index + 1) {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let value = match calculate_xlsx_row(row, indexes) {
            Ok(length) => {
                ok_count += 1;
                Ok(length)
            }
            Err(message) => {
                error_count += 1;
                Err(message)
            }
        };
        results.push(RowResult { row_index, value });
    }

    let start_column = next_result_column(&rows);
    write_xlsx_results(input, output, header_index, start_column, &results)?;
    Ok(XlsxProcessReport {
        output: std::fs::read(output)?,
        processed_count: ok_count + error_count,
        ok_count,
        error_count,
    })
}

fn read_xlsx_rows(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or("Excel ichida sheet topilmadi")??;
    Ok(range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
}

fn write_xlsx_results(
    input: &Path,
    output: &Path,
    header_index: usize,
    start_column: u32,
    results: &[RowResult],
) -> Result<(), Box<dyn Error>> {
    let mut book = reader::xlsx::read(input)?;
    let worksheet = book.sheet_mut(0)?;
    let style_source_column = start_column.saturating_sub(1).max(1);
    let header_row = header_index as u32 + 1;

    worksheet
        .cell_mut((start_column, header_row))
        .set_value("HISOBLANGAN_UZUNLIK");
    worksheet
        .cell_mut((start_column + 1, header_row))
        .set_value("STATUS");
    worksheet
        .cell_mut((start_column + 2, header_row))
        .set_value("XATO");
    copy_result_styles(worksheet, style_source_column, header_row, start_column);

    for row_result in results {
        let excel_row = row_result.row_index as u32 + 1;
        match &row_result.value {
            Ok(length) => {
                worksheet
                    .cell_mut((start_column, excel_row))
                    .set_value(format!("{length:.0}"));
                worksheet
                    .cell_mut((start_column + 1, excel_row))
                    .set_value("OK");
            }
            Err(message) => {
                worksheet
                    .cell_mut((start_column + 1, excel_row))
                    .set_value("XATO");
                worksheet
                    .cell_mut((start_column + 2, excel_row))
                    .set_value(message);
            }
        }
        copy_result_styles(worksheet, style_source_column, excel_row, start_column);
    }

    writer::xlsx::write(&book, output)?;
    Ok(())
}

fn next_result_column(rows: &[Vec<String>]) -> u32 {
    rows.iter()
        .flat_map(|row| {
            row.iter()
                .enumerate()
                .filter(|(_, cell)| !cell.trim().is_empty())
                .map(|(index, _)| index)
        })
        .max()
        .map(|index| index as u32 + 2)
        .unwrap_or(1)
}

fn copy_result_styles(
    worksheet: &mut umya_spreadsheet::Worksheet,
    source_column: u32,
    row: u32,
    start_column: u32,
) {
    let style = worksheet.style((source_column, row)).clone();
    for column in start_column..=start_column + 2 {
        worksheet.set_style((column, row), style.clone());
    }
}

fn calculate_xlsx_row(row: &[String], indexes: ColumnIndexes) -> Result<f64, String> {
    let order = OrderDraft {
        kg: Some(parse_decimal(get_cell(row, indexes.kg))?),
        width_mm: Some(parse_decimal(get_cell(row, indexes.width))?),
        first_material: Some(get_cell(row, indexes.first_material).to_string()),
        first_micron: Some(get_cell(row, indexes.first_micron).to_string()),
        second_material: optional_cell(get_cell(row, indexes.second_material)),
        second_micron: optional_cell(get_cell(row, indexes.second_micron)),
        ..OrderDraft::default()
    };
    calculate_order(&order).map(|result| result.rounded_length)
}

fn get_cell(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("")
}

fn optional_cell(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().all(|ch| ch == '-') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_decimal(value: &str) -> Result<f64, String> {
    let number_text = value
        .chars()
        .filter(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ','))
        .collect::<String>()
        .replace(',', ".");
    let number = number_text
        .parse::<f64>()
        .map_err(|_| format!("raqam noto'g'ri: '{value}'"))?;
    if number <= 0.0 {
        return Err(format!("raqam 0 dan katta bo'lishi kerak: '{value}'"));
    }
    Ok(number)
}

fn find_header_row(rows: &[Vec<String>]) -> Option<(usize, ColumnIndexes)> {
    rows.iter()
        .take(30)
        .enumerate()
        .find_map(|(index, row)| {
            find_columns(row)
                .or_else(|| find_columns(&combined_header_row(rows, index)))
                .map(|columns| (index, columns))
        })
        .or_else(|| find_sheet_data_layout(rows))
}

fn find_columns(row: &[String]) -> Option<ColumnIndexes> {
    Some(ColumnIndexes {
        kg: find_column(row, HeaderKind::Kg)?,
        width: find_column(row, HeaderKind::Width)?,
        first_material: find_column(row, HeaderKind::FirstMaterial)?,
        first_micron: find_column(row, HeaderKind::FirstMicron)?,
        second_material: find_column(row, HeaderKind::SecondMaterial)?,
        second_micron: find_column(row, HeaderKind::SecondMicron)?,
    })
}

fn combined_header_row(rows: &[Vec<String>], index: usize) -> Vec<String> {
    let current = &rows[index];
    let Some(previous) = index.checked_sub(1).and_then(|index| rows.get(index)) else {
        return current.clone();
    };
    let len = current.len().max(previous.len());
    (0..len)
        .map(|column| {
            format!(
                "{} {}",
                previous.get(column).map(String::as_str).unwrap_or(""),
                current.get(column).map(String::as_str).unwrap_or("")
            )
        })
        .collect()
}

fn find_column(row: &[String], kind: HeaderKind) -> Option<usize> {
    row.iter()
        .position(|cell| header_matches(&normalize_header(cell), kind))
}

fn find_sheet_data_layout(rows: &[Vec<String>]) -> Option<(usize, ColumnIndexes)> {
    let indexes = ColumnIndexes {
        kg: 5,
        first_material: 6,
        second_material: 7,
        width: 8,
        first_micron: 9,
        second_micron: 10,
    };

    let first_data_row = rows
        .iter()
        .take(30)
        .position(|row| looks_like_sheet_data_row(row, indexes))?;
    let sample_count = rows
        .iter()
        .skip(first_data_row)
        .take(20)
        .filter(|row| looks_like_sheet_data_row(row, indexes))
        .count();
    if sample_count < 3 || first_data_row == 0 {
        return None;
    }
    Some((first_data_row - 1, indexes))
}

fn looks_like_sheet_data_row(row: &[String], indexes: ColumnIndexes) -> bool {
    parse_decimal(get_cell(row, indexes.kg)).is_ok()
        && parse_decimal(get_cell(row, indexes.width)).is_ok()
        && !get_cell(row, indexes.first_material).trim().is_empty()
        && !get_cell(row, indexes.second_material).trim().is_empty()
        && !get_cell(row, indexes.first_micron).trim().is_empty()
        && !get_cell(row, indexes.second_micron).trim().is_empty()
}

#[derive(Clone, Copy)]
enum HeaderKind {
    Kg,
    Width,
    FirstMaterial,
    FirstMicron,
    SecondMaterial,
    SecondMicron,
}

fn normalize_header(value: &str) -> String {
    transliterate_header(value)
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn transliterate_header(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            'А' | 'а' => "a".chars().collect::<Vec<_>>(),
            'Б' | 'б' => "b".chars().collect(),
            'В' | 'в' => "v".chars().collect(),
            'Г' | 'г' => "g".chars().collect(),
            'Д' | 'д' => "d".chars().collect(),
            'Е' | 'е' | 'Ё' | 'ё' | 'Э' | 'э' => "e".chars().collect(),
            'Ж' | 'ж' => "j".chars().collect(),
            'З' | 'з' => "z".chars().collect(),
            'И' | 'и' | 'Й' | 'й' => "i".chars().collect(),
            'К' | 'к' => "k".chars().collect(),
            'Л' | 'л' => "l".chars().collect(),
            'М' | 'м' => "m".chars().collect(),
            'Н' | 'н' => "n".chars().collect(),
            'О' | 'о' => "o".chars().collect(),
            'П' | 'п' => "p".chars().collect(),
            'Р' | 'р' => "r".chars().collect(),
            'С' | 'с' => "s".chars().collect(),
            'Т' | 'т' => "t".chars().collect(),
            'У' | 'у' => "u".chars().collect(),
            'Ф' | 'ф' => "f".chars().collect(),
            'Х' | 'х' => "x".chars().collect(),
            'Ц' | 'ц' => "ts".chars().collect(),
            'Ч' | 'ч' => "ch".chars().collect(),
            'Ш' | 'ш' => "sh".chars().collect(),
            'Щ' | 'щ' => "sh".chars().collect(),
            'Ы' | 'ы' => "i".chars().collect(),
            'Ю' | 'ю' => "yu".chars().collect(),
            'Я' | 'я' => "ya".chars().collect(),
            'Қ' | 'қ' => "q".chars().collect(),
            'Ғ' | 'ғ' => "g".chars().collect(),
            'Ў' | 'ў' => "o".chars().collect(),
            'Ҳ' | 'ҳ' => "h".chars().collect(),
            _ => vec![ch],
        })
        .collect()
}

fn header_matches(header: &str, kind: HeaderKind) -> bool {
    match kind {
        HeaderKind::Kg => {
            matches!(header, "kg" | "kilo" | "ogirlik" | "ves" | "weight") || header.ends_with("kg")
        }
        HeaderKind::Width => matches!(
            header,
            "razmer" | "razmr" | "olcham" | "size" | "uzunligi" | "dlina" | "materialrazmeri"
        ),
        HeaderKind::FirstMaterial => matches!(
            header,
            "1qavat" | "1qavati" | "1qatlam" | "qavat1" | "qatlam1" | "1layer"
        ),
        HeaderKind::FirstMicron => matches!(
            header,
            "1mikron" | "1micron" | "1mkrn" | "mikron1" | "micron1" | "1qmicron"
        ),
        HeaderKind::SecondMaterial => matches!(
            header,
            "2qavat" | "2qavati" | "2qatlam" | "qavat2" | "qatlam2" | "2layer"
        ),
        HeaderKind::SecondMicron => matches!(
            header,
            "2mikron" | "2micron" | "2mkrn" | "mikron2" | "micron2" | "2qmicron"
        ),
    }
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => trim_float(*value),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::Error(value) => format!("{value:?}"),
    }
}

fn trim_float(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
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
