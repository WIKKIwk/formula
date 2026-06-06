use calamine::{open_workbook_auto, Data, Reader};
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use umya_spreadsheet::{reader, writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaterialFamily {
    FirstLayer,
    McpCpp,
    Jem,
    Pe,
    Twist,
    Empty,
}

#[derive(Debug)]
struct Layer<'a> {
    material: &'a str,
    micron_text: &'a str,
    micron: u32,
}

#[derive(Debug)]
struct Calculation<'a> {
    kg: f64,
    razmer_mm: f64,
    first_layer: Layer<'a>,
    second_layer: Layer<'a>,
    waste_percent: f64,
    round_to: f64,
}

#[derive(Debug)]
struct ResultBreakdown {
    first_coeff: f64,
    second_coeff: f64,
    coeff_sum: f64,
    razmer_sm: f64,
    density_part: f64,
    base_length: f64,
    waste_length: f64,
    length_with_waste: f64,
    rounded_length: f64,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args
        .first()
        .is_some_and(|arg| matches!(arg.as_str(), "--interactive" | "-i"))
    {
        if let Err(error) = run_interactive() {
            eprintln!("Xato: {error}");
            process::exit(1);
        }
        return;
    }

    if should_process_file(&args) {
        if let Err(error) = process_file_command(&args) {
            eprintln!("Xato: {error}");
            process::exit(1);
        }
        return;
    }

    if args.first().is_some_and(|arg| arg == "--demo") {
        run_demo_rows();
        return;
    }

    let calculation = match calculation_from_args(&args) {
        Ok(calculation) => calculation,
        Err(message) => {
            eprintln!("{message}");
            eprintln!();
            print_usage();
            process::exit(2);
        }
    };

    match calculate(&calculation) {
        Ok(result) => print_result(&calculation, &result),
        Err(message) => {
            eprintln!("Xato: {message}");
            process::exit(1);
        }
    }
}

fn calculation_from_args(args: &[String]) -> Result<Calculation<'static>, String> {
    if args.is_empty() {
        return Ok(example_1178());
    }

    let kg = read_number(args, "--kg")?;
    let razmer_mm = read_number(args, "--razmer")?;
    let first_material = read_text(args, "--q1")?;
    let first_micron_text = read_text(args, "--m1")?;
    let first_micron = parse_micron(&first_micron_text)?;
    let second_material = read_text(args, "--q2")?;
    let second_micron_text = if is_empty_material(&second_material) {
        "--".to_string()
    } else {
        read_text(args, "--m2")?
    };
    let third_material = read_optional_text(args, "--q3").unwrap_or_default();
    let third_micron_text = read_optional_text(args, "--m3").unwrap_or_default();
    let (second_material, second_micron_text) = merge_optional_third_layer(
        second_material,
        second_micron_text,
        third_material,
        third_micron_text,
    )?;
    let second_micron = if is_empty_material(&second_material) {
        0
    } else {
        parse_micron(&second_micron_text)?
    };
    let waste_percent = read_optional_number(args, "--waste")?.unwrap_or(5.0);
    let round_to = read_optional_number(args, "--round")?.unwrap_or(500.0);

    Ok(Calculation {
        kg,
        razmer_mm,
        first_layer: Layer {
            material: Box::leak(first_material.into_boxed_str()),
            micron_text: Box::leak(first_micron_text.into_boxed_str()),
            micron: first_micron,
        },
        second_layer: Layer {
            material: Box::leak(second_material.into_boxed_str()),
            micron_text: Box::leak(second_micron_text.into_boxed_str()),
            micron: second_micron,
        },
        waste_percent,
        round_to,
    })
}

fn example_1178() -> Calculation<'static> {
    Calculation {
        kg: 300.0,
        razmer_mm: 530.0,
        first_layer: Layer {
            material: "pet",
            micron_text: "12",
            micron: 12,
        },
        second_layer: Layer {
            material: "pe pr",
            micron_text: "30",
            micron: 30,
        },
        waste_percent: 5.0,
        round_to: 500.0,
    }
}

fn run_interactive() -> Result<(), String> {
    println!("Terminal hisoblash rejimi");
    println!("Chiqish uchun KG joyiga q yozing.");
    println!("3-qavatni skip qilish uchun material joyini bo'sh qoldiring.");
    println!("Atxod foizi va yaxlitlashda Enter bosilsa default qiymat olinadi.");
    println!();

    loop {
        println!("--- Yangi hisob ---");
        let Some(kg) = prompt_decimal_or_quit("Mahsulot og'irligi, kg (q = chiqish)")? else {
            println!("Chiqildi.");
            return Ok(());
        };
        let razmer_mm = prompt_decimal("Razmer, mm")?;
        let first_material = prompt_required("1-qavat material")?;
        let first_micron_text = prompt_micron_required("1-qavat mikron")?;
        let first_micron = parse_micron(&first_micron_text)?;
        let second_material = prompt_optional("2-qavat material (bo'sh bo'lsa skip)")?;
        let second_micron_text = if second_material.is_empty() {
            "--".to_string()
        } else {
            prompt_micron_required("2-qavat mikron")?
        };
        let third_material = prompt_optional("3-qavat material (bo'sh bo'lsa skip)")?;
        let third_micron_text = if third_material.is_empty() {
            String::new()
        } else {
            prompt_micron_required("3-qavat mikron")?
        };
        let waste_percent = prompt_optional_decimal("Atxod foizi", 5.0)?;
        let round_to = prompt_optional_decimal("Yaxlitlash", 500.0)?;
        let (second_material, second_micron_text) = merge_optional_third_layer(
            second_material,
            second_micron_text,
            third_material,
            third_micron_text,
        )?;
        let second_micron = if is_empty_material(&second_material) {
            0
        } else {
            parse_micron(&second_micron_text)?
        };

        let calculation = Calculation {
            kg,
            razmer_mm,
            first_layer: Layer {
                material: &first_material,
                micron_text: &first_micron_text,
                micron: first_micron,
            },
            second_layer: Layer {
                material: &second_material,
                micron_text: &second_micron_text,
                micron: second_micron,
            },
            waste_percent,
            round_to,
        };

        println!();
        match calculate(&calculation) {
            Ok(result) => print_result(&calculation, &result),
            Err(message) => println!("Xato: {message}"),
        }
        println!();
    }
}

fn merge_optional_third_layer(
    second_material: String,
    second_micron_text: String,
    third_material: String,
    third_micron_text: String,
) -> Result<(String, String), String> {
    let second_empty = is_empty_material(&second_material);
    let third_empty = is_empty_material(&third_material);

    match (second_empty, third_empty) {
        (true, true) => Ok(("--".to_string(), "--".to_string())),
        (true, false) => {
            if third_micron_text.trim().is_empty() {
                return Err("3-qavat materiali bor, lekin mikroni berilmagan".to_string());
            }
            Ok((third_material, third_micron_text))
        }
        (false, true) => Ok((second_material, second_micron_text)),
        (false, false) => {
            if third_micron_text.trim().is_empty() {
                return Err("3-qavat materiali bor, lekin mikroni berilmagan".to_string());
            }
            Ok((
                format!("{second_material}/{third_material}"),
                format!("{second_micron_text}/{third_micron_text}"),
            ))
        }
    }
}

fn prompt_required(label: &str) -> Result<String, String> {
    loop {
        let value = prompt_optional(label)?;
        if !value.trim().is_empty() {
            return Ok(value);
        }
        println!("{label} bo'sh bo'lmasligi kerak.");
    }
}

fn prompt_optional(label: &str) -> Result<String, String> {
    print!("{label}: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("stdout flush xatosi: {error}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|error| format!("stdin o'qish xatosi: {error}"))?;

    Ok(input.trim().to_string())
}

fn prompt_decimal(label: &str) -> Result<f64, String> {
    loop {
        let value = prompt_required(label)?;
        match parse_decimal(&value) {
            Ok(number) if number > 0.0 => return Ok(number),
            Ok(_) => println!("{label} 0 dan katta bo'lishi kerak."),
            Err(message) => println!("{message}"),
        }
    }
}

fn prompt_decimal_or_quit(label: &str) -> Result<Option<f64>, String> {
    loop {
        let value = prompt_optional(label)?;
        if matches!(value.trim().to_lowercase().as_str(), "q" | "quit" | "exit") {
            return Ok(None);
        }
        if value.trim().is_empty() {
            println!("{label} bo'sh bo'lmasligi kerak.");
            continue;
        }
        match parse_decimal(&value) {
            Ok(number) if number > 0.0 => return Ok(Some(number)),
            Ok(_) => println!("{label} 0 dan katta bo'lishi kerak."),
            Err(message) => println!("{message}"),
        }
    }
}

fn prompt_optional_decimal(label: &str, default: f64) -> Result<f64, String> {
    loop {
        let value = prompt_optional(&format!("{label} [{default}]"))?;
        if value.trim().is_empty() {
            return Ok(default);
        }
        match parse_decimal(&value) {
            Ok(number) if number > 0.0 => return Ok(number),
            Ok(_) => println!("{label} 0 dan katta bo'lishi kerak."),
            Err(message) => println!("{message}"),
        }
    }
}

fn prompt_micron_required(label: &str) -> Result<String, String> {
    loop {
        let value = prompt_required(label)?;
        match parse_micron(&value) {
            Ok(_) => return Ok(value),
            Err(message) => println!("{message}"),
        }
    }
}

fn calculate(calculation: &Calculation<'_>) -> Result<ResultBreakdown, String> {
    if calculation.kg <= 0.0 {
        return Err("KG 0 dan katta bo'lishi kerak".to_string());
    }
    if calculation.razmer_mm <= 0.0 {
        return Err("RAZMER 0 dan katta bo'lishi kerak".to_string());
    }
    if calculation.round_to <= 0.0 {
        return Err("yaxlitlash qiymati 0 dan katta bo'lishi kerak".to_string());
    }

    let first_family = material_family(calculation.first_layer.material)?;
    let second_family = material_family(calculation.second_layer.material)?;
    let first_coeff = if first_family == MaterialFamily::Empty {
        0.0
    } else {
        coefficient_cell(&calculation.first_layer, true)?
    };
    let second_coeff = if second_family == MaterialFamily::Empty {
        0.0
    } else {
        coefficient_cell(&calculation.second_layer, false)?
    };
    if first_coeff + second_coeff <= 0.0 {
        return Err("kamida bitta qavat materiali bo'lishi kerak".to_string());
    }
    let coeff_sum = first_coeff + second_coeff;
    let razmer_sm = calculation.razmer_mm / 10.0;
    let density_part = coeff_sum * razmer_sm;
    let base_length = calculation.kg / density_part * 6000.0;
    let waste_length = base_length * calculation.waste_percent / 100.0;
    let length_with_waste = base_length + waste_length;
    let rounded_length = round_up(length_with_waste, calculation.round_to);

    Ok(ResultBreakdown {
        first_coeff,
        second_coeff,
        coeff_sum,
        razmer_sm,
        density_part,
        base_length,
        waste_length,
        length_with_waste,
        rounded_length,
    })
}

fn coefficient_cell(layer: &Layer<'_>, is_first_layer: bool) -> Result<f64, String> {
    let materials = split_materials(layer.material);
    let microns = parse_micron_parts(layer.micron_text)?;

    if materials.len() == 1 {
        return coefficient_single(materials[0], layer.micron, is_first_layer);
    }

    if materials.len() != microns.len() {
        return Err(format!(
            "material va mikron soni mos emas: '{}' va '{}'",
            layer.material, layer.micron_text
        ));
    }

    materials
        .iter()
        .zip(microns)
        .map(|(material, micron)| coefficient_single(material, micron, is_first_layer))
        .sum()
}

fn coefficient_single(material: &str, micron: u32, is_first_layer: bool) -> Result<f64, String> {
    let family = material_family(material)?;

    if is_first_layer
        && !matches!(family, MaterialFamily::Empty | MaterialFamily::Twist)
        && micron <= 20
    {
        return Ok(1.0);
    }
    if family == MaterialFamily::FirstLayer && micron <= 20 {
        return Ok(1.0);
    }

    let coefficient = match family {
        MaterialFamily::FirstLayer | MaterialFamily::McpCpp => mcp_cpp_coefficient(micron),
        MaterialFamily::Jem => jem_coefficient(micron),
        MaterialFamily::Pe => pe_coefficient(micron),
        MaterialFamily::Twist => Some(2.0),
        MaterialFamily::Empty => None,
    };

    coefficient.ok_or_else(|| coefficient_error(material, micron, family))
}

fn material_family(material: &str) -> Result<MaterialFamily, String> {
    let normalized = normalize_token(material);

    if normalized == "--" || normalized.is_empty() {
        return Ok(MaterialFamily::Empty);
    }
    if matches!(normalized.as_str(), "-" | "yoq" | "yuq" | "none" | "null") {
        return Ok(MaterialFamily::Empty);
    }
    if normalized.starts_with("twis")
        || normalized.starts_with("tuisim")
        || normalized.starts_with("tuism")
        || normalized.starts_with("tvis")
    {
        return Ok(MaterialFamily::Twist);
    }
    if normalized.starts_with("pet")
        || normalized.starts_with("mpet")
        || is_close_material(&normalized, "pet")
    {
        return Ok(MaterialFamily::FirstLayer);
    }
    if normalized.starts_with("opp")
        || normalized.starts_with("popp")
        || normalized == "st01"
        || is_close_material(&normalized, "opp")
        || is_close_material(&normalized, "popp")
    {
        return Ok(MaterialFamily::FirstLayer);
    }
    if matches!(normalized.as_str(), "map" | "mcpp" | "msr" | "msp") {
        return Ok(MaterialFamily::McpCpp);
    }
    if normalized.starts_with("mat")
        || normalized.starts_with("pff")
        || normalized.starts_with("pf")
        || is_close_material(&normalized, "mat")
    {
        return Ok(MaterialFamily::FirstLayer);
    }
    if normalized.starts_with("pe") || is_close_material(&normalized, "pe") {
        return Ok(MaterialFamily::Pe);
    }
    if normalized.starts_with("cpp")
        || normalized.starts_with("mcp")
        || is_close_material(&normalized, "cpp")
        || is_close_material(&normalized, "mcp")
    {
        return Ok(MaterialFamily::McpCpp);
    }
    if normalized.starts_with("jem") || is_close_material(&normalized, "jem") {
        return Ok(MaterialFamily::Jem);
    }

    Err(format!("noma'lum material: '{material}'"))
}

fn is_empty_material(material: &str) -> bool {
    let normalized = normalize_token(material);
    normalized.is_empty() || matches!(normalized.as_str(), "--" | "-" | "yoq" | "yuq")
}

fn split_materials(material: &str) -> Vec<&str> {
    material
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect()
}

fn is_close_material(value: &str, expected: &str) -> bool {
    if value == expected {
        return true;
    }
    if value.len() != expected.len() {
        return false;
    }
    levenshtein(value, expected) <= 1
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut costs: Vec<usize> = (0..=right.len()).collect();

    for (i, left_char) in left.chars().enumerate() {
        let mut previous = i;
        costs[0] = i + 1;

        for (j, right_char) in right.chars().enumerate() {
            let current = costs[j + 1];
            costs[j + 1] = if left_char == right_char {
                previous
            } else {
                1 + previous.min(current).min(costs[j])
            };
            previous = current;
        }
    }

    costs[right.len()]
}

fn should_process_file(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--file" | "--write-xlsx"))
        || args
            .first()
            .is_some_and(|arg| !arg.starts_with("--") && looks_like_table_file(arg))
}

fn looks_like_table_file(path: &str) -> bool {
    FileFormat::from_path(Path::new(path)).is_some()
}

fn process_file_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let input = read_optional_text(args, "--file")
        .or_else(|| args.first().filter(|arg| !arg.starts_with("--")).cloned())
        .ok_or("fayl yo'li berilmagan")?;
    let input_path = Path::new(&input);
    let input_format = FileFormat::from_path(input_path)
        .ok_or_else(|| format!("qo'llab-quvvatlanmaydigan fayl: {}", input_path.display()))?;
    let output = read_optional_text(args, "--out").unwrap_or_else(|| default_output_path(&input));
    let output_path = Path::new(&output);
    let output_format = FileFormat::from_path(output_path).ok_or_else(|| {
        format!(
            "output format qo'llab-quvvatlanmaydi: {}",
            output_path.display()
        )
    })?;

    if !args.iter().any(|arg| arg == "--out") && input_format != output_format {
        return Err(format!(
            "output formati input bilan bir xil bo'lishi kerak: input={:?}, output={:?}",
            input_format, output_format
        )
        .into());
    }

    if output_format == FileFormat::Xlsx || args.iter().any(|arg| arg == "--write-xlsx") {
        let report = write_results_to_xlsx(input_path, output_path)?;

        println!("Hisoblandi: {}", report.processed_count);
        println!("OK: {}", report.ok_count);
        println!("Xato: {}", report.error_count);
        println!("Output: {output}");

        return Ok(());
    }

    let rows = read_table(input_path)?;
    let report = calculate_table(&rows)?;
    write_report_delimited(output_path, &report, output_format.delimiter()?)?;

    println!("Hisoblandi: {}", report.processed_count);
    println!("OK: {}", report.ok_count);
    println!("Xato: {}", report.error_count);
    println!("Output: {output}");

    Ok(())
}

fn default_output_path(input: &str) -> String {
    let path = Path::new(input);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("csv");

    parent
        .join(format!("{stem}_hisoblangan.{extension}"))
        .to_string_lossy()
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileFormat {
    Xlsx,
    Xlsm,
    Xls,
    Ods,
    Csv,
    Tsv,
    Html,
    Pdf,
}

impl FileFormat {
    fn from_path(path: &Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_lowercase)
            .as_deref()
        {
            Some("xlsx") => Some(Self::Xlsx),
            Some("xlsm") => Some(Self::Xlsm),
            Some("xls") => Some(Self::Xls),
            Some("ods") => Some(Self::Ods),
            Some("csv") => Some(Self::Csv),
            Some("tsv") => Some(Self::Tsv),
            Some("html" | "htm") => Some(Self::Html),
            Some("pdf") => Some(Self::Pdf),
            _ => None,
        }
    }

    fn delimiter(self) -> Result<u8, Box<dyn Error>> {
        match self {
            Self::Csv => Ok(b','),
            Self::Tsv => Ok(b'\t'),
            _ => Err(format!(
                "{self:?} formatini shu formatda qayta yozish hozircha qo'llab-quvvatlanmaydi"
            )
            .into()),
        }
    }
}

#[derive(Debug)]
struct TableReport {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    processed_count: usize,
    ok_count: usize,
    error_count: usize,
}

#[derive(Debug)]
struct SheetReport {
    header_index: usize,
    results: Vec<RowResult>,
    processed_count: usize,
    ok_count: usize,
    error_count: usize,
}

#[derive(Debug)]
struct RowResult {
    row_index: usize,
    value: Result<f64, String>,
}

#[derive(Debug, Clone, Copy)]
struct ColumnIndexes {
    kg: usize,
    razmer: usize,
    first_material: usize,
    first_micron: usize,
    second_material: usize,
    second_micron: usize,
}

fn read_table(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("csv") => read_csv(path),
        Some("tsv") => read_tsv(path),
        Some("xlsx" | "xlsm" | "xls" | "ods") => read_excel(path),
        _ => Err(format!("qo'llab-quvvatlanmaydigan fayl: {}", path.display()).into()),
    }
}

fn read_csv(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    read_delimited(path, b',')
}

fn read_tsv(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    read_delimited(path, b'\t')
}

fn read_delimited(path: &Path, delimiter: u8) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .flexible(true)
        .from_path(path)?;
    let mut rows = Vec::new();

    for record in reader.records() {
        rows.push(record?.iter().map(ToOwned::to_owned).collect());
    }

    Ok(rows)
}

fn read_excel(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or("Excel ichida sheet topilmadi")??;

    Ok(range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
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

fn calculate_table(rows: &[Vec<String>]) -> Result<TableReport, Box<dyn Error>> {
    let sheet_report = calculate_sheet(rows)?;
    let header_index = sheet_report.header_index;
    let mut headers = rows[header_index].clone();
    headers.extend([
        "HISOBLANGAN_UZUNLIK".to_string(),
        "STATUS".to_string(),
        "XATO".to_string(),
    ]);

    let mut output_rows = Vec::new();

    for row_result in &sheet_report.results {
        let row = &rows[row_result.row_index];
        let mut output_row = row.clone();
        output_row.resize(headers.len() - 3, String::new());

        match &row_result.value {
            Ok(length) => {
                output_row.push(format!("{length:.0}"));
                output_row.push("OK".to_string());
                output_row.push(String::new());
            }
            Err(message) => {
                output_row.push(String::new());
                output_row.push("XATO".to_string());
                output_row.push(message.clone());
            }
        }

        output_rows.push(output_row);
    }

    Ok(TableReport {
        headers,
        rows: output_rows,
        processed_count: sheet_report.processed_count,
        ok_count: sheet_report.ok_count,
        error_count: sheet_report.error_count,
    })
}

fn calculate_sheet(rows: &[Vec<String>]) -> Result<SheetReport, Box<dyn Error>> {
    let (header_index, indexes) = find_header_row(rows)
        .ok_or("kerakli ustunlar topilmadi: KG, RAZMER, 1 QAVAT, 1 MIKRON, 2 QAVAT, 2 MIKRON")?;
    let mut results = Vec::new();
    let mut ok_count = 0;
    let mut error_count = 0;

    for (row_index, row) in rows.iter().enumerate().skip(header_index + 1) {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }

        let value = match calculate_row(row, indexes) {
            Ok(result) => {
                ok_count += 1;
                Ok(result.rounded_length)
            }
            Err(message) => {
                error_count += 1;
                Err(message)
            }
        };

        results.push(RowResult { row_index, value });
    }

    Ok(SheetReport {
        header_index,
        processed_count: results.len(),
        results,
        ok_count,
        error_count,
    })
}

fn write_results_to_xlsx(input: &Path, output: &Path) -> Result<SheetReport, Box<dyn Error>> {
    if FileFormat::from_path(input) != Some(FileFormat::Xlsx)
        || FileFormat::from_path(output) != Some(FileFormat::Xlsx)
    {
        return Err("xlsx formatida yozish faqat .xlsx -> .xlsx uchun ishlaydi".into());
    }

    let rows = read_table(input)?;
    let sheet_report = calculate_sheet(&rows)?;
    let mut book = reader::xlsx::read(input)?;
    let worksheet = book.sheet_mut(0)?;

    let write_column = worksheet.highest_column() + 1;
    let header_row = sheet_report.header_index as u32 + 1;
    worksheet
        .cell_mut((write_column, header_row))
        .set_value("HISOBLANGAN_UZUNLIK");

    for row_result in &sheet_report.results {
        let excel_row = row_result.row_index as u32 + 1;
        match &row_result.value {
            Ok(length) => {
                worksheet
                    .cell_mut((write_column, excel_row))
                    .set_value(format!("{length:.0}"));
            }
            Err(message) => {
                worksheet
                    .cell_mut((write_column, excel_row))
                    .set_value(format!("XATO: {message}"));
            }
        }
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    writer::xlsx::write(&book, output)?;

    Ok(sheet_report)
}

fn calculate_row(row: &[String], indexes: ColumnIndexes) -> Result<ResultBreakdown, String> {
    let kg_text = get_cell(row, indexes.kg);
    let razmer_text = get_cell(row, indexes.razmer);
    let q1 = get_cell(row, indexes.first_material);
    let m1 = get_cell(row, indexes.first_micron);
    let q2 = get_cell(row, indexes.second_material);
    let m2 = get_cell(row, indexes.second_micron);

    let first_micron = if is_empty_material(q1) {
        0
    } else {
        parse_micron(m1)?
    };
    let second_micron = if is_empty_material(q2) {
        0
    } else {
        parse_micron(m2)?
    };

    let calculation = Calculation {
        kg: parse_decimal(kg_text).map_err(|message| format!("KG: {message}"))?,
        razmer_mm: parse_decimal(razmer_text).map_err(|message| format!("RAZMER: {message}"))?,
        first_layer: Layer {
            material: q1,
            micron_text: m1,
            micron: first_micron,
        },
        second_layer: Layer {
            material: q2,
            micron_text: m2,
            micron: second_micron,
        },
        waste_percent: 5.0,
        round_to: 500.0,
    };

    calculate(&calculation)
}

fn get_cell(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("")
}

fn find_header_row(rows: &[Vec<String>]) -> Option<(usize, ColumnIndexes)> {
    rows.iter()
        .take(30)
        .enumerate()
        .find_map(|(index, row)| find_columns(row).map(|columns| (index, columns)))
}

fn find_columns(row: &[String]) -> Option<ColumnIndexes> {
    Some(ColumnIndexes {
        kg: find_column(row, HeaderKind::Kg)?,
        razmer: find_column(row, HeaderKind::Razmer)?,
        first_material: find_column(row, HeaderKind::FirstMaterial)?,
        first_micron: find_column(row, HeaderKind::FirstMicron)?,
        second_material: find_column(row, HeaderKind::SecondMaterial)?,
        second_micron: find_column(row, HeaderKind::SecondMicron)?,
    })
}

#[derive(Clone, Copy)]
enum HeaderKind {
    Kg,
    Razmer,
    FirstMaterial,
    FirstMicron,
    SecondMaterial,
    SecondMicron,
}

fn find_column(row: &[String], kind: HeaderKind) -> Option<usize> {
    row.iter()
        .position(|cell| header_matches(&normalize_header(cell), kind))
}

fn normalize_header(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn header_matches(header: &str, kind: HeaderKind) -> bool {
    match kind {
        HeaderKind::Kg => matches!(header, "kg" | "kilo" | "ogirlik" | "weight"),
        HeaderKind::Razmer => matches!(header, "razmer" | "razmr" | "olcham" | "size"),
        HeaderKind::FirstMaterial => {
            matches!(
                header,
                "1qavat" | "1qatlam" | "qavat1" | "qatlam1" | "1layer"
            )
        }
        HeaderKind::FirstMicron => {
            matches!(
                header,
                "1mikron" | "1micron" | "1mkrn" | "mikron1" | "micron1"
            )
        }
        HeaderKind::SecondMaterial => {
            matches!(
                header,
                "2qavat" | "2qatlam" | "qavat2" | "qatlam2" | "2layer"
            )
        }
        HeaderKind::SecondMicron => {
            matches!(
                header,
                "2mikron" | "2micron" | "2mkrn" | "mikron2" | "micron2"
            )
        }
    }
}

fn parse_decimal(value: &str) -> Result<f64, String> {
    let mut normalized = value.trim().replace(' ', "");
    if normalized.contains(',') && !normalized.contains('.') {
        normalized = normalized.replace(',', ".");
    } else {
        normalized = normalized.replace(',', "");
    }

    normalized
        .parse::<f64>()
        .map_err(|_| format!("raqam noto'g'ri: '{value}'"))
}

fn write_report_delimited(
    path: &Path,
    report: &TableReport,
    delimiter: u8,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_path(path)?;
    writer.write_record(&report.headers)?;
    for row in &report.rows {
        writer.write_record(row)?;
    }
    writer.flush()?;

    Ok(())
}

fn mcp_cpp_coefficient(micron: u32) -> Option<f64> {
    interpolate(
        micron,
        &[
            (20, 1.07),
            (25, 1.3),
            (30, 1.6),
            (35, 2.0),
            (40, 2.15),
            (45, 2.7),
            (50, 2.8),
            (60, 3.2),
        ],
    )
}

fn coefficient_error(material: &str, micron: u32, family: MaterialFamily) -> String {
    let available = match family {
        MaterialFamily::FirstLayer | MaterialFamily::McpCpp => "20, 25, 30, 35, 40, 45, 50, 60",
        MaterialFamily::Jem => "25, 30",
        MaterialFamily::Pe => "30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90",
        MaterialFamily::Twist => "twist/tuisim uchun mikron jadvali ishlatilmaydi",
        MaterialFamily::Empty => "bo'sh material",
    };

    format!(
        "'{}' materiali uchun {} mikron jadvalda topilmadi. Bu material oilasida bor mikronlar: {}",
        material, micron, available
    )
}

fn jem_coefficient(micron: u32) -> Option<f64> {
    interpolate(micron, &[(25, 1.0), (30, 1.5)])
}

fn pe_coefficient(micron: u32) -> Option<f64> {
    interpolate(
        micron,
        &[
            (30, 2.0),
            (35, 2.3),
            (40, 2.6),
            (45, 3.0),
            (50, 3.3),
            (55, 3.6),
            (60, 4.0),
            (65, 4.3),
            (70, 4.6),
            (75, 5.0),
            (80, 5.3),
            (85, 5.6),
            (90, 6.0),
        ],
    )
}

fn interpolate(micron: u32, table: &[(u32, f64)]) -> Option<f64> {
    for window in table.windows(2) {
        let (left_micron, left_value) = window[0];
        let (right_micron, right_value) = window[1];
        if micron == left_micron {
            return Some(left_value);
        }
        if micron > left_micron && micron < right_micron {
            let ratio = (micron - left_micron) as f64 / (right_micron - left_micron) as f64;
            return Some(left_value + (right_value - left_value) * ratio);
        }
    }
    table
        .last()
        .and_then(|(table_micron, value)| (*table_micron == micron).then_some(*value))
}

fn round_up(value: f64, step: f64) -> f64 {
    (value / step).ceil() * step
}

fn run_demo_rows() {
    let rows = [
        ("1178", 300.0, 530.0, "pet", "12", "pe pr", "30"),
        ("1179", 600.0, 715.0, "mat", "20", "pe oq", "55"),
        ("1180", 600.0, 755.0, "mat", "20", "pe oq", "55"),
        ("1181", 600.0, 595.0, "mat", "20", "pe oq", "55"),
        ("1182", 600.0, 755.0, "mat", "20", "pe oq", "55"),
        ("1183", 1500.0, 405.0, "mat", "20", "cpp", "45"),
        ("1184", 1500.0, 405.0, "mat", "20", "cpp", "45"),
        ("1185", 3000.0, 475.0, "tuisim", "23", "--", "--"),
        ("1186", 3000.0, 1055.0, "opp", "18/20", "oppm", "25/30"),
        ("1187", 3000.0, 585.0, "opp", "18", "oppm", "25/30"),
        ("1188", 3000.0, 565.0, "opp", "18", "oppm", "20"),
        ("1189", 2000.0, 565.0, "opp", "18", "oppm", "20"),
        ("1190", 2000.0, 565.0, "opp", "18", "oppm", "20"),
        ("1191", 300.0, 815.0, "pet", "12", "pe pr", "65"),
        ("1192", 400.0, 815.0, "pet", "12", "pe pr", "65"),
        ("1193", 1000.0, 975.0, "pet", "12", "cpp", "35"),
        ("1194", 600.0, 765.0, "pet", "12", "cpp", "20"),
        ("1207", 3000.0, 635.0, "pet", "12", "oppm/pe pr", "20/30"),
    ];

    println!("KOD\tKG\tRAZMER\t1Q\t1M\t2Q\t2M\tUZUNLIK");

    for (code, kg, razmer_mm, q1, m1, q2, m2) in rows {
        let first_micron = match parse_micron(m1) {
            Ok(micron) => micron,
            Err(message) => {
                println!("{code}\tXATO: {message}");
                continue;
            }
        };
        let second_micron = if is_empty_material(q2) {
            0
        } else {
            match parse_micron(m2) {
                Ok(micron) => micron,
                Err(message) => {
                    println!("{code}\tXATO: {message}");
                    continue;
                }
            }
        };

        let calculation = Calculation {
            kg,
            razmer_mm,
            first_layer: Layer {
                material: q1,
                micron_text: m1,
                micron: first_micron,
            },
            second_layer: Layer {
                material: q2,
                micron_text: m2,
                micron: second_micron,
            },
            waste_percent: 5.0,
            round_to: 500.0,
        };

        match calculate(&calculation) {
            Ok(result) => println!(
                "{code}\t{kg:.0}\t{razmer_mm:.0}\t{q1}\t{m1}\t{q2}\t{m2}\t{:.0}",
                result.rounded_length
            ),
            Err(message) => println!("{code}\tXATO: {message}"),
        }
    }
}

fn read_text(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("{name} berilmagan"))?;

    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{name} uchun qiymat berilmagan"))
}

fn read_number(args: &[String], name: &str) -> Result<f64, String> {
    read_text(args, name)?
        .parse::<f64>()
        .map_err(|_| format!("{name} raqam bo'lishi kerak"))
}

fn read_optional_number(args: &[String], name: &str) -> Result<Option<f64>, String> {
    if args.iter().any(|arg| arg == name) {
        read_number(args, name).map(Some)
    } else {
        Ok(None)
    }
}

fn read_optional_text(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1).cloned()
}

fn parse_micron(value: &str) -> Result<u32, String> {
    let normalized = value.trim();
    if normalized == "--" || normalized.is_empty() {
        return Err(format!("micron qiymati noto'g'ri: '{value}'"));
    }

    normalized
        .split('/')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| format!("micron butun raqam bo'lishi kerak: '{value}'"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| format!("micron qiymati noto'g'ri: '{value}'"))
}

fn parse_micron_parts(value: &str) -> Result<Vec<u32>, String> {
    let normalized = value.trim();
    if normalized == "--" || normalized.is_empty() {
        return Err(format!("micron qiymati noto'g'ri: '{value}'"));
    }

    normalized
        .split('/')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| format!("micron butun raqam bo'lishi kerak: '{value}'"))
        })
        .collect()
}

fn print_result(calculation: &Calculation<'_>, result: &ResultBreakdown) {
    println!("Hisob:");
    println!(
        "1-qavat: {} {} micron => {}",
        calculation.first_layer.material, calculation.first_layer.micron_text, result.first_coeff
    );
    println!(
        "2-qavat: {} {} micron => {}",
        calculation.second_layer.material,
        calculation.second_layer.micron_text,
        result.second_coeff
    );
    println!(
        "koeffitsient: {} + {} = {}",
        result.first_coeff, result.second_coeff, result.coeff_sum
    );
    println!(
        "razmer: {} mm = {} sm",
        calculation.razmer_mm, result.razmer_sm
    );
    println!(
        "{} x {} = {}",
        result.coeff_sum, result.razmer_sm, result.density_part
    );
    println!(
        "{} / {} x 6000 = {:.0}",
        calculation.kg, result.density_part, result.base_length
    );
    println!(
        "{:.0} x {}% = {:.0} atxod",
        result.base_length, calculation.waste_percent, result.waste_length
    );
    println!(
        "{:.0} + {:.0} = {:.0}",
        result.base_length, result.waste_length, result.length_with_waste
    );
    println!("yakuniy uzunlik: {:.0}", result.rounded_length);
}

fn print_usage() {
    eprintln!("Ishlatish:");
    eprintln!("  cargo run");
    eprintln!("  cargo run -- --interactive");
    eprintln!("  cargo run -- --demo");
    eprintln!("  cargo run -- --file ish.xlsx");
    eprintln!("  cargo run -- --file ish.xlsx --out natija.xlsx");
    eprintln!("  cargo run -- --file ish.csv");
    eprintln!("  cargo run -- --file ish.tsv");
    eprintln!("  cargo run -- examples/sample.csv");
    eprintln!("  cargo run -- --kg 300 --razmer 530 --q1 pet --m1 12 --q2 \"pe pr\" --m2 30");
    eprintln!("  cargo run -- --kg 3000 --razmer 635 --q1 pet --m1 12 --q2 oppm --m2 20 --q3 \"pe pr\" --m3 30");
    eprintln!("  cargo run -- --kg 300 --razmer 530 --q1 pet --m1 12 --q2 \"pe pr\" --m2 30 --waste 5 --round 500");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_row_1178() {
        let result = calculate(&example_1178()).unwrap();

        assert_eq!(result.first_coeff, 1.0);
        assert_eq!(result.second_coeff, 2.0);
        assert_eq!(result.coeff_sum, 3.0);
        assert_eq!(result.razmer_sm, 53.0);
        assert!((result.base_length - 11320.7547).abs() < 0.001);
        assert_eq!(result.rounded_length, 12000.0);
    }

    #[test]
    fn maps_pet_20_or_more_to_mcp_cpp_table() {
        let layer = Layer {
            material: "pet",
            micron_text: "30",
            micron: 30,
        };

        assert_eq!(coefficient_cell(&layer, true).unwrap(), 1.6);
    }

    #[test]
    fn maps_pe_family_by_prefix() {
        let layer = Layer {
            material: "pe pr",
            micron_text: "30",
            micron: 30,
        };

        assert_eq!(coefficient_cell(&layer, false).unwrap(), 2.0);
    }

    #[test]
    fn calculates_twist_without_second_layer() {
        let calculation = Calculation {
            kg: 3000.0,
            razmer_mm: 475.0,
            first_layer: Layer {
                material: "tuisim",
                micron_text: "23",
                micron: 23,
            },
            second_layer: Layer {
                material: "--",
                micron_text: "--",
                micron: 0,
            },
            waste_percent: 5.0,
            round_to: 500.0,
        };

        let result = calculate(&calculation).unwrap();

        assert_eq!(result.first_coeff, 2.0);
        assert_eq!(result.second_coeff, 0.0);
        assert_eq!(result.rounded_length, 199000.0);
    }

    #[test]
    fn calculates_three_layer_cell_from_second_layer_slash() {
        let calculation = Calculation {
            kg: 3000.0,
            razmer_mm: 635.0,
            first_layer: Layer {
                material: "pet",
                micron_text: "12",
                micron: 12,
            },
            second_layer: Layer {
                material: "oppm/pe pr",
                micron_text: "20/30",
                micron: 30,
            },
            waste_percent: 5.0,
            round_to: 500.0,
        };

        let result = calculate(&calculation).unwrap();

        assert_eq!(result.first_coeff, 1.0);
        assert_eq!(result.second_coeff, 3.0);
        assert_eq!(result.rounded_length, 74500.0);
    }

    #[test]
    fn tolerates_simple_material_typos() {
        assert_eq!(material_family("pett").unwrap(), MaterialFamily::FirstLayer);
        assert_eq!(material_family("map").unwrap(), MaterialFamily::McpCpp);
        assert_eq!(material_family("twism").unwrap(), MaterialFamily::Twist);
        assert_eq!(material_family("pff").unwrap(), MaterialFamily::FirstLayer);
        assert_eq!(material_family("mpet").unwrap(), MaterialFamily::FirstLayer);
        assert_eq!(material_family("PE PR").unwrap(), MaterialFamily::Pe);
    }

    #[test]
    fn does_not_confuse_pe_with_pet() {
        assert_eq!(material_family("pe").unwrap(), MaterialFamily::Pe);
    }

    #[test]
    fn calculates_with_empty_first_layer() {
        let calculation = Calculation {
            kg: 150.0,
            razmer_mm: 850.0,
            first_layer: Layer {
                material: "--",
                micron_text: "--",
                micron: 0,
            },
            second_layer: Layer {
                material: "pe pr",
                micron_text: "60",
                micron: 60,
            },
            waste_percent: 5.0,
            round_to: 500.0,
        };

        let result = calculate(&calculation).unwrap();

        assert_eq!(result.first_coeff, 0.0);
        assert_eq!(result.second_coeff, 4.0);
        assert_eq!(result.rounded_length, 3000.0);
    }

    #[test]
    fn maps_petm_and_mpet_12_to_one() {
        assert_eq!(coefficient_single("petm", 12, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("mpet", 12, false).unwrap(), 1.0);
    }

    #[test]
    fn accepts_new_formula_aliases_and_interpolates() {
        assert_eq!(material_family("st01").unwrap(), MaterialFamily::FirstLayer);
        assert_eq!(material_family("twisjem").unwrap(), MaterialFamily::Twist);
        assert_eq!(material_family("pf").unwrap(), MaterialFamily::FirstLayer);
        assert_eq!(coefficient_single("opp", 18, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("oppm", 12, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("cpp", 55, false).unwrap(), 3.0);
        let mcp_23 = coefficient_single("mcp", 23, false).unwrap();
        assert!((mcp_23 - 1.208).abs() < 0.001);
    }

    #[test]
    fn merges_optional_third_layer_for_cli_and_interactive() {
        let (material, micron) = merge_optional_third_layer(
            "oppm".to_string(),
            "20".to_string(),
            "pe pr".to_string(),
            "30".to_string(),
        )
        .unwrap();

        assert_eq!(material, "oppm/pe pr");
        assert_eq!(micron, "20/30");
    }
}
