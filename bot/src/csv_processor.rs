use crate::calc::calculate_order;
use crate::order::OrderDraft;
use std::error::Error;

#[derive(Debug)]
pub struct CsvProcessReport {
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

pub fn process_csv(input: &[u8]) -> Result<CsvProcessReport, Box<dyn Error>> {
    let delimiter = detect_delimiter(input);
    let rows = read_csv(input, delimiter)?;
    let (header_index, indexes) = find_header_row(&rows)
        .ok_or("kerakli ustunlar topilmadi: KG, RAZMER, 1 QAVAT, 1 MIKRON, 2 QAVAT, 2 MIKRON")?;
    let mut headers = rows[header_index].clone();
    headers.extend([
        "HISOBLANGAN_UZUNLIK".to_string(),
        "STATUS".to_string(),
        "XATO".to_string(),
    ]);

    let mut output_rows = Vec::new();
    let mut ok_count = 0;
    let mut error_count = 0;

    for row in rows.iter().skip(header_index + 1) {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }

        let mut output_row = row.clone();
        output_row.resize(headers.len() - 3, String::new());

        match calculate_csv_row(row, indexes) {
            Ok(length) => {
                ok_count += 1;
                output_row.push(format!("{length:.0}"));
                output_row.push("OK".to_string());
                output_row.push(String::new());
            }
            Err(message) => {
                error_count += 1;
                output_row.push(String::new());
                output_row.push("XATO".to_string());
                output_row.push(message);
            }
        }

        output_rows.push(output_row);
    }

    let output = write_csv(&headers, &output_rows, delimiter)?;
    Ok(CsvProcessReport {
        output,
        processed_count: ok_count + error_count,
        ok_count,
        error_count,
    })
}

fn read_csv(input: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(input);
    let mut rows = Vec::new();
    for record in reader.records() {
        rows.push(record?.iter().map(ToOwned::to_owned).collect());
    }
    Ok(rows)
}

fn write_csv(
    headers: &[String],
    rows: &[Vec<String>],
    delimiter: u8,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());
    writer.write_record(headers)?;
    for row in rows {
        writer.write_record(row)?;
    }
    Ok(writer.into_inner()?)
}

fn detect_delimiter(input: &[u8]) -> u8 {
    let sample = String::from_utf8_lossy(input);
    let mut best = (b',', 0usize);
    for delimiter in [b',', b';', b'\t'] {
        let count = sample
            .lines()
            .take(10)
            .map(|line| {
                line.as_bytes()
                    .iter()
                    .filter(|byte| **byte == delimiter)
                    .count()
            })
            .sum();
        if count > best.1 {
            best = (delimiter, count);
        }
    }
    best.0
}

fn calculate_csv_row(row: &[String], indexes: ColumnIndexes) -> Result<f64, String> {
    let q1 = get_cell(row, indexes.first_material);
    let m1 = get_cell(row, indexes.first_micron);
    let q2 = get_cell(row, indexes.second_material);
    let m2 = get_cell(row, indexes.second_micron);

    let order = OrderDraft {
        kg: Some(parse_decimal(get_cell(row, indexes.kg))?),
        width_mm: Some(parse_decimal(get_cell(row, indexes.width))?),
        first_material: Some(q1.to_string()),
        first_micron: Some(m1.to_string()),
        second_material: optional_cell(q2),
        second_micron: optional_cell(m2),
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
    rows.iter().take(30).enumerate().find_map(|(index, row)| {
        find_columns(row)
            .or_else(|| find_columns(&combined_header_row(rows, index)))
            .map(|columns| (index, columns))
    })
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
        HeaderKind::Width => {
            matches!(
                header,
                "razmer" | "razmr" | "olcham" | "size" | "uzunligi" | "dlina" | "materialrazmeri"
            )
        }
        HeaderKind::FirstMaterial => {
            matches!(
                header,
                "1qavat" | "1qavati" | "1qatlam" | "qavat1" | "qatlam1" | "1layer"
            )
        }
        HeaderKind::FirstMicron => {
            matches!(
                header,
                "1mikron" | "1micron" | "1mkrn" | "mikron1" | "micron1" | "1qmicron"
            )
        }
        HeaderKind::SecondMaterial => {
            matches!(
                header,
                "2qavat" | "2qavati" | "2qatlam" | "qavat2" | "qatlam2" | "2layer"
            )
        }
        HeaderKind::SecondMicron => {
            matches!(
                header,
                "2mikron" | "2micron" | "2mkrn" | "mikron2" | "micron2" | "2qmicron"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::process_csv;

    #[test]
    fn processes_csv_and_returns_csv() {
        let input = b"KG,RAZMER,1 QAVAT,1 MIKRON,2 QAVAT,2 MIKRON\n300,530,pet,12,pe pr,30\n";
        let report = process_csv(input).unwrap();
        let output = String::from_utf8(report.output).unwrap();

        assert!(output.contains("HISOBLANGAN_UZUNLIK,STATUS,XATO"));
        assert!(output.contains("12000,OK,"));
        assert_eq!(report.processed_count, 1);
        assert_eq!(report.ok_count, 1);
        assert_eq!(report.error_count, 0);
    }

    #[test]
    fn processes_semicolon_csv() {
        let input = b"KG;RAZMER;1 QAVAT;1 MIKRON;2 QAVAT;2 MIKRON\n300;530;pet;12;pe pr;30\n";
        let report = process_csv(input).unwrap();
        let output = String::from_utf8(report.output).unwrap();

        assert!(output.contains("HISOBLANGAN_UZUNLIK;STATUS;XATO"));
        assert!(output.contains("12000;OK;"));
        assert_eq!(report.processed_count, 1);
    }

    #[test]
    fn processes_cyrillic_headers() {
        let input = "КГ;РАЗМЕР;1 ҚАВАТ;1 МИКРОН;2 ҚАВАТ;2 МИКРОН\n300;530;pet;12;pe pr;30\n";
        let report = process_csv(input.as_bytes()).unwrap();
        let output = String::from_utf8(report.output).unwrap();

        assert!(output.contains("12000;OK;"));
        assert_eq!(report.processed_count, 1);
    }

    #[test]
    fn processes_two_row_sheet_headers() {
        let input = concat!(
            ",oym,tushgan,ish,buyurtmalar nomi,tushgan ,1-,2-,material,1-q,2-q,umumiy\n",
            ",sana,vaqti,kod,,kg,qavati,qavati,razmeri,micron,micron,metri\n",
            ",30/06,18;30,/1411,spring zelen,300,pet,cpp,960,12,25,8500\n",
            ",30/06,18;31,/1412,zizi,1000,pet,pe pr,985,12,30,21000\n"
        );
        let report = process_csv(input.as_bytes()).unwrap();
        let output = String::from_utf8(report.output).unwrap();

        assert_eq!(report.processed_count, 2);
        assert_eq!(report.ok_count, 2);
        assert!(output.contains(",OK,"));
    }
}
