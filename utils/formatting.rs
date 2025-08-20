/// Utility functions for formatting various display values

/// Format market cap for display
pub fn format_market_cap(mc: f64) -> String {
    if mc >= 1_000_000_000.0 {
        format!("{:.2}B", mc / 1_000_000_000.0)
    } else if mc >= 1_000_000.0 {
        format!("{:.1}M", mc / 1_000_000.0)
    } else if mc >= 1_000.0 {
        format!("{:.0}K", mc / 1_000.0)
    } else {
        format!("{:.0}", mc)
    }
}

/// Format volume for display
pub fn format_volume(vol: f64) -> String {
    if vol >= 1_000_000_000.0 {
        format!("{:.2}B", vol / 1_000_000_000.0)
    } else if vol >= 1_000_000.0 {
        format!("{:.1}M", vol / 1_000_000.0)
    } else if vol >= 1_000.0 {
        format!("{:.0}K", vol / 1_000.0)
    } else {
        format!("{:.0}", vol)
    }
}

/// Format SOL amount for display
pub fn format_sol(amount: f64) -> String {
    if amount >= 1_000.0 {
        format!("{:.2}K", amount / 1_000.0)
    } else if amount >= 1.0 {
        format!("{:.4}", amount)
    } else if amount >= 0.01 {
        format!("{:.6}", amount)
    } else {
        format!("{:.9}", amount)
    }
}

/// Format USD amount for display
pub fn format_usd(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        format!("${:.2}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("${:.0}K", amount / 1_000.0)
    } else if amount >= 1.0 {
        format!("${:.2}", amount)
    } else {
        format!("${:.4}", amount)
    }
}

/// Format percentage for display
pub fn format_percentage(pct: f64) -> String {
    if pct > 0.0 {
        format!("+{:.2}%", pct)
    } else {
        format!("{:.2}%", pct)
    }
}

/// Format token amount with appropriate precision
pub fn format_token_amount(amount: f64) -> String {
    if amount >= 1_000_000_000.0 {
        format!("{:.2}B", amount / 1_000_000_000.0)
    } else if amount >= 1_000_000.0 {
        format!("{:.2}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("{:.1}K", amount / 1_000.0)
    } else if amount >= 1.0 {
        format!("{:.2}", amount)
    } else {
        format!("{:.6}", amount)
    }
}

/// Format time duration for display
pub fn format_duration(seconds: u64) -> String {
    if seconds >= 86400 {
        let days = seconds / 86400;
        format!("{}d", days)
    } else if seconds >= 3600 {
        let hours = seconds / 3600;
        format!("{}h", hours)
    } else if seconds >= 60 {
        let minutes = seconds / 60;
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds)
    }
}

/// Truncate string to specified length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Format wallet address for display (shortened)
pub fn format_address(address: &str) -> String {
    if address.len() > 10 {
        format!("{}...{}", &address[..4], &address[address.len() - 4..])
    } else {
        address.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_market_cap() {
        assert_eq!(format_market_cap(1_500_000_000.0), "1.50B");
        assert_eq!(format_market_cap(500_000.0), "500K");
        assert_eq!(format_market_cap(50.0), "50");
    }

    #[test]
    fn test_format_percentage() {
        assert_eq!(format_percentage(10.5), "+10.50%");
        assert_eq!(format_percentage(-5.25), "-5.25%");
    }

    #[test]
    fn test_format_address() {
        let addr = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
        assert_eq!(format_address(addr), "DezX...B263");
        assert_eq!(format_address("short"), "short");
    }
}