//! Port of `AnalogElectronicsTool.java` — electronics computations with
//! `BigDecimal` + `MathContext.DECIMAL128` semantics. Transcendental helpers
//! (`sqrt`, `atan2`, `log10`, `10^x`) use the pure-Rust `astro-float` crate at
//! 128-bit precision (≈ DECIMAL128), so results are accurate well beyond Java
//! `StrictMath` (f64).
//!
//! All public functions return `String`. Errors are embedded as
//! `"Error: {msg}"` to mirror the Java tool's behavior.

use std::cell::RefCell;
use std::num::NonZeroU64;
use std::str::FromStr;

use astro_float::{BigFloat, Consts, Radix, RoundingMode as AfRm};
use bigdecimal::{BigDecimal, RoundingMode};
use num_traits::{Signed, Zero};

use crate::engine::bigdecimal_ext::{DECIMAL128_PRECISION, DIVISION_SCALE, TWO_PI, strip_plain};

const ERROR_PREFIX: &str = "Error: ";
const SERIES: &str = "series";
const PARALLEL: &str = "parallel";
const REQUIRED_KNOWNS: u32 = 2;
const AF_PRECISION: usize = 128;

thread_local! {
    static AF_CONSTS: RefCell<Consts> =
        RefCell::new(Consts::new().expect("failed to initialize astro-float Consts"));
}

fn precision() -> NonZeroU64 {
    NonZeroU64::new(DECIMAL128_PRECISION).expect("precision is non-zero")
}

fn mul_ctx(lhs: &BigDecimal, rhs: &BigDecimal) -> BigDecimal {
    (lhs * rhs).with_precision_round(precision(), RoundingMode::HalfUp)
}

fn add_ctx(lhs: &BigDecimal, rhs: &BigDecimal) -> BigDecimal {
    (lhs + rhs).with_precision_round(precision(), RoundingMode::HalfUp)
}

fn sub_ctx(lhs: &BigDecimal, rhs: &BigDecimal) -> BigDecimal {
    (lhs - rhs).with_precision_round(precision(), RoundingMode::HalfUp)
}

fn div_scaled(lhs: &BigDecimal, rhs: &BigDecimal) -> BigDecimal {
    (lhs / rhs).with_scale_round(DIVISION_SCALE, RoundingMode::HalfUp)
}

// --- astro-float bridge helpers ---

fn bd_to_bf(value: &BigDecimal) -> BigFloat {
    AF_CONSTS.with(|cc| {
        BigFloat::parse(
            &value.to_plain_string(),
            Radix::Dec,
            AF_PRECISION,
            AfRm::None,
            &mut cc.borrow_mut(),
        )
    })
}

fn bf_to_bd(value: &BigFloat) -> BigDecimal {
    let formatted = AF_CONSTS.with(|cc| {
        value
            .format(Radix::Dec, AfRm::ToEven, &mut cc.borrow_mut())
            .expect("astro-float format failed")
    });
    BigDecimal::from_str(&formatted).expect("astro-float output parses as BigDecimal")
}

fn af_sqrt(value: &BigDecimal) -> BigDecimal {
    let bf = bd_to_bf(value);
    bf_to_bd(&bf.sqrt(AF_PRECISION, AfRm::ToEven))
}

fn af_log10(value: &BigDecimal) -> BigDecimal {
    let bf = bd_to_bf(value);
    let out = AF_CONSTS.with(|cc| bf.log10(AF_PRECISION, AfRm::ToEven, &mut cc.borrow_mut()));
    bf_to_bd(&out)
}

fn af_pow10(exponent: &BigDecimal) -> BigDecimal {
    // 10^x = e^(x * ln(10)) ; astro-float provides `pow(base, exp)` with cc.
    let base = AF_CONSTS.with(|cc| {
        BigFloat::parse(
            "10",
            Radix::Dec,
            AF_PRECISION,
            AfRm::None,
            &mut cc.borrow_mut(),
        )
    });
    let exp_bf = bd_to_bf(exponent);
    let out =
        AF_CONSTS.with(|cc| base.pow(&exp_bf, AF_PRECISION, AfRm::ToEven, &mut cc.borrow_mut()));
    bf_to_bd(&out)
}

/// atan2 implemented via `atan` with quadrant correction.
/// Uses a hardcoded 128-bit-precision pi literal (library's `pi_num` is private).
fn af_atan2_degrees(y: &BigDecimal, x: &BigDecimal) -> BigDecimal {
    const PI_LITERAL: &str =
        "3.14159265358979323846264338327950288419716939937510582097494459230781640628620";
    let y_bf = bd_to_bf(y);
    let x_bf = bd_to_bf(x);
    let radians = AF_CONSTS.with(|cc| {
        let mut consts = cc.borrow_mut();
        let pi = BigFloat::parse(
            PI_LITERAL,
            Radix::Dec,
            AF_PRECISION,
            AfRm::None,
            &mut consts,
        );
        let two = BigFloat::parse("2", Radix::Dec, AF_PRECISION, AfRm::None, &mut consts);
        let zero = BigFloat::parse("0", Radix::Dec, AF_PRECISION, AfRm::None, &mut consts);
        let half_pi = pi.div(&two, AF_PRECISION, AfRm::ToEven);
        let x_cmp = x_bf.cmp(&zero).unwrap_or(0);
        let y_cmp = y_bf.cmp(&zero).unwrap_or(0);
        if x_cmp > 0 {
            y_bf.div(&x_bf, AF_PRECISION, AfRm::ToEven).atan(
                AF_PRECISION,
                AfRm::ToEven,
                &mut consts,
            )
        } else if x_cmp < 0 && y_cmp >= 0 {
            let base = y_bf.div(&x_bf, AF_PRECISION, AfRm::ToEven).atan(
                AF_PRECISION,
                AfRm::ToEven,
                &mut consts,
            );
            base.add(&pi, AF_PRECISION, AfRm::ToEven)
        } else if x_cmp < 0 {
            let base = y_bf.div(&x_bf, AF_PRECISION, AfRm::ToEven).atan(
                AF_PRECISION,
                AfRm::ToEven,
                &mut consts,
            );
            base.sub(&pi, AF_PRECISION, AfRm::ToEven)
        } else if y_cmp >= 0 {
            half_pi
        } else {
            half_pi.neg()
        }
    });
    let rad_bd = bf_to_bd(&radians);
    // 180 / π to 40 digits.
    let deg_per_rad = BigDecimal::from_str("57.29577951308232087679815481410517033240")
        .expect("valid 180/pi literal");
    mul_ctx(&rad_bd, &deg_per_rad)
}

// --- Parsing helpers ---

fn parse_bd(value: &str, field: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(value.trim()).map_err(|_| format!("Invalid number for {field}: {value}"))
}

fn is_present(value: &str) -> bool {
    !value.is_empty()
}

fn validate_non_zero(value: &BigDecimal, name: &str) -> Result<(), String> {
    if value.is_zero() {
        Err(format!("{name} must not be zero"))
    } else {
        Ok(())
    }
}

fn validate_positive(value: &BigDecimal, name: &str) -> Result<(), String> {
    if value.is_zero() || value.is_negative() {
        Err(format!("{name} must be positive"))
    } else {
        Ok(())
    }
}

fn err(msg: impl AsRef<str>) -> String {
    format!("{ERROR_PREFIX}{}", msg.as_ref())
}

fn parse_csv(values: &str) -> Result<Vec<BigDecimal>, String> {
    let trimmed = values.trim();
    if trimmed.is_empty() {
        return Err("At least one value is required".to_string());
    }
    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.is_empty() {
        return Err("At least one value is required".to_string());
    }
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        out.push(parse_bd(part.trim(), "value")?);
    }
    Ok(out)
}

fn validate_filter_type(filter_type: &str) -> Result<String, String> {
    let lower = filter_type.to_ascii_lowercase();
    if lower == "lowpass" || lower == "highpass" {
        Ok(lower)
    } else {
        Err("Filter type must be 'lowpass' or 'highpass'".to_string())
    }
}

// --- Ohm's law ---

/// Ohm's Law: provide exactly two of V/I/R/P (non-empty) and compute the rest.
pub fn ohms_law(voltage: &str, current: &str, resistance: &str, power: &str) -> String {
    match compute_ohms_law(voltage, current, resistance, power) {
        Ok(json) => json,
        Err(msg) => err(msg),
    }
}

fn compute_ohms_law(
    voltage: &str,
    current: &str,
    resistance: &str,
    power: &str,
) -> Result<String, String> {
    let has_v = is_present(voltage);
    let has_i = is_present(current);
    let has_r = is_present(resistance);
    let has_p = is_present(power);
    let count = u32::from(has_v) + u32::from(has_i) + u32::from(has_r) + u32::from(has_p);
    if count != REQUIRED_KNOWNS {
        return Err(format!(
            "Exactly 2 of V, I, R, P must be provided. Got: {count}"
        ));
    }
    dispatch_ohms(has_v, has_i, has_r, voltage, current, resistance, power)
}

fn dispatch_ohms(
    has_v: bool,
    has_i: bool,
    has_r: bool,
    volt: &str,
    curr: &str,
    res: &str,
    pow: &str,
) -> Result<String, String> {
    let key = (u8::from(has_v) << 2) | (u8::from(has_i) << 1) | u8::from(has_r);
    match key {
        6 => ohms_from_vi(parse_bd(volt, "voltage")?, parse_bd(curr, "current")?),
        5 => ohms_from_vr(parse_bd(volt, "voltage")?, parse_bd(res, "resistance")?),
        4 => ohms_from_vp(parse_bd(volt, "voltage")?, parse_bd(pow, "power")?),
        3 => ohms_from_ir(parse_bd(curr, "current")?, parse_bd(res, "resistance")?),
        2 => ohms_from_ip(parse_bd(curr, "current")?, parse_bd(pow, "power")?),
        _ => ohms_from_rp(parse_bd(res, "resistance")?, parse_bd(pow, "power")?),
    }
}

fn ohms_from_vi(voltage: BigDecimal, current: BigDecimal) -> Result<String, String> {
    validate_non_zero(&current, "Current")?;
    let resistance = div_scaled(&voltage, &current);
    let power = mul_ctx(&voltage, &current);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_from_vr(voltage: BigDecimal, resistance: BigDecimal) -> Result<String, String> {
    validate_non_zero(&resistance, "Resistance")?;
    let current = div_scaled(&voltage, &resistance);
    let power = mul_ctx(&voltage, &current);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_from_vp(voltage: BigDecimal, power: BigDecimal) -> Result<String, String> {
    validate_non_zero(&voltage, "Voltage")?;
    let current = div_scaled(&power, &voltage);
    let resistance = div_scaled(&voltage, &current);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_from_ir(current: BigDecimal, resistance: BigDecimal) -> Result<String, String> {
    let voltage = mul_ctx(&current, &resistance);
    let power = mul_ctx(&voltage, &current);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_from_ip(current: BigDecimal, power: BigDecimal) -> Result<String, String> {
    validate_non_zero(&current, "Current")?;
    let voltage = div_scaled(&power, &current);
    let resistance = div_scaled(&voltage, &current);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_from_rp(resistance: BigDecimal, power: BigDecimal) -> Result<String, String> {
    let pr_product = mul_ctx(&power, &resistance);
    let voltage = af_sqrt(&pr_product);
    let pr_ratio = div_scaled(&power, &resistance);
    let current = af_sqrt(&pr_ratio);
    Ok(ohms_json(&voltage, &current, &resistance, &power))
}

fn ohms_json(
    voltage: &BigDecimal,
    current: &BigDecimal,
    resistance: &BigDecimal,
    power: &BigDecimal,
) -> String {
    format!(
        "{{\"voltage\":\"{}\",\"current\":\"{}\",\"resistance\":\"{}\",\"power\":\"{}\"}}",
        strip_plain(voltage),
        strip_plain(current),
        strip_plain(resistance),
        strip_plain(power),
    )
}

// --- Resistor / Capacitor / Inductor combination ---

/// Resistor combination: series sums, parallel reciprocal-sums.
pub fn resistor_combination(values: &str, mode: &str) -> String {
    match combine(values, mode, false) {
        Ok(val) => strip_plain(&val),
        Err(msg) => err(msg),
    }
}

/// Capacitor combination: series reciprocal-sums, parallel sums (reversed from R/L).
pub fn capacitor_combination(values: &str, mode: &str) -> String {
    match combine(values, mode, true) {
        Ok(val) => strip_plain(&val),
        Err(msg) => err(msg),
    }
}

/// Inductor combination: same as resistor (series sums, parallel reciprocal-sums).
pub fn inductor_combination(values: &str, mode: &str) -> String {
    match combine(values, mode, false) {
        Ok(val) => strip_plain(&val),
        Err(msg) => err(msg),
    }
}

fn combine(values: &str, mode: &str, reversed: bool) -> Result<BigDecimal, String> {
    let parsed = parse_csv(values)?;
    let lower = mode.to_ascii_lowercase();
    let use_sum = match (lower.as_str(), reversed) {
        (SERIES, false) | (PARALLEL, true) => true,
        (PARALLEL, false) | (SERIES, true) => false,
        _ => return Err("Mode must be 'series' or 'parallel'".to_string()),
    };
    if use_sum {
        Ok(sum_values(&parsed))
    } else {
        reciprocal_sum(&parsed)
    }
}

fn sum_values(values: &[BigDecimal]) -> BigDecimal {
    let mut total = BigDecimal::zero();
    for val in values {
        total = add_ctx(&total, val);
    }
    total
}

fn reciprocal_sum(values: &[BigDecimal]) -> Result<BigDecimal, String> {
    let one = BigDecimal::from(1);
    let mut reciprocal = BigDecimal::zero();
    for val in values {
        validate_non_zero(val, "Component value")?;
        let r = div_scaled(&one, val);
        reciprocal = add_ctx(&reciprocal, &r);
    }
    validate_non_zero(&reciprocal, "Reciprocal sum")?;
    Ok(div_scaled(&one, &reciprocal))
}

// --- Dividers ---

/// Voltage divider: Vout = Vin * R2 / (R1 + R2).
pub fn voltage_divider(vin: &str, r1: &str, r2: &str) -> String {
    let result = (|| -> Result<String, String> {
        let vin_v = parse_bd(vin, "vin")?;
        let r1_v = parse_bd(r1, "R1")?;
        let r2_v = parse_bd(r2, "R2")?;
        let sum = add_ctx(&r1_v, &r2_v);
        validate_non_zero(&sum, "R1 + R2")?;
        let vout = div_scaled(&mul_ctx(&vin_v, &r2_v), &sum);
        Ok(strip_plain(&vout))
    })();
    result.unwrap_or_else(err)
}

/// Current divider: I1 = It*R2/(R1+R2), I2 = It*R1/(R1+R2).
pub fn current_divider(total_current: &str, r1: &str, r2: &str) -> String {
    let result = (|| -> Result<String, String> {
        let it = parse_bd(total_current, "total_current")?;
        let r1_v = parse_bd(r1, "R1")?;
        let r2_v = parse_bd(r2, "R2")?;
        let sum = add_ctx(&r1_v, &r2_v);
        validate_non_zero(&sum, "R1 + R2")?;
        let i1 = div_scaled(&mul_ctx(&it, &r2_v), &sum);
        let i2 = div_scaled(&mul_ctx(&it, &r1_v), &sum);
        Ok(format!(
            "{{\"i1\":\"{}\",\"i2\":\"{}\"}}",
            strip_plain(&i1),
            strip_plain(&i2)
        ))
    })();
    result.unwrap_or_else(err)
}

// --- Time constants ---

/// RC time constant and cutoff frequency.
pub fn rc_time_constant(resistance: &str, capacitance: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r = parse_bd(resistance, "resistance")?;
        let c = parse_bd(capacitance, "capacitance")?;
        let tau = mul_ctx(&r, &c);
        validate_non_zero(&tau, "R*C")?;
        let denom = mul_ctx(&TWO_PI, &tau);
        let freq = div_scaled(&BigDecimal::from(1), &denom);
        Ok(format!(
            "{{\"tau\":\"{}\",\"cutoffFrequency\":\"{}\"}}",
            strip_plain(&tau),
            strip_plain(&freq)
        ))
    })();
    result.unwrap_or_else(err)
}

/// RL time constant and cutoff frequency.
pub fn rl_time_constant(resistance: &str, inductance: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r = parse_bd(resistance, "resistance")?;
        let l = parse_bd(inductance, "inductance")?;
        validate_non_zero(&r, "Resistance")?;
        let tau = div_scaled(&l, &r);
        let denom = mul_ctx(&TWO_PI, &l);
        let freq = div_scaled(&r, &denom);
        Ok(format!(
            "{{\"tau\":\"{}\",\"cutoffFrequency\":\"{}\"}}",
            strip_plain(&tau),
            strip_plain(&freq)
        ))
    })();
    result.unwrap_or_else(err)
}

// --- RLC / impedance ---

/// RLC resonant frequency, Q factor, bandwidth.
pub fn rlc_resonance(r: &str, l: &str, c: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r_v = parse_bd(r, "resistance")?;
        let l_v = parse_bd(l, "inductance")?;
        let c_v = parse_bd(c, "capacitance")?;
        let lc = mul_ctx(&l_v, &c_v);
        validate_positive(&lc, "L*C")?;
        let sqrt_lc = af_sqrt(&lc);
        let resonant_denom = mul_ctx(&TWO_PI, &sqrt_lc);
        let resonant_freq = div_scaled(&BigDecimal::from(1), &resonant_denom);
        validate_non_zero(&c_v, "Capacitance")?;
        let l_over_c = div_scaled(&l_v, &c_v);
        validate_positive(&l_over_c, "L/C")?;
        let z_ratio = af_sqrt(&l_over_c);
        validate_non_zero(&r_v, "Resistance")?;
        let q_factor = div_scaled(&z_ratio, &r_v);
        validate_non_zero(&q_factor, "Q factor")?;
        let bandwidth = div_scaled(&resonant_freq, &q_factor);
        Ok(format!(
            "{{\"resonantFrequency\":\"{}\",\"qFactor\":\"{}\",\"bandwidth\":\"{}\"}}",
            strip_plain(&resonant_freq),
            strip_plain(&q_factor),
            strip_plain(&bandwidth)
        ))
    })();
    result.unwrap_or_else(err)
}

/// Series RLC impedance magnitude + phase (degrees).
pub fn impedance(r: &str, l: &str, c: &str, frequency: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r_v = parse_bd(r, "resistance")?;
        let l_v = parse_bd(l, "inductance")?;
        let c_v = parse_bd(c, "capacitance")?;
        let f = parse_bd(frequency, "frequency")?;
        let omega = mul_ctx(&TWO_PI, &f);
        let x_l = mul_ctx(&omega, &l_v);
        let omega_c = mul_ctx(&omega, &c_v);
        validate_non_zero(&omega_c, "omega*C")?;
        let x_c = div_scaled(&BigDecimal::from(1), &omega_c);
        let reactance = sub_ctx(&x_l, &x_c);
        let r_squared = mul_ctx(&r_v, &r_v);
        let x_squared = mul_ctx(&reactance, &reactance);
        let sum_sq = add_ctx(&r_squared, &x_squared);
        let magnitude = af_sqrt(&sum_sq);
        let phase_deg = af_atan2_degrees(&reactance, &r_v);
        Ok(format!(
            "{{\"magnitude\":\"{}\",\"phase\":\"{}\"}}",
            strip_plain(&magnitude),
            strip_plain(&phase_deg)
        ))
    })();
    result.unwrap_or_else(err)
}

// --- Decibel ---

/// Decibel conversion: `powerToDb`, `voltageToDb`, `dbToPower`, `dbToVoltage`.
pub fn decibel_convert(value: &str, mode: &str) -> String {
    let result = (|| -> Result<String, String> {
        let val = parse_bd(value, "value")?;
        let out = compute_decibel(&val, mode)?;
        Ok(strip_plain(&out))
    })();
    result.unwrap_or_else(err)
}

fn compute_decibel(val: &BigDecimal, mode: &str) -> Result<BigDecimal, String> {
    let ten = BigDecimal::from(10);
    let twenty = BigDecimal::from(20);
    match mode {
        "powerToDb" => {
            validate_positive(val, "Power value")?;
            Ok(mul_ctx(&ten, &af_log10(val)))
        }
        "voltageToDb" => {
            validate_positive(val, "Voltage value")?;
            Ok(mul_ctx(&twenty, &af_log10(val)))
        }
        "dbToPower" => {
            let exponent = div_scaled(val, &ten);
            Ok(af_pow10(&exponent))
        }
        "dbToVoltage" => {
            let exponent = div_scaled(val, &twenty);
            Ok(af_pow10(&exponent))
        }
        _ => Err("Mode must be powerToDb, voltageToDb, dbToPower, or dbToVoltage".to_string()),
    }
}

// --- Filter cutoff ---

/// RC filter cutoff frequency. fc = 1 / (2π R C).
pub fn filter_cutoff(resistance: &str, reactive: &str, filter_type: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r = parse_bd(resistance, "resistance")?;
        let c = parse_bd(reactive, "capacitance")?;
        let rc = mul_ctx(&r, &c);
        validate_non_zero(&rc, "R*C")?;
        let denom = mul_ctx(&TWO_PI, &rc);
        let freq = div_scaled(&BigDecimal::from(1), &denom);
        let ftype = validate_filter_type(filter_type)?;
        Ok(format!(
            "{{\"cutoffFrequency\":\"{}\",\"filterType\":\"{}\"}}",
            strip_plain(&freq),
            ftype
        ))
    })();
    result.unwrap_or_else(err)
}

// --- LED / Wheatstone ---

/// LED current-limiting resistor: R = (Vs - Vf) / If.
pub fn led_resistor(vs: &str, vf: &str, i_f: &str) -> String {
    let result = (|| -> Result<String, String> {
        let vs_v = parse_bd(vs, "supply_voltage")?;
        let vf_v = parse_bd(vf, "forward_voltage")?;
        let if_v = parse_bd(i_f, "forward_current")?;
        if vs_v <= vf_v {
            return Err("Supply voltage must be greater than forward voltage".to_string());
        }
        if if_v.is_zero() || if_v.is_negative() {
            return Err("Forward current must be greater than zero".to_string());
        }
        let diff = sub_ctx(&vs_v, &vf_v);
        Ok(strip_plain(&div_scaled(&diff, &if_v)))
    })();
    result.unwrap_or_else(err)
}

/// Wheatstone bridge balance resistor: R4 = R3 * R2 / R1.
pub fn wheatstone_bridge(r1: &str, r2: &str, r3: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r1_v = parse_bd(r1, "R1")?;
        let r2_v = parse_bd(r2, "R2")?;
        let r3_v = parse_bd(r3, "R3")?;
        validate_non_zero(&r1_v, "R1")?;
        let r4 = div_scaled(&mul_ctx(&r3_v, &r2_v), &r1_v);
        Ok(strip_plain(&r4))
    })();
    result.unwrap_or_else(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohms_law_vi_produces_r_and_p() {
        let json = ohms_law("12", "2", "", "");
        assert!(json.contains("\"voltage\":\"12\""));
        assert!(json.contains("\"current\":\"2\""));
        assert!(json.contains("\"resistance\":\"6\""));
        assert!(json.contains("\"power\":\"24\""));
    }

    #[test]
    fn ohms_law_vr_produces_i_and_p() {
        let json = ohms_law("10", "", "5", "");
        assert!(json.contains("\"current\":\"2\""));
        assert!(json.contains("\"power\":\"20\""));
    }

    #[test]
    fn ohms_law_rp_uses_sqrt_path() {
        // R=4, P=100 → V=sqrt(400)=20, I=sqrt(25)=5
        let json = ohms_law("", "", "4", "100");
        assert!(json.contains("\"voltage\":\"20\""), "got: {json}");
        assert!(json.contains("\"current\":\"5\""), "got: {json}");
    }

    #[test]
    fn ohms_law_wrong_count_errors() {
        assert_eq!(
            ohms_law("1", "2", "3", ""),
            "Error: Exactly 2 of V, I, R, P must be provided. Got: 3"
        );
        assert_eq!(
            ohms_law("", "", "", ""),
            "Error: Exactly 2 of V, I, R, P must be provided. Got: 0"
        );
    }

    #[test]
    fn resistor_series_sums() {
        assert_eq!(resistor_combination("10, 20, 30", "series"), "60");
    }

    #[test]
    fn resistor_parallel_halves_equal_pair() {
        assert_eq!(resistor_combination("10, 10", "parallel"), "5");
    }

    #[test]
    fn capacitor_parallel_sums() {
        assert_eq!(
            capacitor_combination("1e-6, 2e-6, 3e-6", "parallel"),
            "0.000006"
        );
    }

    #[test]
    fn capacitor_series_reciprocal() {
        // Two 2µF caps in series → 1µF
        assert_eq!(capacitor_combination("2e-6, 2e-6", "series"), "0.000001");
    }

    #[test]
    fn inductor_series_sums() {
        assert_eq!(inductor_combination("0.001, 0.002", "series"), "0.003");
    }

    #[test]
    fn combination_bad_mode_errors() {
        assert_eq!(
            resistor_combination("10, 10", "weird"),
            "Error: Mode must be 'series' or 'parallel'"
        );
    }

    #[test]
    fn voltage_divider_half() {
        assert_eq!(voltage_divider("5", "1000", "1000"), "2.5");
    }

    #[test]
    fn current_divider_returns_json() {
        let json = current_divider("2", "1000", "1000");
        assert!(json.contains("\"i1\":\"1\""));
        assert!(json.contains("\"i2\":\"1\""));
    }

    #[test]
    fn rc_time_constant_millis() {
        // 1kΩ × 1µF = 1ms
        let json = rc_time_constant("1000", "0.000001");
        assert!(json.contains("\"tau\":\"0.001\""), "got: {json}");
        assert!(json.contains("\"cutoffFrequency\""));
    }

    #[test]
    fn rl_time_constant_basic() {
        // L=1H, R=1kΩ → τ = 0.001
        let json = rl_time_constant("1000", "1");
        assert!(json.contains("\"tau\":\"0.001\""), "got: {json}");
    }

    #[test]
    fn led_resistor_standard() {
        // 5V supply, 2V Vf, 20mA If → 150Ω
        assert_eq!(led_resistor("5", "2", "0.02"), "150");
    }

    #[test]
    fn led_resistor_bad_voltages() {
        assert_eq!(
            led_resistor("2", "5", "0.01"),
            "Error: Supply voltage must be greater than forward voltage"
        );
    }

    #[test]
    fn led_resistor_bad_current() {
        assert_eq!(
            led_resistor("5", "2", "0"),
            "Error: Forward current must be greater than zero"
        );
    }

    #[test]
    fn wheatstone_basic() {
        // R1=100, R2=200, R3=300 → R4 = 300*200/100 = 600
        assert_eq!(wheatstone_bridge("100", "200", "300"), "600");
    }

    #[test]
    fn wheatstone_zero_denominator() {
        assert_eq!(
            wheatstone_bridge("0", "10", "10"),
            "Error: R1 must not be zero"
        );
    }

    #[test]
    fn decibel_power_to_db() {
        // 10*log10(100) = 20
        assert_eq!(decibel_convert("100", "powerToDb"), "20");
    }

    #[test]
    fn decibel_voltage_to_db() {
        // 20*log10(10) = 20
        assert_eq!(decibel_convert("10", "voltageToDb"), "20");
    }

    #[test]
    fn decibel_bad_mode() {
        assert_eq!(
            decibel_convert("1", "oops"),
            "Error: Mode must be powerToDb, voltageToDb, dbToPower, or dbToVoltage"
        );
    }

    #[test]
    fn decibel_negative_power_error() {
        assert_eq!(
            decibel_convert("-1", "powerToDb"),
            "Error: Power value must be positive"
        );
    }

    #[test]
    fn filter_cutoff_json_and_type() {
        let json = filter_cutoff("1000", "0.000001", "LowPass");
        assert!(json.contains("\"cutoffFrequency\""));
        assert!(json.contains("\"filterType\":\"lowpass\""));
    }

    #[test]
    fn filter_cutoff_bad_type_errors() {
        assert_eq!(
            filter_cutoff("1000", "0.000001", "bandstop"),
            "Error: Filter type must be 'lowpass' or 'highpass'"
        );
    }

    #[test]
    fn impedance_purely_resistive_at_resonance() {
        // Pick R=10, L=1e-3, C=1e-6 — at f ≈ 5032.92 Hz, X_L=X_C → mag=R=10, phase≈0
        let json = impedance("10", "0.001", "0.000001", "5032.9216");
        assert!(json.contains("\"magnitude\""));
        assert!(json.contains("\"phase\""));
        // Magnitude near 10 (resistance only at resonance).
        let start = json.find("\"magnitude\":\"").unwrap() + "\"magnitude\":\"".len();
        let rest = &json[start..];
        let end = rest.find('"').unwrap();
        let mag: f64 = rest[..end].parse().unwrap();
        assert!((mag - 10.0).abs() < 0.05, "mag was {mag}");
    }

    #[test]
    fn rlc_resonance_produces_all_fields() {
        let json = rlc_resonance("10", "0.001", "0.000001");
        assert!(json.contains("\"resonantFrequency\""));
        assert!(json.contains("\"qFactor\""));
        assert!(json.contains("\"bandwidth\""));
    }
}
