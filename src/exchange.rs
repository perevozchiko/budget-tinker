use crate::model::*;
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct RateRow {
    date: String,
    base: String,
    quote: String,
    rate: f64,
}
/// The API's base is the account currency, quote is the budget currency.
pub fn fetch(currency: &str, base: &str, month: &str) -> Result<(i64, String)> {
    if !CURRENCIES.contains(&currency) || !CURRENCIES.contains(&base) || !valid_month(month) {
        return Err("Invalid currency or month".into());
    }
    if month > current_month().as_str() {
        return Err("Enter a planned exchange rate manually for future months".into());
    }
    let date = if month == current_month() {
        today()
    } else {
        Budget::month_end(month)
    };
    let url = format!("https://api.frankfurter.dev/v2/rate/{currency}/{base}?date={date}");
    let curl = if cfg!(target_os = "macos") {
        "/usr/bin/curl"
    } else {
        "curl"
    };
    let result = Command::new(curl)
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "20",
            "--proto",
            "=https",
            &url,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "Could not fetch the exchange rate. You can enter it manually. {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    let row: RateRow = serde_json::from_slice(&result.stdout)
        .map_err(|e| format!("Rate service response: {e}"))?;
    if row.base != currency
        || row.quote != base
        || !row.rate.is_finite()
        || row.rate <= 0.0
        || row.date > date
        || chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d").is_err()
    {
        return Err("The service returned an invalid exchange rate".into());
    }
    Ok((parse_fixed(&format!("{:.8}", row.rate), 8)?, row.date))
}
