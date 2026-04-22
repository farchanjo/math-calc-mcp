//! Port of `UnitConverterTool.java` — measurement unit conversions with
//! category validation and auto-detection.
//!
//! All public functions return `String`. Engine errors are embedded as
//! `"Error: {msg}"` to match the Java tool's behavior.

use std::str::FromStr;

use bigdecimal::BigDecimal;

use crate::engine::bigdecimal_ext::strip_plain;
use crate::engine::unit_registry::{self, UnitCategory, UnitError};

fn parse_value(value: &str) -> Result<BigDecimal, UnitError> {
    BigDecimal::from_str(value)
        .map_err(|_| UnitError::UnknownUnit(format!("invalid number: {value}")))
}

fn validate_category(code: &str, expected: UnitCategory) -> Result<(), UnitError> {
    match unit_registry::find_unit(code) {
        None => Err(UnitError::UnknownUnit(code.to_string())),
        Some(def) if def.category != expected => Err(UnitError::WrongCategory {
            code: code.to_string(),
            category: expected.as_str().to_string(),
        }),
        Some(_) => Ok(()),
    }
}

/// Convert `value` between two units within an explicit `category`.
///
/// Mirrors Java `UnitConverterTool#convert`: parses the category name
/// (case-insensitive), validates both unit codes belong to that category,
/// and delegates to the registry. Errors are returned inline.
#[must_use]
pub fn convert(value: &str, from_unit: &str, to_unit: &str, category: &str) -> String {
    let result: Result<String, UnitError> = (|| {
        let cat = UnitCategory::parse(category)?;
        validate_category(from_unit, cat)?;
        validate_category(to_unit, cat)?;
        let parsed = parse_value(value)?;
        let out = unit_registry::convert(&parsed, from_unit, to_unit)?;
        Ok(strip_plain(&out))
    })();
    match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

/// Convert `value` between two units, auto-detecting the shared category.
///
/// Mirrors Java `UnitConverterTool#convertAutoDetect`: relies on the registry's
/// built-in cross-category validation. Errors are returned inline.
#[must_use]
pub fn convert_auto_detect(value: &str, from_unit: &str, to_unit: &str) -> String {
    let result: Result<String, UnitError> = (|| {
        let parsed = parse_value(value)?;
        let out = unit_registry::convert(&parsed, from_unit, to_unit)?;
        Ok(strip_plain(&out))
    })();
    match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn km_to_m_happy_path() {
        assert_eq!(convert("1", "km", "m", "LENGTH"), "1000");
    }

    #[test]
    fn category_parse_case_insensitive() {
        assert_eq!(convert("1", "km", "m", "length"), "1000");
    }

    #[test]
    fn cross_category_unit_in_wrong_category() {
        let out = convert("1", "kg", "m", "LENGTH");
        assert!(
            out.starts_with("Error: Unit 'kg' is not in category LENGTH"),
            "got: {out}"
        );
    }

    #[test]
    fn unknown_category_error() {
        let out = convert("1", "km", "m", "NOT_A_CATEGORY");
        assert!(
            out.starts_with("Error: Unknown category: NOT_A_CATEGORY"),
            "got: {out}"
        );
    }

    #[test]
    fn unknown_unit_error() {
        let out = convert("1", "zzz", "m", "LENGTH");
        assert!(out.starts_with("Error: Unknown unit: zzz"), "got: {out}");
    }

    #[test]
    fn auto_detect_km_to_mi() {
        let out = convert_auto_detect("1", "km", "mi");
        assert_eq!(out, "0.6213711922373339696174341843633182");
    }

    #[test]
    fn auto_detect_cross_category_error() {
        let out = convert_auto_detect("1", "kg", "m");
        assert!(
            out.starts_with("Error: Cannot convert between"),
            "got: {out}"
        );
    }

    #[test]
    fn auto_detect_temperature_celsius_fahrenheit() {
        assert_eq!(convert_auto_detect("100", "c", "f"), "212");
    }

    #[test]
    fn invalid_number_returns_error() {
        let out = convert("not-a-number", "km", "m", "LENGTH");
        assert!(out.starts_with("Error: "), "got: {out}");
    }
}
