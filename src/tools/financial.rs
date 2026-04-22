//! Port of `FinancialCalculatorTool.java` — arbitrary-precision financial math
//! matching Java `BigDecimal` + `MathContext.DECIMAL128` semantics (34 digits,
//! `HALF_UP` rounding).
//!
//! Every public entry point mirrors the Java MCP contract: it returns a `String` and
//! encodes validation failures inline as `"Error: ..."`.

use std::num::NonZeroU64;
use std::str::FromStr;

use bigdecimal::{BigDecimal, Context, RoundingMode};
use num_traits::{ToPrimitive, Zero};
use serde::Serialize;

use crate::engine::bigdecimal_ext::{DECIMAL128_PRECISION, DIVISION_SCALE, strip_plain};

const DISPLAY_SCALE: i64 = 2;
const MONTHS_PER_YEAR: i64 = 12;

/// DECIMAL128 context: 34 significant digits with HALF_UP rounding.
fn decimal128_ctx() -> Context {
    Context::default()
        .with_prec(DECIMAL128_PRECISION)
        .expect("DECIMAL128_PRECISION is non-zero")
        .with_rounding_mode(RoundingMode::HalfUp)
}

fn hundred() -> BigDecimal {
    BigDecimal::from(100)
}

fn one() -> BigDecimal {
    BigDecimal::from(1)
}

/// Parse a decimal string into a `BigDecimal`, returning a Java-style error message.
fn parse(input: &str, field: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(input).map_err(|_| format!("Invalid {field}: '{input}'"))
}

fn validate_positive(value: &BigDecimal, name: &str) -> Result<(), String> {
    if value <= &BigDecimal::zero() {
        Err(format!("{name} must be greater than zero"))
    } else {
        Ok(())
    }
}

fn validate_non_negative(value: &BigDecimal, name: &str) -> Result<(), String> {
    if value < &BigDecimal::zero() {
        Err(format!("{name} must not be negative"))
    } else {
        Ok(())
    }
}

/// `a + b` at DECIMAL128 precision (mirrors Java `.add(b, PRECISION)`).
fn add_ctx(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    (a + b).with_precision_round(
        NonZeroU64::new(DECIMAL128_PRECISION).expect("non-zero"),
        RoundingMode::HalfUp,
    )
}

/// `a - b` at DECIMAL128 precision.
fn sub_ctx(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    (a - b).with_precision_round(
        NonZeroU64::new(DECIMAL128_PRECISION).expect("non-zero"),
        RoundingMode::HalfUp,
    )
}

/// `a * b` at DECIMAL128 precision.
fn mul_ctx(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    decimal128_ctx().multiply(a, b)
}

/// Integer-power under DECIMAL128 context (matches Java `.pow(int, PRECISION)`).
fn pow_ctx(base: &BigDecimal, exp: i64) -> BigDecimal {
    base.powi_with_context(exp, &decimal128_ctx())
}

/// `a / b` at scale 20 with HALF_UP (matches Java `.divide(b, INTERNAL_SCALE, HALF_UP)`).
fn div_scale(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    (a / b).with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp)
}

/// Convert a `BigDecimal` that represents an exact integer into `i64`.
fn int_value_exact(value: &BigDecimal, field: &str) -> Result<i64, String> {
    let normalized = value.normalized();
    // A BigDecimal is an exact integer iff its scale ≤ 0 (or equivalently the
    // fractional part is empty after normalization).
    let fractional = &normalized - normalized.with_scale(0);
    if !fractional.is_zero() {
        return Err(format!("{field} must be an integer"));
    }
    normalized
        .to_i64()
        .ok_or_else(|| format!("{field} is out of i64 range"))
}

// --------------------------------------------------------------------------- //
//  Public tool entry points
// --------------------------------------------------------------------------- //

/// Compound interest: `A = P * (1 + r/n)^(n*t)`.
pub fn compound_interest(
    principal: &str,
    annual_rate: &str,
    years: &str,
    compounds_per_year: i64,
) -> String {
    match compute_compound_interest(principal, annual_rate, years, compounds_per_year) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_compound_interest(
    principal: &str,
    annual_rate: &str,
    years: &str,
    compounds_per_year: i64,
) -> Result<String, String> {
    let principal_amt = parse(principal, "principal")?;
    let rate = parse(annual_rate, "annual rate")?;
    let years_dec = parse(years, "years")?;

    validate_positive(&principal_amt, "Principal")?;
    validate_non_negative(&rate, "Annual rate")?;
    validate_positive(&years_dec, "Years")?;
    if compounds_per_year <= 0 {
        return Err("Compounds per year must be greater than zero".to_string());
    }
    let compounds_count = BigDecimal::from(compounds_per_year);

    let annual_rate_dec = div_scale(&rate, &hundred());
    let rate_over_comp = div_scale(&annual_rate_dec, &compounds_count);
    let one_plus_rate = add_ctx(&one(), &rate_over_comp);
    let total_compounds_dec = mul_ctx(&compounds_count, &years_dec);
    let total_compounds = int_value_exact(&total_compounds_dec, "Compounds * years")?;

    let result = mul_ctx(&principal_amt, &pow_ctx(&one_plus_rate, total_compounds));
    Ok(strip_plain(&result))
}

/// Monthly loan payment (fixed-rate amortizing loan).
pub fn loan_payment(principal: &str, annual_rate: &str, years: &str) -> String {
    match compute_loan_payment(principal, annual_rate, years) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_loan_payment(principal: &str, annual_rate: &str, years: &str) -> Result<String, String> {
    let principal_amt = parse(principal, "principal")?;
    let rate = parse(annual_rate, "annual rate")?;
    let years_dec = parse(years, "years")?;

    validate_positive(&principal_amt, "Principal")?;
    validate_positive(&years_dec, "Years")?;

    let months = mul_ctx(&years_dec, &BigDecimal::from(MONTHS_PER_YEAR));
    let total_months = int_value_exact(&months, "Total months")?;

    if rate.is_zero() {
        let payment = div_scale(&principal_amt, &BigDecimal::from(total_months));
        return Ok(strip_plain(&payment));
    }

    let monthly_rate = div_scale(
        &div_scale(&rate, &hundred()),
        &BigDecimal::from(MONTHS_PER_YEAR),
    );
    let one_plus_r = add_ctx(&one(), &monthly_rate);
    let one_plus_r_pow_n = pow_ctx(&one_plus_r, total_months);

    let numerator = mul_ctx(&mul_ctx(&principal_amt, &monthly_rate), &one_plus_r_pow_n);
    let denominator = sub_ctx(&one_plus_r_pow_n, &one());

    let payment = div_scale(&numerator, &denominator);
    Ok(strip_plain(&payment))
}

/// Present value of a future amount: `PV = FV / (1 + r)^t`.
pub fn present_value(future_value: &str, annual_rate: &str, years: &str) -> String {
    match compute_present_value(future_value, annual_rate, years) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_present_value(
    future_value: &str,
    annual_rate: &str,
    years: &str,
) -> Result<String, String> {
    let future_val = parse(future_value, "future value")?;
    let rate = parse(annual_rate, "annual rate")?;
    let years_dec = parse(years, "years")?;

    validate_positive(&future_val, "Future value")?;
    validate_positive(&years_dec, "Years")?;

    let annual_rate_dec = div_scale(&rate, &hundred());
    let one_plus_r = add_ctx(&one(), &annual_rate_dec);
    let exponent = int_value_exact(&years_dec, "Years")?;
    let divisor = pow_ctx(&one_plus_r, exponent);

    let present_val = div_scale(&future_val, &divisor);
    Ok(strip_plain(&present_val))
}

/// Future value of an ordinary annuity: `FV = PMT * ((1+r)^n - 1) / r`.
pub fn future_value_annuity(payment: &str, annual_rate: &str, years: &str) -> String {
    match compute_future_value_annuity(payment, annual_rate, years) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_future_value_annuity(
    payment: &str,
    annual_rate: &str,
    years: &str,
) -> Result<String, String> {
    let pmt = parse(payment, "payment")?;
    let rate = parse(annual_rate, "annual rate")?;
    let years_dec = parse(years, "years")?;

    validate_positive(&pmt, "Payment")?;
    validate_positive(&years_dec, "Years")?;

    if rate.is_zero() {
        let future_val = mul_ctx(&pmt, &years_dec);
        return Ok(strip_plain(&future_val));
    }

    let annual_rate_dec = div_scale(&rate, &hundred());
    let exponent = int_value_exact(&years_dec, "Years")?;
    let one_plus_r_pow_n = pow_ctx(&add_ctx(&one(), &annual_rate_dec), exponent);
    let numerator = sub_ctx(&one_plus_r_pow_n, &one());

    let future_val = div_scale(&mul_ctx(&pmt, &numerator), &annual_rate_dec);
    Ok(strip_plain(&future_val))
}

/// Return on investment as a percentage: `ROI = (gain - cost) / cost * 100`.
pub fn return_on_investment(gain: &str, cost: &str) -> String {
    match compute_roi(gain, cost) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_roi(gain: &str, cost: &str) -> Result<String, String> {
    let gain_amount = parse(gain, "gain")?;
    let cost_amount = parse(cost, "cost")?;

    if cost_amount.is_zero() {
        return Err("Cost must not be zero".to_string());
    }

    let diff = sub_ctx(&gain_amount, &cost_amount);
    let ratio = div_scale(&diff, &cost_amount);
    let roi = mul_ctx(&ratio, &hundred());
    Ok(strip_plain(&roi))
}

/// JSON row representing a single month in the amortization schedule.
#[derive(Serialize)]
struct AmortRow {
    month: i64,
    payment: String,
    principal: String,
    interest: String,
    balance: String,
}

/// Generate a monthly amortization schedule as a JSON array.
pub fn amortization_schedule(principal: &str, annual_rate: &str, years: &str) -> String {
    match compute_amortization(principal, annual_rate, years) {
        Ok(s) => s,
        Err(err) => format!("Error: {err}"),
    }
}

fn compute_amortization(principal: &str, annual_rate: &str, years: &str) -> Result<String, String> {
    let principal_amt = parse(principal, "principal")?;
    let rate = parse(annual_rate, "annual rate")?;
    let years_dec = parse(years, "years")?;

    validate_positive(&principal_amt, "Principal")?;
    validate_positive(&years_dec, "Years")?;

    let months = mul_ctx(&years_dec, &BigDecimal::from(MONTHS_PER_YEAR));
    let total_months = int_value_exact(&months, "Total months")?;

    let (monthly_rate, monthly_payment) = if rate.is_zero() {
        let payment = div_scale(&principal_amt, &BigDecimal::from(total_months));
        (BigDecimal::zero(), payment)
    } else {
        let rate_monthly = div_scale(
            &div_scale(&rate, &hundred()),
            &BigDecimal::from(MONTHS_PER_YEAR),
        );
        let one_plus_r = add_ctx(&one(), &rate_monthly);
        let one_plus_r_pow_n = pow_ctx(&one_plus_r, total_months);
        let numerator = mul_ctx(&mul_ctx(&principal_amt, &rate_monthly), &one_plus_r_pow_n);
        let denominator = sub_ctx(&one_plus_r_pow_n, &one());
        let payment = div_scale(&numerator, &denominator);
        (rate_monthly, payment)
    };

    let mut balance = principal_amt.clone();
    let mut rows: Vec<AmortRow> = Vec::with_capacity(total_months as usize);

    for month in 1..=total_months {
        let interest =
            mul_ctx(&balance, &monthly_rate).with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp);

        let (pmt_amount, principal_part) = if month == total_months {
            let principal_part = balance.clone();
            let pmt_amount = add_ctx(&principal_part, &interest);
            balance = BigDecimal::zero();
            (pmt_amount, principal_part)
        } else {
            let pmt_amount = monthly_payment.clone();
            let principal_part = sub_ctx(&pmt_amount, &interest);
            balance = sub_ctx(&balance, &principal_part);
            (pmt_amount, principal_part)
        };

        rows.push(AmortRow {
            month,
            payment: format_currency(&pmt_amount),
            principal: format_currency(&principal_part),
            interest: format_currency(&interest),
            balance: format_currency(&balance),
        });
    }

    serde_json::to_string(&rows).map_err(|e| format!("JSON serialization failed: {e}"))
}

/// Format a `BigDecimal` as a 2-decimal currency string (HALF_UP).
fn format_currency(value: &BigDecimal) -> String {
    value
        .with_scale_round(DISPLAY_SCALE, RoundingMode::HalfUp)
        .to_plain_string()
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // ---- compound_interest ----

    #[test]
    fn compound_interest_annual() {
        // $1000 at 5% annual, 10 years, compounded annually.
        // Reference (Python decimal): 1000 * (1.05)^10 = 1628.894626777442
        let out = compound_interest("1000", "5", "10", 1);
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 1628.894626777442).abs() < 1e-9, "got {out}");
    }

    #[test]
    fn compound_interest_monthly() {
        // $1000 at 6% APR, 2 years, compounded monthly.
        // 1000 * (1 + 0.06/12)^24 ≈ 1127.15966...
        let out = compound_interest("1000", "6", "2", 12);
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 1127.15966).abs() < 1e-3, "got {out}");
    }

    #[test]
    fn compound_interest_zero_rate() {
        let out = compound_interest("500", "0", "5", 4);
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 500.0).abs() < 1e-9, "got {out}");
    }

    #[test]
    fn compound_interest_negative_principal() {
        let out = compound_interest("-100", "5", "1", 1);
        assert_eq!(out, "Error: Principal must be greater than zero");
    }

    #[test]
    fn compound_interest_zero_compounds() {
        let out = compound_interest("1000", "5", "1", 0);
        assert_eq!(out, "Error: Compounds per year must be greater than zero");
    }

    // ---- loan_payment ----

    #[test]
    fn loan_payment_standard() {
        // $100,000 at 6% APR over 30 years.
        // Python decimal: monthly rate = 0.005; n = 360.
        //   payment = 100000 * 0.005 * (1.005)^360 / ((1.005)^360 - 1)
        //           ≈ 599.550548...
        let out = loan_payment("100000", "6", "30");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 599.550548).abs() < 1e-3, "got {out}");
    }

    #[test]
    fn loan_payment_zero_rate() {
        // $1200, 0% interest, 1 year → $100/month
        let out = loan_payment("1200", "0", "1");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 100.0).abs() < 1e-9, "got {out}");
    }

    #[test]
    fn loan_payment_zero_principal() {
        let out = loan_payment("0", "5", "10");
        assert_eq!(out, "Error: Principal must be greater than zero");
    }

    // ---- present_value ----

    #[test]
    fn present_value_basic() {
        // PV of $1000 in 5 years at 8% = 1000 / 1.08^5 ≈ 680.5832...
        let out = present_value("1000", "8", "5");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 680.58320).abs() < 1e-3, "got {out}");
    }

    // ---- future_value_annuity ----

    #[test]
    fn future_value_annuity_basic() {
        // $100/yr at 7% for 10 years: 100 * ((1.07^10 - 1)/0.07) ≈ 1381.644796...
        let out = future_value_annuity("100", "7", "10");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 1381.644796).abs() < 1e-3, "got {out}");
    }

    #[test]
    fn future_value_annuity_zero_rate() {
        // Zero rate: pmt * years. 200 * 5 = 1000.
        let out = future_value_annuity("200", "0", "5");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 1000.0).abs() < 1e-9, "got {out}");
    }

    // ---- return_on_investment ----

    #[test]
    fn roi_basic() {
        // (150 - 100) / 100 * 100 = 50
        let out = return_on_investment("150", "100");
        let parsed: f64 = out.parse().unwrap();
        assert!((parsed - 50.0).abs() < 1e-9, "got {out}");
    }

    #[test]
    fn roi_zero_cost_error() {
        let out = return_on_investment("100", "0");
        assert_eq!(out, "Error: Cost must not be zero");
    }

    // ---- amortization_schedule ----

    #[test]
    fn amortization_schedule_shape_and_first_month() {
        // $10,000 at 6% APR over 1 year = 12 rows.
        let out = amortization_schedule("10000", "6", "1");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = parsed.as_array().expect("is array");
        assert_eq!(arr.len(), 12);

        let first = &arr[0];
        assert_eq!(first["month"].as_i64().unwrap(), 1);
        // Monthly rate = 0.5% ; interest month 1 = 10000 * 0.005 = 50.00
        assert_eq!(first["interest"].as_str().unwrap(), "50.00");

        // payment ≈ 860.66 → principal = 860.66 - 50.00 ≈ 810.66
        let interest: f64 = first["interest"].as_str().unwrap().parse().unwrap();
        let principal: f64 = first["principal"].as_str().unwrap().parse().unwrap();
        let payment: f64 = first["payment"].as_str().unwrap().parse().unwrap();
        let balance: f64 = first["balance"].as_str().unwrap().parse().unwrap();
        assert!((payment - (interest + principal)).abs() < 0.02);
        assert!((balance - (10_000.0 - principal)).abs() < 0.02);
    }

    #[test]
    fn amortization_schedule_last_month_zero_balance() {
        let out = amortization_schedule("5000", "5", "1");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = parsed.as_array().unwrap();
        let last = arr.last().unwrap();
        assert_eq!(last["balance"].as_str().unwrap(), "0.00");
    }

    #[test]
    fn amortization_schedule_zero_rate() {
        // Zero-rate: all interest = 0.00, equal principal per month.
        let out = amortization_schedule("1200", "0", "1");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 12);
        for row in arr {
            assert_eq!(row["interest"].as_str().unwrap(), "0.00");
        }
    }
}
