use crate::order::OrderDraft;

#[derive(Debug, Clone)]
pub struct CalcResult {
    pub first_coeff: f64,
    pub other_coeff: f64,
    pub coeff_sum: f64,
    pub width_sm: f64,
    pub base_length: f64,
    pub waste_length: f64,
    pub rounded_length: f64,
}

pub fn calculate_order(order: &OrderDraft) -> Result<CalcResult, String> {
    let kg = OrderDraft::require_number(order.kg, "KG")?;
    let width_mm = OrderDraft::require_number(order.width_mm, "RAZMER")?;
    let q1 = OrderDraft::require_text(&order.first_material, "1-qavat")?;
    let m1 = OrderDraft::require_text(&order.first_micron, "1-mikron")?;
    let q2 = order.second_material.clone().unwrap_or_default();
    let m2 = order
        .second_micron
        .clone()
        .unwrap_or_else(|| "--".to_string());
    let q3 = order.third_material.clone().unwrap_or_default();
    let m3 = order.third_micron.clone().unwrap_or_default();
    let (q_other, m_other) = merge_layers(q2, m2, q3, m3)?;
    let first_micron = parse_micron(&m1)?;
    let other_micron = if is_empty_material(&q_other) {
        0
    } else {
        parse_micron(&m_other)?
    };

    let first_coeff = coefficient_cell(&q1, &m1, first_micron, true)?;
    let other_coeff = if is_empty_material(&q_other) {
        0.0
    } else {
        coefficient_cell(&q_other, &m_other, other_micron, false)?
    };
    let coeff_sum = first_coeff + other_coeff;
    if coeff_sum <= 0.0 {
        return Err("kamida bitta qavat materiali kerak".to_string());
    }

    let width_sm = width_mm / 10.0;
    let base_length = kg / (coeff_sum * width_sm) * 6000.0;
    let waste_length = base_length * 0.05;
    let rounded_length = round_up(base_length + waste_length, 500.0);

    Ok(CalcResult {
        first_coeff,
        other_coeff,
        coeff_sum,
        width_sm,
        base_length,
        waste_length,
        rounded_length,
    })
}

fn merge_layers(
    q2: String,
    m2: String,
    q3: String,
    m3: String,
) -> Result<(String, String), String> {
    let q2_empty = is_empty_material(&q2);
    let q3_empty = is_empty_material(&q3);
    match (q2_empty, q3_empty) {
        (true, true) => Ok(("--".to_string(), "--".to_string())),
        (true, false) => Ok((q3, m3)),
        (false, true) => Ok((q2, m2)),
        (false, false) => {
            if m3.trim().is_empty() {
                return Err("3-qavat mikroni berilmagan".to_string());
            }
            Ok((format!("{q2}/{q3}"), format!("{m2}/{m3}")))
        }
    }
}

fn coefficient_cell(
    material: &str,
    micron_text: &str,
    micron: u32,
    is_first: bool,
) -> Result<f64, String> {
    let materials = split_parts(material);
    let microns = parse_micron_parts(micron_text)?;
    if materials.len() == 1 {
        return coefficient_single(materials[0], micron, is_first);
    }
    if materials.len() != microns.len() {
        return Err(format!(
            "material/mikron mos emas: {material} / {micron_text}"
        ));
    }
    materials
        .iter()
        .zip(microns)
        .map(|(material, micron)| coefficient_single(material, micron, is_first))
        .sum()
}

fn coefficient_single(material: &str, micron: u32, is_first: bool) -> Result<f64, String> {
    let family = material_family(material)?;
    let normalized = normalize(material);
    if is_first && !matches!(family, Family::Empty | Family::Twist) && micron <= 20 {
        return Ok(1.0);
    }
    if family == Family::First
        && (normalized.starts_with("pet") || normalized.starts_with("mpet"))
        && micron <= 20
    {
        return Ok(1.0);
    }

    let value = match family {
        Family::First | Family::McpCpp => mcp_cpp(micron),
        Family::Jem => jem(micron),
        Family::Pe => pe(micron),
        Family::Twist => Some(2.0),
        Family::Empty => None,
    };
    value.ok_or_else(|| coefficient_error(material, micron, family))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    First,
    McpCpp,
    Jem,
    Pe,
    Twist,
    Empty,
}

fn material_family(material: &str) -> Result<Family, String> {
    let n = normalize(material);
    if n.is_empty() || matches!(n.as_str(), "--" | "-" | "yoq" | "yuq") {
        return Ok(Family::Empty);
    }
    if n.starts_with("twist") || n.starts_with("tuisim") || n.starts_with("twism") {
        return Ok(Family::Twist);
    }
    if n.starts_with("pet") || n.starts_with("mpet") || close(&n, "pet") {
        return Ok(Family::First);
    }
    if n.starts_with("opp") || n.starts_with("popp") || close(&n, "opp") {
        return Ok(Family::First);
    }
    if matches!(n.as_str(), "map" | "mcpp" | "msr" | "msp") {
        return Ok(Family::McpCpp);
    }
    if n.starts_with("mat") || n.starts_with("pff") || close(&n, "mat") {
        return Ok(Family::First);
    }
    if n.starts_with("pe") || close(&n, "pe") {
        return Ok(Family::Pe);
    }
    if n.starts_with("cpp") || n.starts_with("mcp") || close(&n, "cpp") || close(&n, "mcp") {
        return Ok(Family::McpCpp);
    }
    if n.starts_with("jem") || close(&n, "jem") {
        return Ok(Family::Jem);
    }
    Err(format!("noma'lum material: {material}"))
}

fn mcp_cpp(micron: u32) -> Option<f64> {
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

fn jem(micron: u32) -> Option<f64> {
    match micron {
        25 => Some(1.0),
        30 => Some(1.5),
        _ => None,
    }
}

fn pe(micron: u32) -> Option<f64> {
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

fn parse_micron(value: &str) -> Result<u32, String> {
    parse_micron_parts(value)?
        .into_iter()
        .max()
        .ok_or_else(|| format!("micron noto'g'ri: {value}"))
}

fn parse_micron_parts(value: &str) -> Result<Vec<u32>, String> {
    let value = value.trim();
    if value.is_empty() || value == "--" {
        return Err(format!("micron noto'g'ri: {value}"));
    }
    value
        .split('/')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| format!("micron noto'g'ri: {value}"))
        })
        .collect()
}

fn is_empty_material(material: &str) -> bool {
    let n = normalize(material);
    n.is_empty() || matches!(n.as_str(), "--" | "-" | "yoq" | "yuq")
}

fn split_parts(value: &str) -> Vec<&str> {
    value
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect()
}

fn close(value: &str, expected: &str) -> bool {
    value == expected || (value.len() == expected.len() && levenshtein(value, expected) <= 1)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut costs: Vec<usize> = (0..=right.len()).collect();
    for (i, lc) in left.chars().enumerate() {
        let mut previous = i;
        costs[0] = i + 1;
        for (j, rc) in right.chars().enumerate() {
            let current = costs[j + 1];
            costs[j + 1] = if lc == rc {
                previous
            } else {
                1 + previous.min(current).min(costs[j])
            };
            previous = current;
        }
    }
    costs[right.len()]
}

fn coefficient_error(material: &str, micron: u32, family: Family) -> String {
    let available = match family {
        Family::First | Family::McpCpp => "20, 25, 30, 35, 40, 45, 50, 60",
        Family::Jem => "25, 30",
        Family::Pe => "30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90",
        Family::Twist => "twist uchun jadval kerak emas",
        Family::Empty => "bo'sh material",
    };
    format!("'{material}' uchun {micron} mikron topilmadi. Bor mikronlar: {available}")
}

fn round_up(value: f64, step: f64) -> f64 {
    (value / step).ceil() * step
}
