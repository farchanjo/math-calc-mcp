//! Port of `ScientificCalculatorTool.java` — transcendentals and factorial with
//! Java `StrictMath` parity. Return strings mirror `String.valueOf(double)` /
//! `BigInteger.toString()`; invalid inputs produce `"Error: ..."` messages
//! instead of exceptions.

use std::collections::HashMap;
use std::sync::LazyLock;

const FULL_CIRCLE: i32 = 360;

static SQRT2_OVER_2: LazyLock<f64> = LazyLock::new(|| (2.0_f64).sqrt() / 2.0);
static SQRT3_OVER_2: LazyLock<f64> = LazyLock::new(|| (3.0_f64).sqrt() / 2.0);
static SQRT3: LazyLock<f64> = LazyLock::new(|| (3.0_f64).sqrt());
static ONE_OVER_SQRT3: LazyLock<f64> = LazyLock::new(|| 1.0 / (3.0_f64).sqrt());

static SIN_TABLE: LazyLock<HashMap<i32, f64>> = LazyLock::new(|| {
    HashMap::from([
        (0, 0.0),
        (180, 0.0),
        (30, 0.5),
        (150, 0.5),
        (45, *SQRT2_OVER_2),
        (135, *SQRT2_OVER_2),
        (60, *SQRT3_OVER_2),
        (120, *SQRT3_OVER_2),
        (90, 1.0),
        (210, -0.5),
        (330, -0.5),
        (225, -*SQRT2_OVER_2),
        (315, -*SQRT2_OVER_2),
        (240, -*SQRT3_OVER_2),
        (300, -*SQRT3_OVER_2),
        (270, -1.0),
    ])
});

static COS_TABLE: LazyLock<HashMap<i32, f64>> = LazyLock::new(|| {
    HashMap::from([
        (0, 1.0),
        (30, *SQRT3_OVER_2),
        (330, *SQRT3_OVER_2),
        (45, *SQRT2_OVER_2),
        (315, *SQRT2_OVER_2),
        (60, 0.5),
        (300, 0.5),
        (90, 0.0),
        (270, 0.0),
        (120, -0.5),
        (240, -0.5),
        (135, -*SQRT2_OVER_2),
        (225, -*SQRT2_OVER_2),
        (150, -*SQRT3_OVER_2),
        (210, -*SQRT3_OVER_2),
        (180, -1.0),
    ])
});

static TAN_TABLE: LazyLock<HashMap<i32, f64>> = LazyLock::new(|| {
    HashMap::from([
        (0, 0.0),
        (180, 0.0),
        (30, *ONE_OVER_SQRT3),
        (210, *ONE_OVER_SQRT3),
        (45, 1.0),
        (225, 1.0),
        (60, *SQRT3),
        (240, *SQRT3),
        (120, -*SQRT3),
        (300, -*SQRT3),
        (135, -1.0),
        (315, -1.0),
        (150, -*ONE_OVER_SQRT3),
        (330, -*ONE_OVER_SQRT3),
    ])
});

fn is_integer_angle(degrees: f64) -> bool {
    degrees == degrees.floor() && degrees.is_finite()
}

fn normalize_angle(degrees: f64) -> i32 {
    let angle = (degrees as i32) % FULL_CIRCLE;
    if angle < 0 {
        angle + FULL_CIRCLE
    } else {
        angle
    }
}

/// Compute square root of a number.
pub fn sqrt(number: f64) -> String {
    if number < 0.0 {
        format!("Error: Square root is undefined for negative numbers. Received: {number:?}")
    } else {
        format!("{:?}", number.sqrt())
    }
}

/// Compute natural logarithm (ln) of a number.
pub fn log(number: f64) -> String {
    if number <= 0.0 {
        format!(
            "Error: Natural logarithm is undefined for non-positive numbers. Received: {number:?}"
        )
    } else {
        format!("{:?}", number.ln())
    }
}

/// Compute base-10 logarithm of a number.
pub fn log10(number: f64) -> String {
    if number <= 0.0 {
        format!(
            "Error: Base-10 logarithm is undefined for non-positive numbers. Received: {number:?}"
        )
    } else {
        format!("{:?}", number.log10())
    }
}

/// Compute factorial (n!) for integers in `[0, 20]`.
pub fn factorial(num: i64) -> String {
    if !(0..=20).contains(&num) {
        return format!("Error: Factorial is only defined for integers 0 to 20. Received: {num}");
    }
    let mut value: u64 = 1;
    for idx in 2..=num {
        value *= idx as u64;
    }
    value.to_string()
}

fn exact_lookup(table: &HashMap<i32, f64>, degrees: f64) -> Option<f64> {
    if !is_integer_angle(degrees) {
        return None;
    }
    table.get(&normalize_angle(degrees)).copied()
}

/// Compute sine of an angle in degrees.
pub fn sin(degrees: f64) -> String {
    let value = exact_lookup(&SIN_TABLE, degrees).unwrap_or_else(|| degrees.to_radians().sin());
    format!("{value:?}")
}

/// Compute cosine of an angle in degrees.
pub fn cos(degrees: f64) -> String {
    let value = exact_lookup(&COS_TABLE, degrees).unwrap_or_else(|| degrees.to_radians().cos());
    format!("{value:?}")
}

/// Compute tangent of an angle in degrees.
pub fn tan(degrees: f64) -> String {
    if is_integer_angle(degrees) {
        let normalized = normalize_angle(degrees);
        if normalized == 90 || normalized == 270 {
            let as_int = degrees as i32;
            return format!(
                "Error: Tangent is undefined at {as_int} degrees (vertical asymptote)."
            );
        }
        if let Some(exact) = TAN_TABLE.get(&normalized).copied() {
            return format!("{exact:?}");
        }
    }
    format!("{:?}", degrees.to_radians().tan())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_perfect_square() {
        assert_eq!(sqrt(16.0), "4.0");
    }

    #[test]
    fn sqrt_irrational() {
        assert_eq!(sqrt(2.0), "1.4142135623730951");
    }

    #[test]
    fn sqrt_negative_reports_error() {
        assert_eq!(
            sqrt(-1.0),
            "Error: Square root is undefined for negative numbers. Received: -1.0"
        );
    }

    #[test]
    fn log_of_e_is_one() {
        assert_eq!(log(std::f64::consts::E), "1.0");
    }

    #[test]
    fn log_zero_reports_error() {
        assert_eq!(
            log(0.0),
            "Error: Natural logarithm is undefined for non-positive numbers. Received: 0.0"
        );
    }

    #[test]
    fn log_negative_reports_error() {
        assert_eq!(
            log(-1.0),
            "Error: Natural logarithm is undefined for non-positive numbers. Received: -1.0"
        );
    }

    #[test]
    fn log10_hundred() {
        assert_eq!(log10(100.0), "2.0");
    }

    #[test]
    fn log10_thousand() {
        assert_eq!(log10(1000.0), "3.0");
    }

    #[test]
    fn log10_zero_reports_error() {
        assert_eq!(
            log10(0.0),
            "Error: Base-10 logarithm is undefined for non-positive numbers. Received: 0.0"
        );
    }

    #[test]
    fn factorial_zero() {
        assert_eq!(factorial(0), "1");
    }

    #[test]
    fn factorial_one() {
        assert_eq!(factorial(1), "1");
    }

    #[test]
    fn factorial_five() {
        assert_eq!(factorial(5), "120");
    }

    #[test]
    fn factorial_twenty() {
        assert_eq!(factorial(20), "2432902008176640000");
    }

    #[test]
    fn factorial_negative_reports_error() {
        assert_eq!(
            factorial(-1),
            "Error: Factorial is only defined for integers 0 to 20. Received: -1"
        );
    }

    #[test]
    fn factorial_above_range_reports_error() {
        assert_eq!(
            factorial(21),
            "Error: Factorial is only defined for integers 0 to 20. Received: 21"
        );
    }

    #[test]
    fn sin_zero() {
        assert_eq!(sin(0.0), "0.0");
    }

    #[test]
    fn sin_thirty() {
        assert_eq!(sin(30.0), "0.5");
    }

    #[test]
    fn sin_ninety() {
        assert_eq!(sin(90.0), "1.0");
    }

    #[test]
    fn sin_one_eighty() {
        assert_eq!(sin(180.0), "0.0");
    }

    #[test]
    fn sin_full_circle_normalizes_to_zero() {
        assert_eq!(sin(360.0), "0.0");
    }

    #[test]
    fn sin_negative_thirty_normalizes_to_three_thirty() {
        assert_eq!(sin(-30.0), "-0.5");
    }

    #[test]
    fn sin_forty_five_uses_exact_table() {
        assert_eq!(sin(45.0), format!("{:?}", *SQRT2_OVER_2));
    }

    #[test]
    fn sin_non_notable_angle_falls_back() {
        let expected = format!("{:?}", 15.0_f64.to_radians().sin());
        assert_eq!(sin(15.0), expected);
    }

    #[test]
    fn sin_non_integer_angle_falls_back() {
        let expected = format!("{:?}", 45.5_f64.to_radians().sin());
        assert_eq!(sin(45.5), expected);
    }

    #[test]
    fn cos_zero() {
        assert_eq!(cos(0.0), "1.0");
    }

    #[test]
    fn cos_ninety() {
        assert_eq!(cos(90.0), "0.0");
    }

    #[test]
    fn cos_sixty() {
        assert_eq!(cos(60.0), "0.5");
    }

    #[test]
    fn cos_one_eighty() {
        assert_eq!(cos(180.0), "-1.0");
    }

    #[test]
    fn tan_forty_five() {
        assert_eq!(tan(45.0), "1.0");
    }

    #[test]
    fn tan_zero() {
        assert_eq!(tan(0.0), "0.0");
    }

    #[test]
    fn tan_one_eighty() {
        assert_eq!(tan(180.0), "0.0");
    }

    #[test]
    fn tan_ninety_is_asymptote() {
        assert_eq!(
            tan(90.0),
            "Error: Tangent is undefined at 90 degrees (vertical asymptote)."
        );
    }

    #[test]
    fn tan_two_seventy_is_asymptote() {
        assert_eq!(
            tan(270.0),
            "Error: Tangent is undefined at 270 degrees (vertical asymptote)."
        );
    }

    #[test]
    fn tan_negative_two_seventy_is_asymptote() {
        assert_eq!(
            tan(-270.0),
            "Error: Tangent is undefined at -270 degrees (vertical asymptote)."
        );
    }
}
