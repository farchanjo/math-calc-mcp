//! Port of `CalculusTool.java` — numerical calculus (derivatives and integrals)
//! built on top of [`crate::engine::expression`].
//!
//! Algorithms:
//! * Derivative — five-point central difference.
//! * Nth-order derivative — finite difference with binomial coefficients.
//! * Definite integral — composite Simpson's rule with 10,000 intervals.
//! * Tangent line — derivative combined with function value; returns JSON.
//!
//! Every entry point mirrors the Java MCP contract: it returns a `String` and
//! encodes failures inline as `"Error: ..."`.

use std::collections::HashMap;

use crate::engine::expression::{ExpressionError, evaluate_with_variables};

const DERIVATIVE_STEP: f64 = 1e-6;
const SIMPSON_INTERVALS: i32 = 10_000;
const MAX_ORDER: i32 = 10;

/// Format an f64 using Rust's debug representation (closest available match to
/// Java's `String.valueOf(double)` — emits `1.0` for whole doubles).
fn format_f64(value: f64) -> String {
    format!("{value:?}")
}

/// Evaluate `expression` with `variable` bound to `value`.
fn eval(expression: &str, variable: &str, value: f64) -> Result<f64, ExpressionError> {
    let mut vars: HashMap<String, f64> = HashMap::with_capacity(1);
    vars.insert(variable.to_string(), value);
    evaluate_with_variables(expression, &vars)
}

/// Five-point central difference: `f'(x) ≈ (-f(x+2h) + 8f(x+h) - 8f(x-h) + f(x-2h)) / (12h)`.
fn central_difference(
    expression: &str,
    variable: &str,
    point: f64,
    step: f64,
) -> Result<f64, ExpressionError> {
    let f_minus2 = eval(expression, variable, point - 2.0 * step)?;
    let f_minus1 = eval(expression, variable, point - step)?;
    let f_plus1 = eval(expression, variable, point + step)?;
    let f_plus2 = eval(expression, variable, point + 2.0 * step)?;
    Ok((-f_plus2 + 8.0 * f_plus1 - 8.0 * f_minus1 + f_minus2) / (12.0 * step))
}

fn binomial_coeff(total: i32, choose: i32) -> f64 {
    let mut result: f64 = 1.0;
    let bound = choose.min(total - choose);
    for idx in 0..bound {
        result = result * f64::from(total - idx) / f64::from(idx + 1);
    }
    result
}

/// Finite-difference approximation of the `order`-th derivative at `point`.
fn nth_deriv(
    expression: &str,
    variable: &str,
    point: f64,
    order: i32,
) -> Result<f64, ExpressionError> {
    let step = DERIVATIVE_STEP.powf(1.0 / f64::from(order)) * 10.0;
    let half_n = order / 2;
    let mut result = 0.0;

    for idx in 0..=order {
        let x_sample = point + f64::from(idx - half_n) * step;
        let f_sample = eval(expression, variable, x_sample)?;
        let coeff = binomial_coeff(order, idx) * (-1.0_f64).powi(order - idx);
        result += coeff * f_sample;
    }
    Ok(result / step.powi(order))
}

/// Composite Simpson's rule with `SIMPSON_INTERVALS` subintervals.
fn simpson(
    expression: &str,
    variable: &str,
    lower: f64,
    upper: f64,
) -> Result<f64, ExpressionError> {
    let intervals = SIMPSON_INTERVALS;
    let step = (upper - lower) / f64::from(intervals);
    let mut sum = eval(expression, variable, lower)? + eval(expression, variable, upper)?;

    for idx in 1..intervals {
        let x_val = lower + f64::from(idx) * step;
        let f_val = eval(expression, variable, x_val)?;
        let multiplier = if idx % 2 == 0 { 2.0 } else { 4.0 };
        sum += multiplier * f_val;
    }
    Ok(sum * step / 3.0)
}

// --------------------------------------------------------------------------- //
//  Public tool entry points
// --------------------------------------------------------------------------- //

/// Compute the numerical derivative of `expression` w.r.t. `variable` at `point`.
pub fn derivative(expression: &str, variable: &str, point: f64) -> String {
    match central_difference(expression, variable, point, DERIVATIVE_STEP) {
        Ok(value) => format_f64(value),
        Err(err) => format!("Error: {err}"),
    }
}

/// Compute the nth-order numerical derivative. `order` must be in `[1, 10]`.
pub fn nth_derivative(expression: &str, variable: &str, point: f64, order: i32) -> String {
    if !(1..=MAX_ORDER).contains(&order) {
        return format!("Error: Order must be between 1 and {MAX_ORDER}. Received: {order}");
    }
    match nth_deriv(expression, variable, point, order) {
        Ok(value) => format_f64(value),
        Err(err) => format!("Error: {err}"),
    }
}

/// Definite integral over `[lower, upper]` by composite Simpson's rule.
pub fn definite_integral(expression: &str, variable: &str, lower: f64, upper: f64) -> String {
    match simpson(expression, variable, lower, upper) {
        Ok(value) => format_f64(value),
        Err(err) => format!("Error: {err}"),
    }
}

/// Compute the tangent line to `f(x)` at `point`. Returns a JSON object.
pub fn tangent_line(expression: &str, variable: &str, point: f64) -> String {
    let f_at_point = match eval(expression, variable, point) {
        Ok(v) => v,
        Err(err) => return format!("Error: {err}"),
    };
    let slope = match central_difference(expression, variable, point, DERIVATIVE_STEP) {
        Ok(v) => v,
        Err(err) => return format!("Error: {err}"),
    };
    let y_intercept = f_at_point - slope * point;
    let slope_s = format_f64(slope);
    let intercept_s = format_f64(y_intercept);
    format!(
        "{{\"slope\":{slope_s},\"yIntercept\":{intercept_s},\"equation\":\"y = {slope_s}*x + {intercept_s}\"}}"
    )
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_f(s: &str) -> f64 {
        s.parse::<f64>()
            .unwrap_or_else(|_| panic!("not a float: {s}"))
    }

    // ---- derivative ----

    #[test]
    fn derivative_of_x_squared_at_3() {
        // d/dx(x^2) = 2x → at x=3 → 6
        let out = derivative("x^2", "x", 3.0);
        assert!((parse_f(&out) - 6.0).abs() < 1e-6, "got {out}");
    }

    #[test]
    fn derivative_of_constant_is_zero() {
        let out = derivative("7", "x", 2.0);
        assert!(parse_f(&out).abs() < 1e-6, "got {out}");
    }

    #[test]
    fn derivative_of_cubic() {
        // d/dx(x^3) = 3x^2 → at x=2 → 12
        let out = derivative("x^3", "x", 2.0);
        assert!((parse_f(&out) - 12.0).abs() < 1e-5, "got {out}");
    }

    #[test]
    fn derivative_invalid_expression() {
        let out = derivative("1+", "x", 0.0);
        assert!(out.starts_with("Error:"), "got {out}");
    }

    // ---- nth_derivative ----

    #[test]
    fn nth_derivative_second_of_x_squared_is_two() {
        // d²/dx²(x^2) = 2
        let out = nth_derivative("x^2", "x", 5.0, 2);
        assert!((parse_f(&out) - 2.0).abs() < 1e-3, "got {out}");
    }

    #[test]
    fn nth_derivative_first_matches_central_diff() {
        // For order=1, nth_deriv should match the single derivative (loosely).
        let first = derivative("x^3", "x", 2.0);
        let out = nth_derivative("x^3", "x", 2.0, 1);
        assert!(
            (parse_f(&first) - parse_f(&out)).abs() < 1e-2,
            "first={first}, out={out}"
        );
    }

    #[test]
    fn nth_derivative_order_below_range() {
        let out = nth_derivative("x^2", "x", 1.0, 0);
        assert_eq!(out, "Error: Order must be between 1 and 10. Received: 0");
    }

    #[test]
    fn nth_derivative_order_above_range() {
        let out = nth_derivative("x^2", "x", 1.0, 11);
        assert_eq!(out, "Error: Order must be between 1 and 10. Received: 11");
    }

    #[test]
    fn nth_derivative_negative_order() {
        let out = nth_derivative("x^2", "x", 1.0, -1);
        assert_eq!(out, "Error: Order must be between 1 and 10. Received: -1");
    }

    // ---- definite_integral ----

    #[test]
    fn integral_of_x_squared_0_to_1() {
        // ∫₀¹ x² dx = 1/3
        let out = definite_integral("x^2", "x", 0.0, 1.0);
        assert!((parse_f(&out) - (1.0 / 3.0)).abs() < 1e-6, "got {out}");
    }

    #[test]
    fn integral_of_constant() {
        // ∫₀⁵ 4 dx = 20
        let out = definite_integral("4", "x", 0.0, 5.0);
        assert!((parse_f(&out) - 20.0).abs() < 1e-6, "got {out}");
    }

    #[test]
    fn integral_of_x_from_minus1_to_1_is_zero() {
        // Odd function over symmetric interval.
        let out = definite_integral("x", "x", -1.0, 1.0);
        assert!(parse_f(&out).abs() < 1e-6, "got {out}");
    }

    #[test]
    fn integral_invalid_expression() {
        let out = definite_integral("bogus(x)", "x", 0.0, 1.0);
        assert!(out.starts_with("Error:"), "got {out}");
    }

    // ---- tangent_line ----

    #[test]
    fn tangent_line_of_x_squared_at_3() {
        // f(x) = x² at x=3 → slope = 6, y-intercept = 9 - 6*3 = -9
        let out = tangent_line("x^2", "x", 3.0);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let slope = parsed["slope"].as_f64().unwrap();
        let intercept = parsed["yIntercept"].as_f64().unwrap();
        assert!((slope - 6.0).abs() < 1e-5, "slope={slope}");
        assert!((intercept - -9.0).abs() < 1e-4, "intercept={intercept}");
        assert!(parsed["equation"].as_str().unwrap().starts_with("y = "));
    }

    #[test]
    fn tangent_line_of_linear_function() {
        // f(x) = 2*x + 5 → slope = 2 everywhere, y-intercept = 5.
        let out = tangent_line("2*x + 5", "x", 7.0);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let slope = parsed["slope"].as_f64().unwrap();
        let intercept = parsed["yIntercept"].as_f64().unwrap();
        assert!((slope - 2.0).abs() < 1e-5, "slope={slope}");
        assert!((intercept - 5.0).abs() < 1e-5, "intercept={intercept}");
    }

    #[test]
    fn tangent_line_invalid_expression() {
        let out = tangent_line("unknown_fn(x)", "x", 0.0);
        assert!(out.starts_with("Error:"), "got {out}");
    }
}
