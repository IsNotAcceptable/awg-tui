use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.2} {}", UNITS[i])
    }
}

pub fn rate(bps: f64) -> String {
    format!("{}/s", bytes(bps.max(0.0) as u64))
}

/// "12s ago", "3m 05s ago", …
pub fn ago(ts: u64) -> String {
    let d = now_secs().saturating_sub(ts);
    match d {
        0..=59 => format!("{d}s ago"),
        60..=3599 => format!("{}m {:02}s ago", d / 60, d % 60),
        3600..=86399 => format!("{}h {:02}m ago", d / 3600, (d % 3600) / 60),
        _ => format!("{}d {}h ago", d / 86400, (d % 86400) / 3600),
    }
}

/// Shortens a base64 key for display.
pub fn short_key(k: &str) -> String {
    if k.len() > 16 {
        format!("{}…{}", &k[..8], &k[k.len() - 6..])
    } else {
        k.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        assert_eq!(bytes(10), "10 B");
        assert_eq!(bytes(2048), "2.00 KiB");
    }
}
