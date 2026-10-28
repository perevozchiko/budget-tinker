use budgettinker::model::decimals;

const TOO_LARGE: &str = "Calculation is too large";

/// Exact, reduced fractions keep recurring divisions precise until the final rounding.
/// Checked i128 arithmetic bounds calculator complexity without using floating point.
#[derive(Clone, Copy)]
struct Fraction {
    n: i128,
    d: i128,
}

fn gcd(mut a: u128, mut b: u128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    // Every call includes a positive denominator bounded by i128::MAX.
    a as i128
}

impl Fraction {
    fn new(n: i128, d: i128) -> Self {
        debug_assert!(d > 0);
        let g = gcd(n.unsigned_abs(), d as u128);
        Self { n: n / g, d: d / g }
    }

    fn number(raw: &str) -> Result<Self, String> {
        let mut n = 0_i128;
        let mut d = 1_i128;
        let mut fractional = false;
        let mut digits = false;
        for b in raw.bytes() {
            if b == b'.' || b == b',' {
                if fractional {
                    return Err("Invalid number".into());
                }
                fractional = true;
            } else {
                digits = true;
                n = n
                    .checked_mul(10)
                    .and_then(|n| n.checked_add(i128::from(b - b'0')))
                    .ok_or(TOO_LARGE)?;
                if fractional {
                    d = d.checked_mul(10).ok_or(TOO_LARGE)?;
                }
            }
        }
        if !digits {
            return Err("Invalid number".into());
        }
        Ok(Self::new(n, d))
    }

    fn neg(self) -> Result<Self, String> {
        Ok(Self {
            n: self.n.checked_neg().ok_or(TOO_LARGE)?,
            ..self
        })
    }

    fn add(self, rhs: Self, subtract: bool) -> Result<Self, String> {
        let g = gcd(self.d as u128, rhs.d as u128);
        let left = self.n.checked_mul(rhs.d / g).ok_or(TOO_LARGE)?;
        let right = rhs.n.checked_mul(self.d / g).ok_or(TOO_LARGE)?;
        let n = if subtract {
            left.checked_sub(right)
        } else {
            left.checked_add(right)
        }
        .ok_or(TOO_LARGE)?;
        let d = self.d.checked_mul(rhs.d / g).ok_or(TOO_LARGE)?;
        Ok(Self::new(n, d))
    }

    fn mul(self, rhs: Self) -> Result<Self, String> {
        let a = gcd(self.n.unsigned_abs(), rhs.d as u128);
        let b = gcd(rhs.n.unsigned_abs(), self.d as u128);
        let n = (self.n / a).checked_mul(rhs.n / b).ok_or(TOO_LARGE)?;
        let d = (self.d / b).checked_mul(rhs.d / a).ok_or(TOO_LARGE)?;
        Ok(Self::new(n, d))
    }

    fn div(self, rhs: Self) -> Result<Self, String> {
        if rhs.n == 0 {
            return Err("Cannot divide by zero".into());
        }
        let reciprocal = Self {
            n: if rhs.n < 0 { -rhs.d } else { rhs.d },
            d: rhs.n.checked_abs().ok_or(TOO_LARGE)?,
        };
        self.mul(reciprocal)
    }

    fn minor_units(self, currency: &str) -> Result<i64, String> {
        let scaled = self.mul(Self {
            n: 10_i128.pow(decimals(currency)),
            d: 1,
        })?;
        let mut q = scaled.n / scaled.d;
        let r = scaled.n % scaled.d;
        // Round ties away from zero, once, at the currency boundary.
        if r.unsigned_abs() >= (scaled.d / 2 + scaled.d % 2) as u128 {
            q = q
                .checked_add(if scaled.n < 0 { -1 } else { 1 })
                .ok_or(TOO_LARGE)?;
        }
        i64::try_from(q).map_err(|_| TOO_LARGE.into())
    }
}

/// Evaluate exactly, then round the result to the nearest currency minor unit.
pub fn evaluate(input: &str, currency: &str) -> Result<i64, String> {
    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }
    impl Parser<'_> {
        fn skip(&mut self) {
            while self
                .bytes
                .get(self.pos)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.pos += 1;
            }
        }
        fn take(&mut self, c: u8) -> bool {
            self.skip();
            if self.bytes.get(self.pos) == Some(&c) {
                self.pos += 1;
                true
            } else {
                false
            }
        }
        fn factor(&mut self) -> Result<Fraction, String> {
            if self.take(b'-') {
                return self.factor()?.neg();
            }
            if self.take(b'+') {
                return self.factor();
            }
            if self.take(b'(') {
                let v = self.expr()?;
                if !self.take(b')') {
                    return Err("Missing closing parenthesis".into());
                }
                return Ok(v);
            }
            self.skip();
            let start = self.pos;
            while self
                .bytes
                .get(self.pos)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'.' || *b == b',')
            {
                self.pos += 1;
            }
            if start == self.pos {
                return Err("Enter a number".into());
            }
            let raw =
                std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "Invalid number")?;
            Fraction::number(raw)
        }
        fn term(&mut self) -> Result<Fraction, String> {
            let mut v = self.factor()?;
            loop {
                if self.take(b'*') {
                    let rhs = self.factor()?;
                    v = v.mul(rhs)?;
                } else if self.take(b'/') {
                    let rhs = self.factor()?;
                    v = v.div(rhs)?;
                } else {
                    return Ok(v);
                }
            }
        }
        fn expr(&mut self) -> Result<Fraction, String> {
            let mut v = self.term()?;
            loop {
                if self.take(b'+') {
                    v = v.add(self.term()?, false)?;
                } else if self.take(b'-') {
                    v = v.add(self.term()?, true)?;
                } else {
                    return Ok(v);
                }
            }
        }
    }
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let result = p.expr()?;
    p.skip();
    if p.pos != p.bytes.len() {
        return Err("Invalid expression".into());
    }
    result.minor_units(currency)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arithmetic_preserves_currency_precision_and_reports_errors() {
        assert_eq!(evaluate("(10.25 + 2.75) * 3 / 2", "USD"), Ok(1950));
        assert_eq!(evaluate("1 / 8", "USD"), Ok(13));
        assert_eq!(evaluate("-1 / 8", "USD"), Ok(-13));
        assert_eq!(evaluate("1 / 8", "KWD"), Ok(125));
        assert!(evaluate("1 / 0", "USD").is_err());
        assert_eq!(evaluate("1.001", "USD"), Ok(100));
        assert!(evaluate("2 +", "USD").is_err());
    }
    #[test]
    fn compound_expressions_round_only_the_final_amount() {
        for expression in [
            "1 / 3 * 3",
            "0.01 / 3 * 300",
            "(1 / 3 + 1 / 3 + 1 / 3)",
            "1 / (3 / 3)",
        ] {
            assert_eq!(evaluate(expression, "USD"), Ok(100), "{expression}");
        }
        assert_eq!(evaluate("-0.01 / 3 * 300", "USD"), Ok(-100));
        assert_eq!(evaluate("1 / -3 * -3", "USD"), Ok(100));
        assert_eq!(evaluate("1 / 200 + 1 / 200", "USD"), Ok(1));
    }

    #[test]
    fn operand_precision_is_independent_of_currency() {
        assert_eq!(evaluate("1000 * 1.1", "JPY"), Ok(1100));
        assert_eq!(evaluate("100 * 1.075", "USD"), Ok(10750));
        assert_eq!(evaluate("100 / 0.025", "JPY"), Ok(4000));
        assert_eq!(evaluate("100 * 1,075", "USD"), Ok(10750));
        assert_eq!(evaluate("1.5", "JPY"), Ok(2));
        assert_eq!(evaluate("-1.5", "JPY"), Ok(-2));
        assert_eq!(evaluate("0.0005", "KWD"), Ok(1));
        assert_eq!(evaluate("-0.0005", "KWD"), Ok(-1));
    }

    #[test]
    fn invalid_numbers_and_overflows_never_produce_a_saved_amount() {
        for expression in [
            ".",
            "1.2.3",
            "1,2.3",
            "1 / (0.1 - 0.1)",
            "99999999999999999999999999999999999999999999",
            "100000000000000000000 * 100000000000000000000",
            "(1 + 2",
            "2x3",
        ] {
            assert!(evaluate(expression, "USD").is_err(), "{expression}");
        }
        assert_eq!(evaluate("92233720368547758.07", "USD"), Ok(i64::MAX));
        assert_eq!(evaluate("-92233720368547758.08", "USD"), Ok(i64::MIN));
        assert!(evaluate("92233720368547758.075", "USD").is_err());
        assert!(evaluate("-92233720368547758.085", "USD").is_err());
        // Cross-cancellation avoids overflow for large, representable fractions.
        assert_eq!(
            evaluate(
                "100000000000000000000 / 3 * 3 / 100000000000000000000",
                "USD"
            ),
            Ok(100)
        );
    }
}
