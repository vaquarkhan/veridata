//! Policy helpers shared by engine and connectors.

/// Content fields after applying AC-A6 exclude list.
pub fn effective_content_fields(content_fields: &[String], exclude_fields: &[String]) -> Vec<String> {
    content_fields
        .iter()
        .filter(|f| !exclude_fields.contains(f))
        .cloned()
        .collect()
}

/// Parse `late_arrival_window` (e.g. `900s`, `PT15M`, plain seconds) to seconds.
pub fn late_arrival_window_secs(window: &str) -> Option<u64> {
    let w = window.trim();
    if let Some(s) = w.strip_suffix('s') {
        return s.parse().ok();
    }
    if let Ok(n) = w.parse::<u64>() {
        return Some(n);
    }
    if let Some(m) = w.strip_prefix("PT").and_then(|r| r.strip_suffix('M')) {
        return m.parse::<u64>().ok().map(|m| m * 60);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_a6_1_exclude_fields_removed() {
        let fields = vec!["a".into(), "b".into(), "c".into()];
        let out = effective_content_fields(&fields, &["b".into()]);
        assert_eq!(out, vec!["a", "c"]);
    }

    #[test]
    fn ac_b8_1_late_arrival_window_parses() {
        assert_eq!(late_arrival_window_secs("900s"), Some(900));
        assert_eq!(late_arrival_window_secs("120"), Some(120));
    }
}
