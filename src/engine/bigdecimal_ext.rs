//! Helpers for interoperating with `bigdecimal::BigDecimal` as a drop-in for
//! `java.math.BigDecimal` + `MathContext.DECIMAL128`.
//!
//! Java `toPlainString()` vs `toString()`: the `bigdecimal` crate's `Display` already
//! produces plain notation without scientific suffixes, matching `toPlainString()`.

use std::sync::LazyLock;

use bigdecimal::BigDecimal;

/// DECIMAL128 precision — 34 significant digits — matches `MathContext.DECIMAL128`.
pub const DECIMAL128_PRECISION: u64 = 34;

/// Scale used for division across financial/electronics tools (20 decimal places, `HALF_UP`).
pub const DIVISION_SCALE: i64 = 20;

/// 2π with 40 fractional digits — sufficient for DECIMAL128 multiplications.
pub static TWO_PI: LazyLock<BigDecimal> = LazyLock::new(|| {
    "6.2831853071795864769252867665590057683943"
        .parse()
        .expect("valid TWO_PI literal")
});

/// π with 40 fractional digits.
pub static PI: LazyLock<BigDecimal> = LazyLock::new(|| {
    "3.1415926535897932384626433832795028841972"
        .parse()
        .expect("valid PI literal")
});

/// 1/ln(2) — used by 555-timer astable computations.
pub static LN2_RECIPROCAL: LazyLock<BigDecimal> = LazyLock::new(|| {
    "1.44269504088896340735992468100189213742665"
        .parse()
        .expect("valid 1/ln(2) literal")
});

/// Digit count beyond which plain notation would spell out hundreds of
/// trailing zeros; values past this threshold fall back to scientific form.
/// Matches Java `BigDecimal::toString` behaviour (switches at scale=-6 for
/// positive exponents, but tools here prefer a larger cap so typical finance
/// and power-of-ten outputs stay plain).
const PLAIN_MAX_DIGITS: usize = 40;

/// Strip trailing zeros and render as plain (non-scientific) decimal.
///
/// Matches Java `value.stripTrailingZeros().toPlainString()` except that a pure
/// integer result is printed without a trailing `.0`.
///
/// Zero is always rendered as the literal `"0"`; `BigDecimal` otherwise spells
/// out `0E100 - 0E100` as 101 literal zeros. Values whose plain form would
/// exceed [`PLAIN_MAX_DIGITS`] (e.g. `1e100`) fall back to scientific notation
/// so the response envelope stays compact.
#[must_use]
pub fn strip_plain(value: &BigDecimal) -> String {
    use num_traits::Zero;
    if value.is_zero() {
        return "0".to_string();
    }
    let normalized = value.normalized();
    let plain = normalized.to_plain_string();
    // `plain.len()` counts the leading `-` and any `.`, but that's good enough
    // as a size heuristic — we want to bail out well before 100-digit runs.
    if plain.len() > PLAIN_MAX_DIGITS {
        return normalized.to_scientific_notation();
    }
    plain
}

/// True iff the value is exactly zero.
#[must_use]
pub fn is_zero(value: &BigDecimal) -> bool {
    use num_traits::Zero;
    value.is_zero()
}
