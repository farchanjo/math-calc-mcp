//! Port of `BasicCalculatorTool.java` — arbitrary-precision arithmetic with
//! Java `BigDecimal` semantics (exact results, `toPlainString()` formatting,
//! `HALF_UP` rounding for division).

use std::str::FromStr;

use bigdecimal::{BigDecimal, RoundingMode};
use num_traits::Zero;

use crate::engine::bigdecimal_ext::{DIVISION_SCALE, strip_plain};

/// Errors raised by basic-calculator operations. Messages mirror the Java source verbatim.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BasicError {
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Exponent must be a non-negative integer")]
    InvalidExponent,
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
}

fn parse(input: &str) -> Result<BigDecimal, BasicError> {
    BigDecimal::from_str(input).map_err(|_| BasicError::InvalidNumber(input.to_string()))
}

/// Add two arbitrary-precision decimals.
pub fn add(first: &str, second: &str) -> Result<String, BasicError> {
    let lhs = parse(first)?;
    let rhs = parse(second)?;
    Ok((&lhs + &rhs).to_plain_string())
}

/// Subtract `second` from `first`.
pub fn subtract(first: &str, second: &str) -> Result<String, BasicError> {
    let lhs = parse(first)?;
    let rhs = parse(second)?;
    Ok((&lhs - &rhs).to_plain_string())
}

/// Multiply two arbitrary-precision decimals.
pub fn multiply(first: &str, second: &str) -> Result<String, BasicError> {
    let lhs = parse(first)?;
    let rhs = parse(second)?;
    Ok((&lhs * &rhs).to_plain_string())
}

/// Divide `first` by `second` at 20-digit scale with HALF_UP rounding, stripping trailing zeros.
pub fn divide(first: &str, second: &str) -> Result<String, BasicError> {
    let dividend = parse(first)?;
    let divisor = parse(second)?;
    if divisor.is_zero() {
        return Err(BasicError::DivisionByZero);
    }
    let quotient = (&dividend / &divisor).with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp);
    Ok(strip_plain(&quotient))
}

/// Raise `base` to a non-negative integer `exponent`.
pub fn power(base: &str, exponent: &str) -> Result<String, BasicError> {
    let base_value = parse(base)?;
    let exp: u32 = exponent.parse().map_err(|_| BasicError::InvalidExponent)?;
    Ok(base_value.powi(i64::from(exp)).to_plain_string())
}

/// Compute the remainder of `first / second`.
pub fn modulo(first: &str, second: &str) -> Result<String, BasicError> {
    let dividend = parse(first)?;
    let divisor = parse(second)?;
    if divisor.is_zero() {
        return Err(BasicError::DivisionByZero);
    }
    Ok((&dividend % &divisor).to_plain_string())
}

/// Absolute value of a decimal.
pub fn abs(value: &str) -> Result<String, BasicError> {
    Ok(parse(value)?.abs().to_plain_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_integers() {
        assert_eq!(add("1", "2").unwrap(), "3");
    }

    #[test]
    fn subtract_integers() {
        assert_eq!(subtract("10", "3").unwrap(), "7");
    }

    #[test]
    fn multiply_integers() {
        assert_eq!(multiply("3", "4").unwrap(), "12");
    }

    #[test]
    fn add_preserves_decimal_scale() {
        assert_eq!(add("1.5", "2.5").unwrap(), "4.0");
    }

    #[test]
    fn multiply_preserves_decimal_scale() {
        assert_eq!(multiply("2.5", "2").unwrap(), "5.0");
    }

    #[test]
    fn add_avoids_binary_float_drift() {
        assert_eq!(add("0.1", "0.2").unwrap(), "0.3");
    }

    #[test]
    fn divide_twenty_digit_precision() {
        assert_eq!(divide("10", "3").unwrap(), "3.33333333333333333333");
    }

    #[test]
    fn divide_strips_and_rounds_half_up() {
        assert_eq!(divide("1", "7").unwrap(), "0.14285714285714285714");
    }

    #[test]
    fn divide_by_zero() {
        assert_eq!(divide("10", "0"), Err(BasicError::DivisionByZero));
    }

    #[test]
    fn divide_strips_trailing_zeros() {
        assert_eq!(divide("1", "2").unwrap(), "0.5");
        assert_eq!(divide("10", "2").unwrap(), "5");
    }

    #[test]
    fn power_integer_base() {
        assert_eq!(power("2", "10").unwrap(), "1024");
    }

    #[test]
    fn power_decimal_base() {
        assert_eq!(power("1.5", "3").unwrap(), "3.375");
    }

    #[test]
    fn power_zero_exponent() {
        assert_eq!(power("7", "0").unwrap(), "1");
    }

    #[test]
    fn power_negative_exponent_rejected() {
        assert_eq!(power("2", "-1"), Err(BasicError::InvalidExponent));
    }

    #[test]
    fn power_non_integer_exponent_rejected() {
        assert_eq!(power("2", "1.5"), Err(BasicError::InvalidExponent));
    }

    #[test]
    fn modulo_integers() {
        assert_eq!(modulo("10", "3").unwrap(), "1");
    }

    #[test]
    fn modulo_decimal() {
        assert_eq!(modulo("7.5", "2").unwrap(), "1.5");
    }

    #[test]
    fn modulo_by_zero() {
        assert_eq!(modulo("5", "0"), Err(BasicError::DivisionByZero));
    }

    #[test]
    fn abs_negative() {
        assert_eq!(abs("-5").unwrap(), "5");
    }

    #[test]
    fn abs_preserves_scale() {
        assert_eq!(abs("3.14").unwrap(), "3.14");
    }

    #[test]
    fn invalid_input_reports_original_token() {
        assert_eq!(
            add("abc", "1"),
            Err(BasicError::InvalidNumber("abc".to_string()))
        );
    }

    #[test]
    fn invalid_second_operand() {
        assert_eq!(
            subtract("1", "xyz"),
            Err(BasicError::InvalidNumber("xyz".to_string()))
        );
    }
}
