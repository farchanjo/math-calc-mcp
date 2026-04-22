//! Port of `VectorCalculatorTool.java` — SIMD-accelerated array arithmetic.
//!
//! Uses the portable-SIMD [`wide`] crate which auto-dispatches to SSE2/AVX2/AVX-512/NEON
//! at runtime based on `target-cpu` flags. A 256-bit `f64x4` vector width is used for
//! the hot inner loop; the tail is processed scalar.
//!
//! Every public entry point mirrors the Java MCP contract: it returns a `String` and
//! encodes failures inline as `"Error: ..."` (matching the Java tool's exception
//! messages verbatim).

use wide::f64x4;

const EMPTY_ARRAY_MSG: &str = "Input array must not be empty";

/// Parse a comma-separated list of f64 values.
fn parse_array(input: &str) -> Result<Vec<f64>, String> {
    let parts: Vec<&str> = input.split(',').collect();
    let mut result = Vec::with_capacity(parts.len());
    for part in parts {
        let trimmed = part.trim();
        match trimmed.parse::<f64>() {
            Ok(value) => result.push(value),
            Err(err) => return Err(format!("Invalid number '{trimmed}': {err}")),
        }
    }
    Ok(result)
}

/// Format a single f64 the same way Java's `String.valueOf(double)` does — `{:?}`
/// in Rust yields `1.0` for whole doubles (matching Java) and falls back to the
/// shortest round-trip representation otherwise.
fn format_f64(value: f64) -> String {
    format!("{value:?}")
}

/// Sum all elements of a numeric array.
///
/// Java: `VectorCalculatorTool.sumArray`.
pub fn sum_array(numbers: &str) -> String {
    let array = match parse_array(numbers) {
        Ok(arr) => arr,
        Err(err) => return format!("Error: {err}"),
    };
    if array.is_empty() {
        return format!("Error: {EMPTY_ARRAY_MSG}");
    }

    let mut acc = f64x4::splat(0.0);
    let lanes = 4;
    let bound = array.len() - (array.len() % lanes);
    let mut idx = 0;
    while idx < bound {
        let chunk = f64x4::new([array[idx], array[idx + 1], array[idx + 2], array[idx + 3]]);
        acc += chunk;
        idx += lanes;
    }
    let mut result = acc.reduce_add();
    while idx < array.len() {
        result += array[idx];
        idx += 1;
    }
    format_f64(result)
}

/// Dot product of two arrays of equal length.
///
/// Java: `VectorCalculatorTool.dotProduct`.
pub fn dot_product(first: &str, second: &str) -> String {
    let array_a = match parse_array(first) {
        Ok(arr) => arr,
        Err(err) => return format!("Error: {err}"),
    };
    let array_b = match parse_array(second) {
        Ok(arr) => arr,
        Err(err) => return format!("Error: {err}"),
    };
    if array_a.is_empty() || array_b.is_empty() {
        return format!("Error: {EMPTY_ARRAY_MSG}");
    }
    if array_a.len() != array_b.len() {
        return format!(
            "Error: Arrays must be same length. Got {} and {}",
            array_a.len(),
            array_b.len()
        );
    }

    let mut acc = f64x4::splat(0.0);
    let lanes = 4;
    let bound = array_a.len() - (array_a.len() % lanes);
    let mut idx = 0;
    while idx < bound {
        let vector_a = f64x4::new([
            array_a[idx],
            array_a[idx + 1],
            array_a[idx + 2],
            array_a[idx + 3],
        ]);
        let vector_b = f64x4::new([
            array_b[idx],
            array_b[idx + 1],
            array_b[idx + 2],
            array_b[idx + 3],
        ]);
        acc += vector_a * vector_b;
        idx += lanes;
    }
    let mut result = acc.reduce_add();
    while idx < array_a.len() {
        result += array_a[idx] * array_b[idx];
        idx += 1;
    }
    format_f64(result)
}

/// Multiply every element by a scalar, returning a CSV string.
///
/// Java: `VectorCalculatorTool.scaleArray`.
pub fn scale_array(numbers: &str, scalar: &str) -> String {
    let array = match parse_array(numbers) {
        Ok(arr) => arr,
        Err(err) => return format!("Error: {err}"),
    };
    if array.is_empty() {
        return format!("Error: {EMPTY_ARRAY_MSG}");
    }
    let factor = match scalar.trim().parse::<f64>() {
        Ok(value) => value,
        Err(err) => return format!("Error: Invalid scalar '{}': {err}", scalar.trim()),
    };

    let mut result = vec![0.0_f64; array.len()];
    let v_scalar = f64x4::splat(factor);
    let lanes = 4;
    let bound = array.len() - (array.len() % lanes);
    let mut idx = 0;
    while idx < bound {
        let vector_a = f64x4::new([array[idx], array[idx + 1], array[idx + 2], array[idx + 3]]);
        let scaled = (vector_a * v_scalar).to_array();
        result[idx] = scaled[0];
        result[idx + 1] = scaled[1];
        result[idx + 2] = scaled[2];
        result[idx + 3] = scaled[3];
        idx += lanes;
    }
    while idx < array.len() {
        result[idx] = array[idx] * factor;
        idx += 1;
    }

    result
        .iter()
        .map(|val| format_f64(*val))
        .collect::<Vec<_>>()
        .join(",")
}

/// Euclidean norm (magnitude) of a vector: `sqrt(sum(x²))`.
///
/// Java: `VectorCalculatorTool.magnitudeArray`.
pub fn magnitude_array(numbers: &str) -> String {
    let array = match parse_array(numbers) {
        Ok(arr) => arr,
        Err(err) => return format!("Error: {err}"),
    };
    if array.is_empty() {
        return format!("Error: {EMPTY_ARRAY_MSG}");
    }

    let mut acc = f64x4::splat(0.0);
    let lanes = 4;
    let bound = array.len() - (array.len() % lanes);
    let mut idx = 0;
    while idx < bound {
        let vector_a = f64x4::new([array[idx], array[idx + 1], array[idx + 2], array[idx + 3]]);
        acc += vector_a * vector_a;
        idx += lanes;
    }
    let mut sum_of_squares = acc.reduce_add();
    while idx < array.len() {
        sum_of_squares += array[idx] * array[idx];
        idx += 1;
    }
    format_f64(sum_of_squares.sqrt())
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_result(s: &str) -> f64 {
        s.parse::<f64>()
            .unwrap_or_else(|_| panic!("not a float: {s}"))
    }

    // ---- sum_array ----

    #[test]
    fn sum_array_basic() {
        let out = sum_array("1,2,3,4,5,6,7,8,9,10");
        assert!((parse_result(&out) - 55.0).abs() < 1e-9);
    }

    #[test]
    fn sum_array_with_fractions() {
        let out = sum_array("1.5,2.5,3.0");
        assert!((parse_result(&out) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn sum_array_tail_only() {
        // 3 elements: no SIMD iterations, pure tail path.
        let out = sum_array("10,20,30");
        assert!((parse_result(&out) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn sum_array_invalid_input() {
        let out = sum_array("1,foo,3");
        assert!(out.starts_with("Error:"), "got: {out}");
    }

    // ---- dot_product ----

    #[test]
    fn dot_product_known_identity() {
        // [1,2,3] · [4,5,6] = 4 + 10 + 18 = 32
        let out = dot_product("1,2,3", "4,5,6");
        assert!((parse_result(&out) - 32.0).abs() < 1e-9);
    }

    #[test]
    fn dot_product_longer_arrays() {
        // [1..8] · [1..8] = 1+4+9+16+25+36+49+64 = 204
        let out = dot_product("1,2,3,4,5,6,7,8", "1,2,3,4,5,6,7,8");
        assert!((parse_result(&out) - 204.0).abs() < 1e-9);
    }

    #[test]
    fn dot_product_mismatched_lengths() {
        let out = dot_product("1,2,3", "4,5");
        assert_eq!(out, "Error: Arrays must be same length. Got 3 and 2");
    }

    #[test]
    fn dot_product_parse_error() {
        let out = dot_product("1,nope", "3,4");
        assert!(out.starts_with("Error:"), "got: {out}");
    }

    // ---- scale_array ----

    #[test]
    fn scale_array_basic() {
        let out = scale_array("1,2,3,4,5", "2");
        assert_eq!(out, "2.0,4.0,6.0,8.0,10.0");
    }

    #[test]
    fn scale_array_with_negative_scalar() {
        let out = scale_array("1.5,-2.5", "-2");
        // 1.5*-2 = -3.0 ; -2.5*-2 = 5.0
        assert_eq!(out, "-3.0,5.0");
    }

    #[test]
    fn scale_array_invalid_scalar() {
        let out = scale_array("1,2,3", "abc");
        assert!(out.starts_with("Error:"), "got: {out}");
    }

    // ---- magnitude_array ----

    #[test]
    fn magnitude_array_pythagoras() {
        // |[3,4]| = 5
        let out = magnitude_array("3,4");
        assert!((parse_result(&out) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn magnitude_array_3d() {
        // |[2,3,6]| = sqrt(4+9+36) = sqrt(49) = 7
        let out = magnitude_array("2,3,6");
        assert!((parse_result(&out) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn magnitude_array_many_elements() {
        // |[1,1,1,1,1,1,1,1]| = sqrt(8) ≈ 2.82842712474619
        let out = magnitude_array("1,1,1,1,1,1,1,1");
        let expected = 8.0_f64.sqrt();
        assert!((parse_result(&out) - expected).abs() < 1e-9);
    }

    #[test]
    fn magnitude_array_empty() {
        let out = magnitude_array("");
        // ""  → parse_array returns [""] → parse error (not empty-array error).
        assert!(out.starts_with("Error:"), "got: {out}");
    }
}
