use std::env;
use std::process;

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
    let second_micron = if is_empty_material(&second_material) {
        0
    } else {
        read_micron(args, "--m2")?
    };
    let second_micron_text = if is_empty_material(&second_material) {
        "--".to_string()
    } else {
        read_text(args, "--m2")?
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
    let first_coeff = coefficient_cell(&calculation.first_layer, true)?;
    let second_coeff =
        if first_family == MaterialFamily::Twist && second_family == MaterialFamily::Empty {
            0.0
        } else {
            coefficient_cell(&calculation.second_layer, false)?
        };
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

    if is_first_layer && family == MaterialFamily::FirstLayer && micron <= 20 {
        return Ok(1.0);
    }

    match family {
        MaterialFamily::FirstLayer | MaterialFamily::McpCpp => mcp_cpp_coefficient(micron),
        MaterialFamily::Jem => jem_coefficient(micron),
        MaterialFamily::Pe => pe_coefficient(micron),
        MaterialFamily::Twist => Some(2.0),
        MaterialFamily::Empty => None,
    }
    .ok_or_else(|| {
        format!(
            "'{}' materiali uchun {} mikron jadvalda topilmadi",
            material, micron
        )
    })
}

fn material_family(material: &str) -> Result<MaterialFamily, String> {
    let normalized = material.trim().to_lowercase();

    if normalized == "--" || normalized.is_empty() {
        return Ok(MaterialFamily::Empty);
    }
    if normalized.starts_with("twist") || normalized.starts_with("tuisim") {
        return Ok(MaterialFamily::Twist);
    }
    if normalized.starts_with("pet")
        || normalized.starts_with("opp")
        || normalized.starts_with("popp")
        || normalized.starts_with("mat")
    {
        return Ok(MaterialFamily::FirstLayer);
    }
    if normalized.starts_with("pe") {
        return Ok(MaterialFamily::Pe);
    }
    if normalized.starts_with("cpp") || normalized.starts_with("mcp") {
        return Ok(MaterialFamily::McpCpp);
    }
    if normalized.starts_with("jem") {
        return Ok(MaterialFamily::Jem);
    }

    Err(format!("noma'lum material: '{material}'"))
}

fn is_empty_material(material: &str) -> bool {
    let normalized = material.trim();
    normalized.is_empty() || normalized == "--"
}

fn split_materials(material: &str) -> Vec<&str> {
    material
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn mcp_cpp_coefficient(micron: u32) -> Option<f64> {
    match micron {
        20 => Some(1.07),
        25 => Some(1.3),
        30 => Some(1.6),
        35 => Some(2.0),
        40 => Some(2.15),
        45 => Some(2.7),
        50 => Some(2.8),
        60 => Some(3.2),
        _ => None,
    }
}

fn jem_coefficient(micron: u32) -> Option<f64> {
    match micron {
        25 => Some(1.0),
        30 => Some(1.5),
        _ => None,
    }
}

fn pe_coefficient(micron: u32) -> Option<f64> {
    match micron {
        30 => Some(2.0),
        35 => Some(2.3),
        40 => Some(2.6),
        45 => Some(3.0),
        50 => Some(3.3),
        55 => Some(3.6),
        60 => Some(4.0),
        65 => Some(4.3),
        70 => Some(4.6),
        75 => Some(5.0),
        80 => Some(5.3),
        85 => Some(5.6),
        90 => Some(6.0),
        _ => None,
    }
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

fn read_micron(args: &[String], name: &str) -> Result<u32, String> {
    let value = read_text(args, name)?;
    parse_micron(&value)
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
    eprintln!("  cargo run -- --demo");
    eprintln!("  cargo run -- --kg 300 --razmer 530 --q1 pet --m1 12 --q2 \"pe pr\" --m2 30");
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
        assert_eq!(result.second_coeff, 3.0700000000000003);
        assert_eq!(result.rounded_length, 73500.0);
    }
}
