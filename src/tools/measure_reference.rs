//! Port of `MeasureReferenceTool.java` — introspection helpers for the unit
//! registry (category listing, unit listing, conversion factor, explanation).
//!
//! All public functions return `String`. JSON outputs are produced via
//! `serde_json` to guarantee well-formed payloads matching the Java tool.

use serde::Serialize;

use crate::engine::bigdecimal_ext::strip_plain;
use crate::engine::unit_registry::{self, UnitCategory, UnitError};

#[derive(Serialize)]
struct CategoryView<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct UnitView<'a> {
    code: &'a str,
    name: &'a str,
}

/// List every registered category as a JSON array of `{"name":"..."}` objects.
///
/// Matches Java `MeasureReferenceTool#listCategories`.
#[must_use]
pub fn list_categories() -> String {
    let views: Vec<CategoryView<'_>> = unit_registry::list_categories()
        .iter()
        .map(|c| CategoryView { name: c.as_str() })
        .collect();
    serde_json::to_string(&views).unwrap_or_else(|e| format!("Error: {e}"))
}

/// List every unit in `category` as a JSON array of `{"code":..,"name":..}`.
///
/// Matches Java `MeasureReferenceTool#listUnits`; unknown categories yield
/// `"Error: Unknown category: ..."`.
#[must_use]
pub fn list_units(category: &str) -> String {
    let result: Result<String, UnitError> = (|| {
        let cat = UnitCategory::parse(category)?;
        let units = unit_registry::list_units(cat);
        let views: Vec<UnitView<'_>> = units
            .iter()
            .map(|u| UnitView {
                code: u.code.as_str(),
                name: u.name.as_str(),
            })
            .collect();
        serde_json::to_string(&views).map_err(|e| UnitError::UnknownUnit(e.to_string()))
    })();
    match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

/// Return the multiplicative factor that maps `from_unit` to `to_unit`.
///
/// Matches Java `MeasureReferenceTool#getConversionFactor`. Temperatures yield
/// `"Error: Temperature uses formulas, not a fixed factor"`.
#[must_use]
pub fn get_conversion_factor(from_unit: &str, to_unit: &str) -> String {
    match unit_registry::conversion_factor(from_unit, to_unit) {
        Ok(factor) => strip_plain(&factor),
        Err(e) => format!("Error: {e}"),
    }
}

/// Human-readable explanation of a unit conversion.
///
/// Matches Java `MeasureReferenceTool#explainConversion` byte-for-byte: the
/// registry already produces `"1 {from_name} = {factor} {to_name}"` for
/// linear conversions and the temperature formula strings for Celsius,
/// Fahrenheit, Kelvin, and Rankine pairs.
#[must_use]
pub fn explain_conversion(from_unit: &str, to_unit: &str) -> String {
    match unit_registry::explain_conversion(from_unit, to_unit) {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn list_categories_returns_21_entries() {
        let json = list_categories();
        let parsed: Vec<Value> = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed.len(), 21);
        assert_eq!(parsed[0]["name"], "DATA_STORAGE");
        assert_eq!(parsed[1]["name"], "LENGTH");
        assert_eq!(parsed[4]["name"], "TEMPERATURE");
        assert_eq!(parsed[20]["name"], "CURRENT");
    }

    #[test]
    fn list_categories_raw_json_format() {
        let json = list_categories();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("{\"name\":\"DATA_STORAGE\"}"));
        assert!(json.contains("{\"name\":\"CURRENT\"}"));
    }

    #[test]
    fn list_units_length_matches_expected_codes() {
        let json = list_units("LENGTH");
        let parsed: Vec<Value> = serde_json::from_str(&json).expect("valid JSON");
        let codes: Vec<&str> = parsed.iter().map(|v| v["code"].as_str().unwrap()).collect();
        assert_eq!(
            codes,
            vec!["m", "mm", "cm", "km", "in", "ft", "yd", "mi", "nmi"]
        );
        // Name check for the first entry
        assert_eq!(parsed[0]["name"], "meter");
    }

    #[test]
    fn list_units_case_insensitive_category() {
        let json = list_units("length");
        let parsed: Vec<Value> = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed.len(), 9);
    }

    #[test]
    fn list_units_unknown_category() {
        let out = list_units("BOGUS");
        assert_eq!(out, "Error: Unknown category: BOGUS");
    }

    #[test]
    fn conversion_factor_km_to_m() {
        assert_eq!(get_conversion_factor("km", "m"), "1000");
    }

    #[test]
    fn conversion_factor_temperature_error() {
        let out = get_conversion_factor("c", "f");
        assert_eq!(out, "Error: Temperature uses formulas, not a fixed factor");
    }

    #[test]
    fn conversion_factor_unknown_unit() {
        let out = get_conversion_factor("zzz", "m");
        assert_eq!(out, "Error: Unknown unit: zzz");
    }

    #[test]
    fn explain_conversion_linear() {
        assert_eq!(
            explain_conversion("km", "mi"),
            "1 kilometer = 0.6213711922373339696174341843633182 mile"
        );
    }

    #[test]
    fn explain_conversion_temperature_formula() {
        assert_eq!(explain_conversion("c", "f"), "F = C * 9/5 + 32");
        assert_eq!(explain_conversion("k", "c"), "C = K - 273.15");
    }

    #[test]
    fn explain_conversion_cross_category_error() {
        let out = explain_conversion("kg", "m");
        assert!(
            out.starts_with("Error: Cannot convert between"),
            "got: {out}"
        );
    }
}
