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
    calculate_order_variants(order)?
        .into_iter()
        .next()
        .ok_or_else(|| "hisob varianti topilmadi".to_string())
}

pub fn calculate_order_lengths(order: &OrderDraft) -> Result<Vec<f64>, String> {
    calculate_order_variants(order).map(|results| {
        results
            .into_iter()
            .map(|result| result.rounded_length)
            .collect()
    })
}

fn calculate_order_variants(order: &OrderDraft) -> Result<Vec<CalcResult>, String> {
    let mut results = Vec::new();
    for order in order_variants(order) {
        results.push(calculate_single_order(&order)?);
    }
    Ok(results)
}

fn calculate_single_order(order: &OrderDraft) -> Result<CalcResult, String> {
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
    let first_empty = is_empty_material(&q1);
    let first_micron = if first_empty { 0 } else { parse_micron(&m1)? };
    let other_micron = if is_empty_material(&q_other) {
        0
    } else {
        parse_micron(&m_other)?
    };

    let first_coeff = if first_empty {
        0.0
    } else {
        coefficient_cell(&q1, &m1, first_micron, true)?
    };
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

fn order_variants(order: &OrderDraft) -> Vec<OrderDraft> {
    let first_materials = alternatives(order.first_material.as_deref());
    let second_materials = alternatives(order.second_material.as_deref());
    let third_materials = alternatives(order.third_material.as_deref());
    let mut variants = Vec::new();

    for first_material in &first_materials {
        for second_material in &second_materials {
            for third_material in &third_materials {
                let mut variant = order.clone();
                variant.first_material = first_material.clone();
                variant.second_material = second_material.clone();
                variant.third_material = third_material.clone();
                variants.push(variant);
            }
        }
    }
    variants
}

fn alternatives(value: Option<&str>) -> Vec<Option<String>> {
    let Some(value) = value else {
        return vec![None];
    };
    let parts = value
        .split("yoki")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| Some(part.to_string()))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        vec![Some(value.to_string())]
    } else {
        parts
    }
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
    if is_first && !matches!(family, Family::Empty | Family::Twist) && micron <= 20 {
        return Ok(1.0);
    }
    if family == Family::First && micron <= 20 {
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
    if n.starts_with("twis") || n.starts_with("tuisim") {
        return Ok(Family::Twist);
    }
    if n.starts_with("pet") || n.starts_with("mpet") || close(&n, "pet") {
        return Ok(Family::First);
    }
    if n.starts_with("opp") || n.starts_with("popp") || n == "st01" || close(&n, "opp") {
        return Ok(Family::First);
    }
    if matches!(n.as_str(), "map" | "mcpp" | "msr" | "msp") {
        return Ok(Family::McpCpp);
    }
    if n.starts_with("mat") || n.starts_with("pff") || n.starts_with("pf") || close(&n, "mat") {
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

fn jem(micron: u32) -> Option<f64> {
    interpolate(micron, &[(25, 1.0), (30, 1.5)])
}

fn pe(micron: u32) -> Option<f64> {
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
    n.is_empty() || n.chars().all(|ch| ch == '-') || matches!(n.as_str(), "yoq" | "yuq")
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

#[cfg(test)]
mod tests {
    use super::{calculate_order, calculate_order_lengths, coefficient_single};
    use crate::order::OrderDraft;

    #[test]
    fn calculates_with_empty_first_layer() {
        let order = OrderDraft {
            kg: Some(150.0),
            width_mm: Some(810.0),
            first_material: Some("--".to_string()),
            first_micron: Some("--".to_string()),
            second_material: Some("pe".to_string()),
            second_micron: Some("55/60".to_string()),
            ..OrderDraft::default()
        };

        assert_eq!(calculate_order(&order).unwrap().rounded_length, 3000.0);
    }

    #[test]
    fn accepts_new_material_aliases_and_low_microns() {
        assert_eq!(coefficient_single("st01", 18, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("opp", 18, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("pf", 18, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("oppm", 12, false).unwrap(), 1.0);
        assert_eq!(coefficient_single("twisjem", 40, false).unwrap(), 2.0);
    }

    #[test]
    fn interpolates_missing_microns_inside_table() {
        assert_eq!(coefficient_single("cpp", 55, false).unwrap(), 3.0);
        let mcp_23 = coefficient_single("mcp", 23, false).unwrap();
        assert!((mcp_23 - 1.208).abs() < 0.001);
    }

    #[test]
    fn calculates_alternative_materials() {
        let order = OrderDraft {
            kg: Some(300.0),
            width_mm: Some(530.0),
            first_material: Some("pet".to_string()),
            first_micron: Some("12".to_string()),
            second_material: Some("pe oq yoki mcp".to_string()),
            second_micron: Some("30".to_string()),
            ..OrderDraft::default()
        };

        let lengths = calculate_order_lengths(&order).unwrap();
        assert_eq!(lengths, vec![12000.0, 14000.0]);
    }
}
