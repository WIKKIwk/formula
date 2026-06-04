pub fn parse_material_layers(value: &str) -> Vec<(String, String)> {
    value
        .split('+')
        .filter_map(parse_material_layer)
        .take(3)
        .collect()
}

fn parse_material_layer(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    let micron_start = value
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, _)| index)?;
    let mut start = micron_start;
    for (index, ch) in value[..micron_start].char_indices().rev() {
        if ch.is_ascii_digit() || matches!(ch, '/' | ',' | '.') {
            start = index;
        } else {
            break;
        }
    }

    let material = value[..start].trim();
    let micron = value[start..]
        .chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == '/')
        .collect::<String>();
    if material.is_empty() || micron.is_empty() {
        return None;
    }
    Some((normalize_material_name(material), micron))
}

fn normalize_material_name(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    let compact = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.contains("metall") && compact.contains("bopp") {
        return "oppm".to_string();
    }
    if compact == "bopp" {
        return "opp".to_string();
    }
    if compact.starts_with("bopp ") {
        return compact.replacen("bopp", "opp", 1);
    }
    compact
}

#[cfg(test)]
mod tests {
    use super::parse_material_layers;

    #[test]
    fn parses_two_layer_material_text() {
        let layers = parse_material_layers("pet 20 + Metall BOPP 30");
        assert_eq!(
            layers,
            vec![
                ("pet".to_string(), "20".to_string()),
                ("oppm".to_string(), "30".to_string())
            ]
        );
    }

    #[test]
    fn keeps_slash_layer_together() {
        let layers = parse_material_layers("pet 12 + oppm/pe pr 20/30");
        assert_eq!(
            layers,
            vec![
                ("pet".to_string(), "12".to_string()),
                ("oppm/pe pr".to_string(), "20/30".to_string())
            ]
        );
    }
}
