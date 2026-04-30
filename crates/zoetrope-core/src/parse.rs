/// Parse a time string into seconds. Accepts `SS`, `MM:SS`, `HH:MM:SS`, with
/// an optional trailing `s` suffix on plain seconds (`5s`, `1.25s`).
pub fn parse_time(s: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    let without_suffix = trimmed.strip_suffix('s').unwrap_or(trimmed);

    if without_suffix.is_empty() {
        return Err("empty".into());
    }

    let parts: Vec<&str> = without_suffix.split(':').collect();
    let parse = |x: &str| -> Result<f64, String> { x.parse::<f64>().map_err(|e| e.to_string()) };

    let seconds = match parts.as_slice() {
        [s] => parse(s)?,
        [m, s] => parse(m)? * 60.0 + parse(s)?,
        [h, m, s] => parse(h)? * 3600.0 + parse(m)? * 60.0 + parse(s)?,
        _ => return Err("expected SS, MM:SS, or HH:MM:SS".into()),
    };

    if !seconds.is_finite() || seconds < 0.0 {
        return Err("must be non-negative".into());
    }
    Ok(seconds)
}

/// Parse a human-readable size like `5mb`, `500kb`, `2GB`, or a raw byte count.
/// Units are decimal (1 kb = 1,000 bytes, 1 mb = 1,000,000 bytes) to match how
/// GitHub, Slack, and Discord document their upload limits.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty".into());
    }

    let (num_part, multiplier) = split_size_suffix(trimmed)?;
    if num_part.is_empty() {
        return Err("missing number".into());
    }

    let value: f64 = num_part
        .parse()
        .map_err(|_| format!("not a number: {num_part}"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err("must be positive".into());
    }

    Ok((value * multiplier as f64).round() as u64)
}

fn split_size_suffix(s: &str) -> Result<(&str, u64), String> {
    let lower_end = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &s[lower_end.len()..];
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1_000,
        "m" | "mb" => 1_000_000,
        "g" | "gb" => 1_000_000_000,
        other => {
            return Err(format!(
                "unknown size suffix \"{other}\" (expected b, kb, mb, gb)"
            ))
        }
    };
    Ok((lower_end.trim(), multiplier))
}

#[cfg(test)]
mod tests {
    use super::{parse_size, parse_time};

    #[test]
    fn parse_time_seconds_plain() {
        assert_eq!(parse_time("5").unwrap(), 5.0);
        assert_eq!(parse_time("5s").unwrap(), 5.0);
        assert_eq!(parse_time("0.5").unwrap(), 0.5);
        assert_eq!(parse_time("1.25s").unwrap(), 1.25);
    }

    #[test]
    fn parse_time_mm_ss() {
        assert_eq!(parse_time("1:30").unwrap(), 90.0);
        assert_eq!(parse_time("0:05").unwrap(), 5.0);
    }

    #[test]
    fn parse_time_hh_mm_ss() {
        assert_eq!(parse_time("1:00:00").unwrap(), 3600.0);
        assert_eq!(parse_time("0:01:30").unwrap(), 90.0);
    }

    #[test]
    fn parse_time_rejects_garbage() {
        assert!(parse_time("abc").is_err());
        assert!(parse_time("").is_err());
        assert!(parse_time("1:2:3:4").is_err());
    }

    #[test]
    fn parse_size_decimal_units() {
        assert_eq!(parse_size("5mb").unwrap(), 5_000_000);
        assert_eq!(parse_size("5MB").unwrap(), 5_000_000);
        assert_eq!(parse_size("5m").unwrap(), 5_000_000);
        assert_eq!(parse_size("500kb").unwrap(), 500_000);
        assert_eq!(parse_size("500k").unwrap(), 500_000);
        assert_eq!(parse_size("2gb").unwrap(), 2_000_000_000);
        assert_eq!(parse_size("1.5mb").unwrap(), 1_500_000);
    }

    #[test]
    fn parse_size_raw_bytes() {
        assert_eq!(parse_size("5000000").unwrap(), 5_000_000);
        assert_eq!(parse_size("1024b").unwrap(), 1024);
    }

    #[test]
    fn parse_size_rejects_garbage() {
        assert!(parse_size("").is_err());
        assert!(parse_size("5xb").is_err());
        assert!(parse_size("mb").is_err());
        assert!(parse_size("0").is_err());
        assert!(parse_size("-5mb").is_err());
    }
}
