//! Port of `PrintingCalculatorTool.java` — tape calculator with running totals.
//!
//! Behavioral parity:
//! * Arithmetic is exact, backed by `bigdecimal::BigDecimal` (matches `java.math.BigDecimal`).
//! * Division scale = 20, rounding mode = HALF_UP.
//! * Display scale = 2, right-aligned in a 14-char field followed by `"  {op}"`.
//! * `=` emits separator + running total line. `C` clears to zero. `T` emits grand total + clears.
//!
//! Error strings mirror the Java source verbatim and are returned as `"Error: {msg}"`.

use std::str::FromStr;

use bigdecimal::{BigDecimal, RoundingMode};
use num_traits::Zero;
use serde_json::Value;

const ERR_PREFIX: &str = "Error: ";
const DISPLAY_SCALE: i64 = 2;
const DIVISION_SCALE: i64 = 20;
const NUMBER_WIDTH: usize = 14;
const SEPARATOR: &str = "       --------";

/// Run a sequence of operations, returning the printed tape (or `"Error: ..."`).
pub fn calculate_with_tape(operations_json: &str) -> String {
    if operations_json.trim().is_empty() {
        return format!("{ERR_PREFIX}Operations must not be null or empty");
    }

    let entries = match parse_entries(operations_json) {
        Ok(entries) => entries,
        Err(msg) => return format!("{ERR_PREFIX}{msg}"),
    };

    match build_tape(&entries) {
        Ok(tape) => tape,
        Err(msg) => format!("{ERR_PREFIX}{msg}"),
    }
}

// --------------------------------------------------------------------------- //
//  Parsing
// --------------------------------------------------------------------------- //

#[derive(Debug)]
struct Entry {
    operation: String,
    value: Option<String>,
}

fn parse_entries(json: &str) -> Result<Vec<Entry>, String> {
    let parsed: Value =
        serde_json::from_str(json).map_err(|_| "Operations must be a JSON array".to_string())?;

    let array = parsed
        .as_array()
        .ok_or_else(|| "Operations must be a JSON array".to_string())?;

    let mut entries = Vec::with_capacity(array.len());
    for item in array {
        let obj = item
            .as_object()
            .ok_or_else(|| "Operations must be a JSON array".to_string())?;

        let op = obj
            .get("op")
            .ok_or_else(|| "Missing field: op".to_string())?;
        let op_str = match op {
            Value::String(s) => s.clone(),
            Value::Null => return Err("Missing field: op".to_string()),
            other => other.to_string(),
        };

        if !obj.contains_key("value") {
            return Err("Missing field: value".to_string());
        }
        let value_str = match obj.get("value") {
            Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Number(n)) => Some(n.to_string()),
            Some(Value::Bool(b)) => Some(b.to_string()),
            Some(_) => return Err("Missing field: value".to_string()),
            None => return Err("Missing field: value".to_string()),
        };

        entries.push(Entry {
            operation: op_str,
            value: value_str,
        });
    }
    Ok(entries)
}

// --------------------------------------------------------------------------- //
//  Tape construction
// --------------------------------------------------------------------------- //

fn build_tape(entries: &[Entry]) -> Result<String, String> {
    let mut total = BigDecimal::zero();
    let mut tape = String::new();

    for entry in entries {
        if needs_value(&entry.operation) {
            let raw = entry
                .value
                .as_deref()
                .ok_or_else(|| "Missing field: value".to_string())?;
            let value = BigDecimal::from_str(raw).map_err(|_| format!("Invalid number: {raw}"))?;
            total = apply_arithmetic(&total, &value, &entry.operation)?;
            append_line(&mut tape, &value, &entry.operation);
        } else {
            total = apply_control(&mut tape, &total, &entry.operation)?;
        }
    }

    Ok(tape)
}

fn apply_arithmetic(
    total: &BigDecimal,
    value: &BigDecimal,
    operation: &str,
) -> Result<BigDecimal, String> {
    match operation {
        "+" => Ok(total + value),
        "-" => Ok(total - value),
        "*" => Ok(total * value),
        "/" => {
            if value.is_zero() {
                return Err("Division by zero".to_string());
            }
            Ok(total.with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp)
                / value.with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp))
        }
        other => Err(format!("Unknown arithmetic operation: {other}")),
    }
}

fn apply_control(
    tape: &mut String,
    total: &BigDecimal,
    operation: &str,
) -> Result<BigDecimal, String> {
    match operation {
        "=" => {
            tape.push_str(SEPARATOR);
            tape.push('\n');
            append_line(tape, total, "=");
            Ok(total.clone())
        }
        "C" => {
            append_line(tape, &BigDecimal::zero(), "C");
            Ok(BigDecimal::zero())
        }
        "T" => {
            tape.push_str(SEPARATOR);
            tape.push('\n');
            append_line(tape, total, "T");
            Ok(BigDecimal::zero())
        }
        other => Err(format!("Unknown control operation: {other}")),
    }
}

fn append_line(tape: &mut String, value: &BigDecimal, operation: &str) {
    let formatted = value
        .with_scale_round(DISPLAY_SCALE, RoundingMode::HalfUp)
        .to_plain_string();
    tape.push_str(&format!(
        "{formatted:>width$}  {operation}\n",
        width = NUMBER_WIDTH
    ));
}

fn needs_value(operation: &str) -> bool {
    matches!(operation, "+" | "-" | "*" | "/")
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_subtract_with_subtotal() {
        let json = r#"[{"op":"+","value":"100"},{"op":"-","value":"30"},{"op":"=","value":null}]"#;
        let tape = calculate_with_tape(json);
        assert!(tape.contains("100.00  +"), "tape=\n{tape}");
        assert!(tape.contains("30.00  -"), "tape=\n{tape}");
        assert!(tape.contains("70.00  ="), "tape=\n{tape}");
        assert!(tape.contains(SEPARATOR), "tape=\n{tape}");
    }

    #[test]
    fn multiply_divide_and_grand_total() {
        let json = r#"[
            {"op":"+","value":"10"},
            {"op":"*","value":"5"},
            {"op":"/","value":"2"},
            {"op":"T","value":null}
        ]"#;
        let tape = calculate_with_tape(json);
        // 0 + 10 = 10, *5 = 50, /2 = 25
        assert!(tape.contains("25.00  T"), "tape=\n{tape}");
    }

    #[test]
    fn clear_resets_total() {
        let json = r#"[
            {"op":"+","value":"100"},
            {"op":"C","value":null},
            {"op":"+","value":"5"},
            {"op":"=","value":null}
        ]"#;
        let tape = calculate_with_tape(json);
        assert!(tape.contains("0.00  C"), "tape=\n{tape}");
        assert!(tape.contains("5.00  ="), "tape=\n{tape}");
    }

    #[test]
    fn right_aligned_in_14_chars() {
        let json = r#"[{"op":"+","value":"1"}]"#;
        let tape = calculate_with_tape(json);
        let line = tape.lines().next().unwrap();
        // "          1.00  +"
        assert_eq!(line.len(), 14 + 2 + 1, "line=<{line}>");
        assert!(line.ends_with("  +"), "line=<{line}>");
    }

    #[test]
    fn err_empty_input() {
        assert_eq!(
            calculate_with_tape(""),
            "Error: Operations must not be null or empty"
        );
        assert_eq!(
            calculate_with_tape("   "),
            "Error: Operations must not be null or empty"
        );
    }

    #[test]
    fn err_not_a_json_array() {
        assert_eq!(
            calculate_with_tape("{\"op\":\"+\",\"value\":\"1\"}"),
            "Error: Operations must be a JSON array"
        );
        assert_eq!(
            calculate_with_tape("garbage"),
            "Error: Operations must be a JSON array"
        );
    }

    #[test]
    fn err_missing_op_field() {
        let json = r#"[{"value":"1"}]"#;
        assert_eq!(calculate_with_tape(json), "Error: Missing field: op");
    }

    #[test]
    fn err_missing_value_field() {
        let json = r#"[{"op":"+"}]"#;
        assert_eq!(calculate_with_tape(json), "Error: Missing field: value");
    }

    #[test]
    fn err_division_by_zero() {
        let json = r#"[{"op":"+","value":"10"},{"op":"/","value":"0"}]"#;
        assert_eq!(calculate_with_tape(json), "Error: Division by zero");
    }

    #[test]
    fn err_unknown_control_op() {
        // Any op outside +,-,*,/ is routed to apply_control (matches Java).
        let json = r#"[{"op":"X","value":null}]"#;
        assert_eq!(
            calculate_with_tape(json),
            "Error: Unknown control operation: X"
        );
        let json = r#"[{"op":"^","value":"2"}]"#;
        assert_eq!(
            calculate_with_tape(json),
            "Error: Unknown control operation: ^"
        );
    }

    #[test]
    fn numeric_value_accepted() {
        // JSON numeric (not quoted) should still parse.
        let json = r#"[{"op":"+","value":42},{"op":"=","value":null}]"#;
        let tape = calculate_with_tape(json);
        assert!(tape.contains("42.00  +"), "tape=\n{tape}");
        assert!(tape.contains("42.00  ="), "tape=\n{tape}");
    }
}
