//! Port of `ProgrammableCalculatorTool.java` — expression evaluation with
//! optional variable bindings supplied as a JSON map.
//!
//! Matches the Java MCP tool contract: every failure is surfaced as an
//! `"Error: ..."` string instead of an exception, so the MCP client always
//! receives a plain `String` response.

use std::collections::HashMap;

use crate::engine::expression::{
    evaluate as engine_evaluate, evaluate_with_variables as engine_evaluate_with_variables,
};

/// Evaluate a math expression without variables.
///
/// Errors (blank input, malformed syntax, unknown function, …) are returned
/// as `"Error: {message}"` rather than panicking.
#[must_use]
pub fn evaluate(expression: &str) -> String {
    match engine_evaluate(expression) {
        Ok(value) => format_double(value),
        Err(err) => format!("Error: {err}"),
    }
}

/// Evaluate a math expression with variable bindings supplied as a JSON object
/// (e.g. `{"x":3.0,"y":1.5}`).
///
/// JSON parse failures, unknown variables, and evaluation errors are all
/// surfaced as `"Error: {message}"`.
#[must_use]
pub fn evaluate_with_variables(expression: &str, variables_json: &str) -> String {
    let variables: HashMap<String, f64> = match serde_json::from_str(variables_json) {
        Ok(map) => map,
        Err(err) => return format!("Error: {err}"),
    };
    match engine_evaluate_with_variables(expression, &variables) {
        Ok(value) => format_double(value),
        Err(err) => format!("Error: {err}"),
    }
}

/// Render an `f64` matching Java's `String.valueOf(double)` / `Double.toString`:
/// integer-valued floats keep a trailing `.0` (e.g. `14.0`, `1.0`), others use
/// Rust's shortest-round-trip Debug formatting (which coincides with Java's
/// representation for finite values).
fn format_double(value: f64) -> String {
    format!("{value:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- evaluate ----

    #[test]
    fn evaluate_integer_arithmetic() {
        assert_eq!(evaluate("2+3*4"), "14.0");
    }

    #[test]
    fn evaluate_trig_exact() {
        assert_eq!(evaluate("sin(90)"), "1.0");
    }

    #[test]
    fn evaluate_empty_is_error() {
        let out = evaluate("");
        assert!(out.starts_with("Error:"), "got: {out}");
        assert_eq!(out, "Error: Expression must not be null or blank");
    }

    #[test]
    fn evaluate_blank_is_error() {
        assert!(evaluate("   \t\n").starts_with("Error:"));
    }

    #[test]
    fn evaluate_unknown_variable_is_error() {
        let out = evaluate("foo + 1");
        assert_eq!(out, "Error: Unknown variable: foo");
    }

    #[test]
    fn evaluate_decimal_result_has_no_trailing_zero() {
        assert_eq!(evaluate("0.1 + 0.2"), "0.30000000000000004");
    }

    // ---- evaluate_with_variables ----

    #[test]
    fn eval_vars_simple() {
        assert_eq!(
            evaluate_with_variables("2*x + y", r#"{"x":3,"y":1}"#),
            "7.0"
        );
    }

    #[test]
    fn eval_vars_power() {
        assert_eq!(evaluate_with_variables("x^2", r#"{"x":5}"#), "25.0");
    }

    #[test]
    fn eval_vars_empty_object_is_valid() {
        assert_eq!(evaluate_with_variables("1+2", "{}"), "3.0");
    }

    #[test]
    fn eval_vars_invalid_json_is_error() {
        let out = evaluate_with_variables("x+1", "not-json");
        assert!(out.starts_with("Error:"), "got: {out}");
    }

    #[test]
    fn eval_vars_unknown_variable_is_error() {
        let out = evaluate_with_variables("z + 1", r#"{"x":1}"#);
        assert_eq!(out, "Error: Unknown variable: z");
    }

    #[test]
    fn eval_vars_empty_expression_is_error() {
        let out = evaluate_with_variables("", "{}");
        assert_eq!(out, "Error: Expression must not be null or blank");
    }

    #[test]
    fn eval_vars_float_values() {
        assert_eq!(
            evaluate_with_variables("x + y", r#"{"x":1.5,"y":2.5}"#),
            "4.0"
        );
    }
}
