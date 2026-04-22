//! Port of `GraphingCalculatorTool.java` — plotting, Newton-Raphson root solving, and bracketed
//! root finding. Expression evaluation is delegated to [`crate::engine::expression`], ensuring
//! exact parity with the Java `ExpressionEvaluator` (degrees-mode trig, IEEE-754 semantics).

use std::collections::HashMap;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};

use crate::engine::bigdecimal_ext::DECIMAL128_PRECISION;
use crate::engine::expression::evaluate_with_variables;

const ERR_PREFIX: &str = "Error: ";
const MAX_NEWTON_ITERS: i32 = 1000;
const NEWTON_TOLERANCE: f64 = 1e-12;
const DERIVATIVE_STEP: f64 = 1e-8;
const BISECT_ITERS: i32 = 50;
const SCAN_DIVISIONS: i32 = 1000;

// --------------------------------------------------------------------------- //
//  plot_function
// --------------------------------------------------------------------------- //

/// Sample `expression` at `steps + 1` equally spaced points between `min` and `max`.
/// Returns a JSON array of `{"x": f64, "y": f64}` objects.
pub fn plot_function(expression: &str, variable: &str, min: f64, max: f64, steps: i32) -> String {
    if steps <= 0 {
        return format!("{ERR_PREFIX}Steps must be greater than 0");
    }
    if min >= max {
        return format!("{ERR_PREFIX}Min must be less than max");
    }

    let bd_min = match BigDecimal::from_f64(min) {
        Some(v) => v,
        None => return format!("{ERR_PREFIX}Invalid min: {min}"),
    };
    let bd_max = match BigDecimal::from_f64(max) {
        Some(v) => v,
        None => return format!("{ERR_PREFIX}Invalid max: {max}"),
    };
    let bd_steps = BigDecimal::from(steps);
    let step_size = (&bd_max - &bd_min).with_prec(DECIMAL128_PRECISION) / bd_steps;

    let mut points: Vec<serde_json::Value> = Vec::with_capacity((steps + 1) as usize);

    for idx in 0..=steps {
        let idx_bd = BigDecimal::from(idx);
        let x_bd = &bd_min + &step_size * idx_bd;
        let x = x_bd.to_f64().unwrap_or(f64::NAN);

        let mut vars = HashMap::with_capacity(1);
        vars.insert(variable.to_string(), x);
        let y = match evaluate_with_variables(expression, &vars) {
            Ok(v) => v,
            Err(e) => return format!("{ERR_PREFIX}{e}"),
        };

        points.push(serde_json::json!({"x": x, "y": y}));
    }

    serde_json::to_string(&points).unwrap_or_else(|e| format!("{ERR_PREFIX}{e}"))
}

// --------------------------------------------------------------------------- //
//  solve_equation (Newton-Raphson with central-difference derivative)
// --------------------------------------------------------------------------- //

/// Newton-Raphson solver. Returns the root as a stringified `f64`, or `"Error: ..."`.
pub fn solve_equation(expression: &str, variable: &str, initial_guess: f64) -> String {
    let mut guess = initial_guess;

    for _ in 0..MAX_NEWTON_ITERS {
        let f_value = match eval_at(expression, variable, guess) {
            Ok(v) => v,
            Err(msg) => return msg,
        };
        if f_value.abs() < NEWTON_TOLERANCE {
            return guess.to_string();
        }

        let f_plus = match eval_at(expression, variable, guess + DERIVATIVE_STEP) {
            Ok(v) => v,
            Err(msg) => return msg,
        };
        let f_minus = match eval_at(expression, variable, guess - DERIVATIVE_STEP) {
            Ok(v) => v,
            Err(msg) => return msg,
        };
        let derivative = (f_plus - f_minus) / (2.0 * DERIVATIVE_STEP);
        if derivative == 0.0 {
            return format!("{ERR_PREFIX}Derivative is zero; Newton-Raphson cannot continue");
        }

        guess -= f_value / derivative;
    }

    format!("{ERR_PREFIX}Failed to converge after {MAX_NEWTON_ITERS} iterations")
}

// --------------------------------------------------------------------------- //
//  find_roots (scan + bisect)
// --------------------------------------------------------------------------- //

/// Scan `[min, max]` in `SCAN_DIVISIONS` slices, detecting sign changes and
/// already-at-root samples. Refines bracketed intervals with 50 bisection steps.
/// Returns a JSON array of `f64` roots.
pub fn find_roots(expression: &str, variable: &str, min: f64, max: f64) -> String {
    let step = (max - min) / f64::from(SCAN_DIVISIONS);
    let mut roots: Vec<f64> = Vec::new();

    let mut prev_x = min;
    let mut prev_f = match eval_at(expression, variable, prev_x) {
        Ok(v) => v,
        Err(msg) => return msg,
    };

    if prev_f.abs() < NEWTON_TOLERANCE {
        roots.push(prev_x);
    }

    for idx in 1..=SCAN_DIVISIONS {
        let current_x = min + f64::from(idx) * step;
        let current_f = match eval_at(expression, variable, current_x) {
            Ok(v) => v,
            Err(msg) => return msg,
        };

        if current_f.abs() < NEWTON_TOLERANCE {
            push_unique(&mut roots, current_x);
        } else if prev_f * current_f < 0.0 {
            match bisect(expression, variable, prev_x, current_x) {
                Ok(root) => push_unique(&mut roots, root),
                Err(msg) => return msg,
            }
        }

        prev_x = current_x;
        prev_f = current_f;
    }

    serde_json::to_string(&roots).unwrap_or_else(|e| format!("{ERR_PREFIX}{e}"))
}

fn push_unique(roots: &mut Vec<f64>, candidate: f64) {
    if roots
        .iter()
        .any(|existing| (existing - candidate).abs() < 1e-6)
    {
        return;
    }
    roots.push(candidate);
}

fn bisect(
    expression: &str,
    variable: &str,
    lower_bound: f64,
    upper_bound: f64,
) -> Result<f64, String> {
    let mut lower = lower_bound;
    let mut upper = upper_bound;

    for _ in 0..BISECT_ITERS {
        let mid = (lower + upper) / 2.0;
        let f_mid = eval_at(expression, variable, mid)?;
        if f_mid.abs() < NEWTON_TOLERANCE {
            return Ok(mid);
        }
        let f_lo = eval_at(expression, variable, lower)?;
        if f_lo * f_mid < 0.0 {
            upper = mid;
        } else {
            lower = mid;
        }
    }
    Ok((lower + upper) / 2.0)
}

fn eval_at(expression: &str, variable: &str, x: f64) -> Result<f64, String> {
    let mut vars = HashMap::with_capacity(1);
    vars.insert(variable.to_string(), x);
    evaluate_with_variables(expression, &vars).map_err(|e| format!("{ERR_PREFIX}{e}"))
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_x_squared_endpoints() {
        let out = plot_function("x^2", "x", -2.0, 2.0, 4);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = v.as_array().expect("array");
        assert_eq!(arr.len(), 5);
        assert!((arr[0]["x"].as_f64().unwrap() + 2.0).abs() < 1e-9);
        assert!((arr[0]["y"].as_f64().unwrap() - 4.0).abs() < 1e-9);
        assert!((arr[4]["x"].as_f64().unwrap() - 2.0).abs() < 1e-9);
        assert!((arr[4]["y"].as_f64().unwrap() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn plot_invalid_steps() {
        assert_eq!(
            plot_function("x", "x", 0.0, 1.0, 0),
            "Error: Steps must be greater than 0"
        );
        assert_eq!(
            plot_function("x", "x", 0.0, 1.0, -5),
            "Error: Steps must be greater than 0"
        );
    }

    #[test]
    fn plot_invalid_range() {
        assert_eq!(
            plot_function("x", "x", 5.0, 1.0, 10),
            "Error: Min must be less than max"
        );
        assert_eq!(
            plot_function("x", "x", 1.0, 1.0, 10),
            "Error: Min must be less than max"
        );
    }

    #[test]
    fn plot_bubbles_expression_error() {
        let out = plot_function("unknown_var", "x", 0.0, 1.0, 2);
        assert!(out.starts_with("Error:"), "got {out}");
    }

    #[test]
    fn solve_x_squared_minus_four_positive_guess() {
        let out = solve_equation("x^2 - 4", "x", 3.0);
        let root: f64 = out
            .parse()
            .unwrap_or_else(|_| panic!("expected numeric, got {out}"));
        assert!((root - 2.0).abs() < 1e-6, "got {root}");
    }

    #[test]
    fn solve_x_squared_minus_four_negative_guess() {
        let out = solve_equation("x^2 - 4", "x", -3.0);
        let root: f64 = out
            .parse()
            .unwrap_or_else(|_| panic!("expected numeric, got {out}"));
        assert!((root + 2.0).abs() < 1e-6, "got {root}");
    }

    #[test]
    fn solve_derivative_zero_error() {
        // f(x) = 5 is constant, derivative is 0 everywhere.
        let out = solve_equation("5 - 5 + 0*x + 1", "x", 0.0);
        assert!(
            out == "Error: Derivative is zero; Newton-Raphson cannot continue",
            "got {out}"
        );
    }

    #[test]
    fn find_roots_x_squared_minus_four() {
        let out = find_roots("x^2 - 4", "x", -5.0, 5.0);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = v.as_array().expect("array");
        assert_eq!(arr.len(), 2, "expected two roots, got {out}");
        let mut roots: Vec<f64> = arr.iter().map(|e| e.as_f64().unwrap()).collect();
        roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((roots[0] + 2.0).abs() < 1e-6, "got {roots:?}");
        assert!((roots[1] - 2.0).abs() < 1e-6, "got {roots:?}");
    }

    #[test]
    fn find_roots_no_roots_returns_empty() {
        let out = find_roots("x^2 + 1", "x", -5.0, 5.0);
        assert_eq!(out, "[]");
    }

    #[test]
    fn find_roots_cubic_three_roots() {
        // x^3 - x has roots at -1, 0, 1
        let out = find_roots("x^3 - x", "x", -2.0, 2.0);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = v.as_array().expect("array");
        let mut roots: Vec<f64> = arr.iter().map(|e| e.as_f64().unwrap()).collect();
        roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(roots.len(), 3, "got {roots:?}");
        assert!((roots[0] + 1.0).abs() < 1e-6, "got {roots:?}");
        assert!(roots[1].abs() < 1e-6, "got {roots:?}");
        assert!((roots[2] - 1.0).abs() < 1e-6, "got {roots:?}");
    }

    #[test]
    fn find_roots_bubbles_expression_error() {
        let out = find_roots("bogus_var", "x", -1.0, 1.0);
        assert!(out.starts_with("Error:"), "got {out}");
    }

    #[test]
    fn solve_bubbles_expression_error() {
        let out = solve_equation("bogus_var", "x", 1.0);
        assert!(out.starts_with("Error:"), "got {out}");
    }
}
