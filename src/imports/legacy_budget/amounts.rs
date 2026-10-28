use super::*;

/// Parse JSON numbers as exact decimal strings, never binary floating point.
pub(super) fn money(v: &Value, currency: &str) -> Result<i64> {
    let Value::Number(n) = v else {
        return Err("Missing or invalid monetary amount".into());
    };
    let raw = n.to_string();
    let (mantissa, exponent) =
        raw.split_once(['e', 'E'])
            .map_or(Ok((raw.as_str(), 0i32)), |(a, b)| {
                b.parse::<i32>()
                    .map(|e| (a, e))
                    .map_err(|_| "Invalid monetary exponent")
            })?;
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.trim_start_matches('-');
    let fractional = unsigned.split_once('.').map_or(0, |(_, f)| f.len() as i32);
    let digits: String = unsigned.chars().filter(|c| *c != '.').collect();
    let mut value: i128 = digits.parse().map_err(|_| "Amount is too large")?;
    let shift = exponent
        .checked_add(decimals(currency) as i32)
        .and_then(|e| e.checked_sub(fractional))
        .ok_or("Invalid monetary exponent")?;
    if value == 0 {
        return Ok(0);
    }
    if !(-38..=38).contains(&shift) {
        return Err("Amount has unsupported precision or magnitude".into());
    }
    if shift >= 0 {
        value = value
            .checked_mul(10i128.pow(shift as u32))
            .ok_or("Amount is too large")?;
    } else {
        let divisor = 10i128.pow((-shift) as u32);
        if value % divisor != 0 {
            return Err("Amount has more precision than the budget currency".into());
        }
        value /= divisor;
    }
    if value > 100_000_000_000_000 {
        return Err("Amount exceeds the supported limit".into());
    }
    Ok(if negative {
        -(value as i64)
    } else {
        value as i64
    })
}
