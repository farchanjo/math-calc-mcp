//! Port of `CookingConverterTool.java` — narrow cooking-specific converters
//! for volume, weight, and oven temperature (including UK gas mark).
//!
//! All public functions return `String`. Errors are embedded as
//! `"Error: {msg}"` to match the Java tool's behavior.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use num_traits::ToPrimitive;

use crate::engine::bigdecimal_ext::strip_plain;
use crate::engine::unit_registry::{self, UnitError};

const VOLUME_UNITS: &[&str] = &[
    "l", "ml", "uscup", "cup", "tbsp", "tsp", "usfloz", "floz", "usgal", "gal", "igal",
];
const WEIGHT_UNITS: &[&str] = &["kg", "g", "mg", "lb", "oz"];
const TEMP_UNITS: &[&str] = &["c", "f"];
const GAS_MARK: &str = "gasmark";

fn resolve_volume_alias(code: &str) -> String {
    let lower = code.to_ascii_lowercase();
    match lower.as_str() {
        "cup" => "uscup".to_string(),
        "floz" => "usfloz".to_string(),
        "gal" => "usgal".to_string(),
        _ => lower,
    }
}

fn parse_value(value: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(value).map_err(|_| format!("invalid number: {value}"))
}

fn validate_allowed(code: &str, allowed: &[&str], kind: &str) -> Result<(), String> {
    let lower = code.to_ascii_lowercase();
    if allowed.iter().any(|a| *a == lower) {
        Ok(())
    } else {
        Err(format!("'{code}' is not a valid {kind} unit"))
    }
}

fn validate_oven_unit(code: &str) -> Result<(), String> {
    if TEMP_UNITS.contains(&code) {
        Ok(())
    } else {
        Err(format!(
            "Oven temperature unit must be c, f, or gasmark. Received: {code}"
        ))
    }
}

/// Convert a cooking volume (l, ml, cups, tbsp, tsp, floz, gal, etc.).
///
/// Accepts aliases `cup`/`floz`/`gal` which resolve to US variants, matching
/// the Java `CookingConverterTool#convertCookingVolume` behavior.
#[must_use]
pub fn convert_cooking_volume(value: &str, from_unit: &str, to_unit: &str) -> String {
    let result: Result<String, String> = (|| {
        validate_allowed(from_unit, VOLUME_UNITS, "cooking volume")?;
        validate_allowed(to_unit, VOLUME_UNITS, "cooking volume")?;
        let from = resolve_volume_alias(from_unit);
        let dest = resolve_volume_alias(to_unit);
        let parsed = parse_value(value)?;
        let out =
            unit_registry::convert(&parsed, &from, &dest).map_err(|e: UnitError| e.to_string())?;
        Ok(strip_plain(&out))
    })();
    match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

/// Convert a cooking weight (kg, g, mg, lb, oz).
#[must_use]
pub fn convert_cooking_weight(value: &str, from_unit: &str, to_unit: &str) -> String {
    let result: Result<String, String> = (|| {
        validate_allowed(from_unit, WEIGHT_UNITS, "cooking weight")?;
        validate_allowed(to_unit, WEIGHT_UNITS, "cooking weight")?;
        let parsed = parse_value(value)?;
        let out = unit_registry::convert(&parsed, from_unit, to_unit)
            .map_err(|e: UnitError| e.to_string())?;
        Ok(strip_plain(&out))
    })();
    match result {
        Ok(s) => s,
        Err(e) => format!("Error: {e}"),
    }
}

fn oven_to_celsius(input: &BigDecimal, source: &str) -> Result<BigDecimal, String> {
    if source == GAS_MARK {
        let mark = input
            .to_i32()
            .ok_or_else(|| format!("Gas mark must be an integer. Received: {input}"))?;
        unit_registry::gas_mark_to_celsius(mark).map_err(|e| e.to_string())
    } else {
        validate_oven_unit(source)?;
        unit_registry::to_celsius(source, input).map_err(|e| e.to_string())
    }
}

fn celsius_to_oven(celsius: &BigDecimal, target: &str) -> Result<BigDecimal, String> {
    if target == GAS_MARK {
        let mark = unit_registry::celsius_to_gas_mark(celsius).map_err(|e| e.to_string())?;
        Ok(BigDecimal::from(mark))
    } else {
        validate_oven_unit(target)?;
        unit_registry::from_celsius(target, celsius).map_err(|e| e.to_string())
    }
}

/// Convert an oven temperature between Celsius, Fahrenheit, and UK gas mark.
///
/// Routing mirrors the Java logic: source → Celsius → target. Gas marks are
/// looked up via the registry's fixed table.
#[must_use]
pub fn convert_oven_temperature(value: &str, from_unit: &str, to_unit: &str) -> String {
    let result: Result<String, String> = (|| {
        let source = from_unit.to_ascii_lowercase();
        let target = to_unit.to_ascii_lowercase();
        let input = parse_value(value)?;
        let celsius = oven_to_celsius(&input, &source)?;
        let out = celsius_to_oven(&celsius, &target)?;
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
    fn weight_lb_to_oz_happy_path() {
        assert_eq!(convert_cooking_weight("1", "lb", "oz"), "16");
    }

    #[test]
    fn weight_invalid_unit() {
        let out = convert_cooking_weight("1", "lb", "m");
        assert_eq!(out, "Error: 'm' is not a valid cooking weight unit");
    }

    #[test]
    fn volume_cup_alias_to_tbsp() {
        // 1 US cup = 16 tbsp exactly.
        assert_eq!(convert_cooking_volume("1", "cup", "tbsp"), "16");
    }

    #[test]
    fn volume_floz_alias_to_ml() {
        // 1 US fl oz = 29.5735295625 ml.
        assert_eq!(convert_cooking_volume("1", "floz", "ml"), "29.5735295625");
    }

    #[test]
    fn volume_gal_alias_to_l() {
        // 1 US gallon = 3.785411784 liters exactly.
        assert_eq!(convert_cooking_volume("1", "gal", "l"), "3.785411784");
    }

    #[test]
    fn volume_invalid_unit() {
        let out = convert_cooking_volume("1", "kg", "l");
        assert_eq!(out, "Error: 'kg' is not a valid cooking volume unit");
    }

    #[test]
    fn oven_celsius_to_fahrenheit() {
        assert_eq!(convert_oven_temperature("100", "c", "f"), "212");
    }

    #[test]
    fn oven_gas_mark_to_celsius() {
        assert_eq!(convert_oven_temperature("4", "gasmark", "c"), "180");
    }

    #[test]
    fn oven_celsius_to_gas_mark() {
        assert_eq!(convert_oven_temperature("200", "c", "gasmark"), "6");
    }

    #[test]
    fn oven_invalid_unit_error() {
        let out = convert_oven_temperature("100", "k", "c");
        assert!(
            out.starts_with("Error: Oven temperature unit must be c, f, or gasmark. Received: k"),
            "got: {out}"
        );
    }

    #[test]
    fn oven_gasmark_roundtrip() {
        // gasmark 6 => 200 C => gasmark 6
        assert_eq!(convert_oven_temperature("6", "gasmark", "gasmark"), "6");
    }

    #[test]
    fn oven_fahrenheit_to_gasmark() {
        // 356 F = 180 C ≈ gasmark 4
        assert_eq!(convert_oven_temperature("356", "f", "gasmark"), "4");
    }
}
