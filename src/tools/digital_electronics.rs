//! Port of `DigitalElectronicsTool.java` — base conversion, two's complement,
//! Gray code, bitwise ops, ADC/DAC, 555-timer, and Nyquist computations.
//!
//! Uses `num_bigint::BigInt` for base conversion and two's complement to match
//! Java `BigInteger` semantics, and `bigdecimal::BigDecimal` at DECIMAL128 for
//! analog computations (555 timer, ADC/DAC, frequency↔period, Nyquist).

use std::num::NonZeroU64;
use std::str::FromStr;

use bigdecimal::{BigDecimal, RoundingMode};
use num_bigint::BigInt;
use num_traits::{Num, One, Signed, Zero};

use crate::engine::bigdecimal_ext::{
    DECIMAL128_PRECISION, DIVISION_SCALE, LN2_RECIPROCAL, strip_plain,
};

const ERROR_PREFIX: &str = "Error: ";
const TO_TWOS: &str = "toTwos";
const FROM_TWOS: &str = "fromTwos";
const TO_GRAY: &str = "toGray";
const FROM_GRAY: &str = "fromGray";
const MIN_BASE: i32 = 2;
const MAX_BASE: i32 = 36;
const MAX_BITS: i32 = 64;

fn err(msg: impl AsRef<str>) -> String {
    format!("{ERROR_PREFIX}{}", msg.as_ref())
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

fn parse_bd(value: &str, field: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(value.trim()).map_err(|_| format!("Invalid number for {field}: {value}"))
}

fn pow2(bits: u32) -> BigDecimal {
    BigDecimal::from(BigInt::one() << bits)
}

fn check_base(base: i32) -> Result<(), String> {
    if !(MIN_BASE..=MAX_BASE).contains(&base) {
        Err(format!("Base must be between {MIN_BASE} and {MAX_BASE}"))
    } else {
        Ok(())
    }
}

fn check_bit_width(bits: i32) -> Result<(), String> {
    if !(1..=MAX_BITS).contains(&bits) {
        Err(format!("Bit width must be between 1 and {MAX_BITS}"))
    } else {
        Ok(())
    }
}

fn pad_binary(binary: &str, width: usize) -> String {
    if binary.len() >= width {
        binary.to_string()
    } else {
        let mut out = String::with_capacity(width);
        for _ in 0..(width - binary.len()) {
            out.push('0');
        }
        out.push_str(binary);
        out
    }
}

/// Convert between any two bases (2..=36).
pub fn convert_base(value: &str, from_base: i32, to_base: i32) -> String {
    if let Err(msg) = check_base(from_base).and_then(|_| check_base(to_base)) {
        return err(msg);
    }
    match BigInt::from_str_radix(value.trim(), from_base as u32) {
        Ok(big) => big.to_str_radix(to_base as u32).to_uppercase(),
        Err(_) => err(format!("Invalid number '{value}' for base {from_base}")),
    }
}

/// Two's-complement encode (`toTwos`) or decode (`fromTwos`).
pub fn twos_complement(value: &str, bits: i32, direction: &str) -> String {
    if let Err(msg) = check_bit_width(bits) {
        return err(msg);
    }
    let bits_u = bits as u32;
    match direction {
        TO_TWOS => encode_to_twos(value, bits_u),
        FROM_TWOS => decode_from_twos(value, bits_u),
        _ => err(format!("Direction must be '{TO_TWOS}' or '{FROM_TWOS}'")),
    }
}

fn encode_to_twos(value: &str, bits: u32) -> String {
    let parsed = match BigInt::from_str(value.trim()) {
        Ok(v) => v,
        Err(_) => return err(format!("Invalid decimal value: {value}")),
    };
    let mask: BigInt = (BigInt::one() << bits) - BigInt::one();
    let twos = parsed & mask;
    pad_binary(&twos.to_str_radix(2), bits as usize)
}

fn decode_from_twos(value: &str, bits: u32) -> String {
    let trimmed = value.trim();
    let parsed = match BigInt::from_str_radix(trimmed, 2) {
        Ok(v) => v,
        Err(_) => return err(format!("Invalid binary value: {value}")),
    };
    let msb_set = trimmed.starts_with('1') && trimmed.len() == bits as usize;
    let result = if msb_set {
        parsed - (BigInt::one() << bits)
    } else {
        parsed
    };
    result.to_string()
}

/// Gray-code encode (`toGray`) or decode (`fromGray`).
pub fn gray_code(value: &str, direction: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(|c| c != '0' && c != '1') {
        return err(format!("Invalid binary value: {value}"));
    }
    let width = trimmed.len();
    match direction {
        TO_GRAY => encode_binary_to_gray(trimmed, width),
        FROM_GRAY => decode_gray_to_binary(trimmed, width),
        _ => err(format!("Direction must be '{TO_GRAY}' or '{FROM_GRAY}'")),
    }
}

fn encode_binary_to_gray(binary: &str, width: usize) -> String {
    let num: BigInt = BigInt::from_str_radix(binary, 2).expect("validated binary");
    let shifted: BigInt = &num >> 1_u32;
    let gray: BigInt = &num ^ &shifted;
    pad_binary(&gray.to_str_radix(2), width)
}

fn decode_gray_to_binary(binary: &str, width: usize) -> String {
    let mut num: BigInt = BigInt::from_str_radix(binary, 2).expect("validated binary");
    let mut mask: BigInt = &num >> 1_u32;
    while !mask.is_zero() {
        num = &num ^ &mask;
        mask >>= 1_u32;
    }
    pad_binary(&num.to_str_radix(2), width)
}

/// Bitwise AND/OR/XOR/NOT/SHL/SHR. Output JSON `{decimal, binary}`.
pub fn bitwise_op(a: &str, b: &str, operation: &str) -> String {
    let val_a = match BigInt::from_str(a.trim()) {
        Ok(v) => v,
        Err(_) => return err(format!("Invalid operand A: {a}")),
    };
    let op = operation.to_ascii_uppercase();
    let computed: Option<BigInt> = match op.as_str() {
        "AND" => BigInt::from_str(b.trim()).ok().map(|vb| &val_a & &vb),
        "OR" => BigInt::from_str(b.trim()).ok().map(|vb| &val_a | &vb),
        "XOR" => BigInt::from_str(b.trim()).ok().map(|vb| &val_a ^ &vb),
        "NOT" => Some(!val_a.clone()),
        "SHL" => b.trim().parse::<u32>().ok().map(|shift| &val_a << shift),
        "SHR" => b.trim().parse::<u32>().ok().map(|shift| &val_a >> shift),
        _ => return err(format!("Unknown operation: {op}")),
    };
    match computed {
        Some(value) => {
            let binary = if value.is_negative() {
                format!("-{}", (-&value).to_str_radix(2))
            } else {
                value.to_str_radix(2)
            };
            format!("{{\"decimal\":\"{}\",\"binary\":\"{}\"}}", value, binary)
        }
        None => err(format!("Invalid operand B: {b}")),
    }
}

/// ADC resolution: `lsb = Vref / 2^bits`, `stepCount = 2^bits - 1`.
pub fn adc_resolution(bits: i32, vref: &str) -> String {
    let result = (|| -> Result<String, String> {
        if !(1..=MAX_BITS).contains(&bits) {
            return Err(format!("Bit width must be between 1 and {MAX_BITS}"));
        }
        let vref_v = parse_bd(vref, "vref")?;
        let levels = pow2(bits as u32);
        let lsb = div_scaled(&vref_v, &levels);
        let step_count = sub_ctx(&levels, &BigDecimal::from(1));
        Ok(format!(
            "{{\"lsb\":\"{}\",\"stepCount\":\"{}\",\"bits\":{}}}",
            strip_plain(&lsb),
            strip_plain(&step_count),
            bits
        ))
    })();
    result.unwrap_or_else(err)
}

/// DAC output voltage: Vout = Vref * code / 2^bits.
pub fn dac_output(bits: i32, vref: &str, code: i64) -> String {
    let result = (|| -> Result<String, String> {
        if !(1..=MAX_BITS).contains(&bits) {
            return Err(format!("Bit width must be between 1 and {MAX_BITS}"));
        }
        // Use i128 to compute max_code for bits up to 63 safely; bits==64 would overflow i64.
        let max_code: i128 = if bits == 64 {
            i128::from(i64::MAX)
        } else {
            (1_i128 << bits) - 1
        };
        if i128::from(code) < 0 || i128::from(code) > max_code {
            return Err(format!("Code must be between 0 and {max_code}"));
        }
        let vref_v = parse_bd(vref, "vref")?;
        let levels = pow2(bits as u32);
        let vout = div_scaled(&mul_ctx(&vref_v, &BigDecimal::from(code)), &levels);
        Ok(strip_plain(&vout))
    })();
    result.unwrap_or_else(err)
}

/// 555-timer astable output: frequency, dutyCycle (%), period.
pub fn timer_555_astable(r1: &str, r2: &str, c: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r1_v = parse_bd(r1, "R1")?;
        let r2_v = parse_bd(r2, "R2")?;
        let c_v = parse_bd(c, "C")?;
        let two = BigDecimal::from(2);
        let r1_plus_2r2 = add_ctx(&r1_v, &mul_ctx(&r2_v, &two));
        let denominator = mul_ctx(&r1_plus_2r2, &c_v);
        if denominator.is_zero() {
            return Err("(R1 + 2R2)*C must not be zero".to_string());
        }
        let freq = div_scaled(&LN2_RECIPROCAL, &denominator);
        if freq.is_zero() {
            return Err("Frequency must not be zero".to_string());
        }
        let period = div_scaled(&BigDecimal::from(1), &freq);
        let r1_plus_r2 = add_ctx(&r1_v, &r2_v);
        let duty = mul_ctx(
            &div_scaled(&r1_plus_r2, &r1_plus_2r2),
            &BigDecimal::from(100),
        );
        Ok(format!(
            "{{\"frequency\":\"{}\",\"dutyCycle\":\"{}\",\"period\":\"{}\"}}",
            strip_plain(&freq),
            strip_plain(&duty),
            strip_plain(&period)
        ))
    })();
    result.unwrap_or_else(err)
}

/// 555-timer monostable pulse width: 1.1 * R * C.
pub fn timer_555_monostable(r: &str, c: &str) -> String {
    let result = (|| -> Result<String, String> {
        let r_v = parse_bd(r, "R")?;
        let c_v = parse_bd(c, "C")?;
        let constant: BigDecimal = "1.1".parse().expect("valid constant");
        let pulse = mul_ctx(&mul_ctx(&constant, &r_v), &c_v);
        Ok(format!("{{\"pulseWidth\":\"{}\"}}", strip_plain(&pulse)))
    })();
    result.unwrap_or_else(err)
}

/// Convert between frequency and period (reciprocal).
pub fn frequency_period(value: &str, mode: &str) -> String {
    let result = (|| -> Result<String, String> {
        match mode {
            "freqToPeriod" | "periodToFreq" => {}
            _ => return Err("Mode must be 'freqToPeriod' or 'periodToFreq'".to_string()),
        }
        let val = parse_bd(value, "value")?;
        if val.is_zero() || val.is_negative() {
            return Err("Value must be positive".to_string());
        }
        Ok(strip_plain(&div_scaled(&BigDecimal::from(1), &val)))
    })();
    result.unwrap_or_else(err)
}

/// Nyquist minimum sampling rate: 2 × bandwidth.
pub fn nyquist_rate(bandwidth_hz: &str) -> String {
    let result = (|| -> Result<String, String> {
        let bw = parse_bd(bandwidth_hz, "bandwidth")?;
        let min_rate = mul_ctx(&bw, &BigDecimal::from(2));
        Ok(format!(
            "{{\"minSampleRate\":\"{}\",\"bandwidth\":\"{}\"}}",
            strip_plain(&min_rate),
            strip_plain(&bw)
        ))
    })();
    result.unwrap_or_else(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_base_decimal_to_hex() {
        assert_eq!(convert_base("255", 10, 16), "FF");
    }

    #[test]
    fn convert_base_binary_to_decimal() {
        assert_eq!(convert_base("1010", 2, 10), "10");
    }

    #[test]
    fn convert_base_hex_to_binary() {
        assert_eq!(convert_base("FF", 16, 2), "11111111");
    }

    #[test]
    fn convert_base_bad_base_errors() {
        assert_eq!(
            convert_base("10", 1, 10),
            "Error: Base must be between 2 and 36"
        );
        assert_eq!(
            convert_base("10", 10, 37),
            "Error: Base must be between 2 and 36"
        );
    }

    #[test]
    fn convert_base_invalid_digit_errors() {
        let out = convert_base("XYZ", 10, 16);
        assert!(out.starts_with("Error:"), "got: {out}");
    }

    #[test]
    fn twos_complement_negative_eight_bit() {
        assert_eq!(twos_complement("-5", 8, "toTwos"), "11111011");
    }

    #[test]
    fn twos_complement_positive_eight_bit() {
        assert_eq!(twos_complement("5", 8, "toTwos"), "00000101");
    }

    #[test]
    fn twos_complement_roundtrip_negative() {
        assert_eq!(twos_complement("11111011", 8, "fromTwos"), "-5");
    }

    #[test]
    fn twos_complement_roundtrip_positive() {
        assert_eq!(twos_complement("00000101", 8, "fromTwos"), "5");
    }

    #[test]
    fn twos_complement_bad_bits_errors() {
        assert_eq!(
            twos_complement("5", 0, "toTwos"),
            "Error: Bit width must be between 1 and 64"
        );
        assert_eq!(
            twos_complement("5", 65, "toTwos"),
            "Error: Bit width must be between 1 and 64"
        );
    }

    #[test]
    fn twos_complement_bad_direction_errors() {
        assert_eq!(
            twos_complement("5", 8, "toward"),
            "Error: Direction must be 'toTwos' or 'fromTwos'"
        );
    }

    #[test]
    fn gray_code_to_gray() {
        assert_eq!(gray_code("1010", "toGray"), "1111");
    }

    #[test]
    fn gray_code_from_gray_roundtrip() {
        assert_eq!(gray_code("1111", "fromGray"), "1010");
    }

    #[test]
    fn gray_code_bad_direction_errors() {
        assert_eq!(
            gray_code("1010", "flip"),
            "Error: Direction must be 'toGray' or 'fromGray'"
        );
    }

    #[test]
    fn gray_code_invalid_binary_errors() {
        assert!(gray_code("102", "toGray").starts_with("Error:"));
    }

    #[test]
    fn bitwise_and_ones() {
        let json = bitwise_op("12", "10", "AND");
        assert!(json.contains("\"decimal\":\"8\""));
        assert!(json.contains("\"binary\":\"1000\""));
    }

    #[test]
    fn bitwise_or_simple() {
        let json = bitwise_op("12", "10", "OR");
        assert!(json.contains("\"decimal\":\"14\""));
    }

    #[test]
    fn bitwise_xor_simple() {
        let json = bitwise_op("12", "10", "XOR");
        assert!(json.contains("\"decimal\":\"6\""));
    }

    #[test]
    fn bitwise_shl() {
        let json = bitwise_op("1", "4", "SHL");
        assert!(json.contains("\"decimal\":\"16\""));
        assert!(json.contains("\"binary\":\"10000\""));
    }

    #[test]
    fn bitwise_shr() {
        let json = bitwise_op("16", "4", "SHR");
        assert!(json.contains("\"decimal\":\"1\""));
    }

    #[test]
    fn bitwise_unknown_op_errors() {
        assert_eq!(
            bitwise_op("1", "1", "NAND"),
            "Error: Unknown operation: NAND"
        );
    }

    #[test]
    fn adc_resolution_ten_bit() {
        let json = adc_resolution(10, "5");
        // lsb = 5/1024 ≈ 0.00488281...
        assert!(json.contains("\"bits\":10"));
        assert!(json.contains("\"stepCount\":\"1023\""));
        let start = json.find("\"lsb\":\"").unwrap() + "\"lsb\":\"".len();
        let rest = &json[start..];
        let end = rest.find('"').unwrap();
        let lsb: f64 = rest[..end].parse().unwrap();
        assert!((lsb - 5.0 / 1024.0).abs() < 1e-10, "lsb = {lsb}");
    }

    #[test]
    fn adc_resolution_bad_bits_errors() {
        assert_eq!(
            adc_resolution(0, "5"),
            "Error: Bit width must be between 1 and 64"
        );
    }

    #[test]
    fn dac_output_midpoint() {
        // bits=10, vref=5, code=512 → 5*512/1024 = 2.5
        assert_eq!(dac_output(10, "5", 512), "2.5");
    }

    #[test]
    fn dac_output_out_of_range_errors() {
        assert_eq!(
            dac_output(8, "5", 1000),
            "Error: Code must be between 0 and 255"
        );
        assert_eq!(
            dac_output(8, "5", -1),
            "Error: Code must be between 0 and 255"
        );
    }

    #[test]
    fn timer_555_astable_sensible_duty() {
        let json = timer_555_astable("1000", "1000", "0.000001");
        // f = 1.4427/(3000*1e-6) = ~480.9 Hz, duty = 2000/3000 * 100 ≈ 66.67
        assert!(json.contains("\"frequency\""));
        assert!(json.contains("\"dutyCycle\""));
        let start = json.find("\"dutyCycle\":\"").unwrap() + "\"dutyCycle\":\"".len();
        let rest = &json[start..];
        let end = rest.find('"').unwrap();
        let duty: f64 = rest[..end].parse().unwrap();
        assert!((duty - 66.6666).abs() < 0.01, "duty was {duty}");
    }

    #[test]
    fn timer_555_monostable_formula() {
        // 1.1 * 1000 * 1e-6 = 0.0011
        let json = timer_555_monostable("1000", "0.000001");
        assert!(json.contains("\"pulseWidth\":\"0.0011\""), "got: {json}");
    }

    #[test]
    fn frequency_period_freq_to_period() {
        assert_eq!(frequency_period("1000", "freqToPeriod"), "0.001");
    }

    #[test]
    fn frequency_period_period_to_freq() {
        assert_eq!(frequency_period("0.001", "periodToFreq"), "1000");
    }

    #[test]
    fn frequency_period_bad_mode_errors() {
        assert_eq!(
            frequency_period("10", "flip"),
            "Error: Mode must be 'freqToPeriod' or 'periodToFreq'"
        );
    }

    #[test]
    fn frequency_period_non_positive_errors() {
        assert_eq!(
            frequency_period("0", "freqToPeriod"),
            "Error: Value must be positive"
        );
        assert_eq!(
            frequency_period("-5", "periodToFreq"),
            "Error: Value must be positive"
        );
    }

    #[test]
    fn nyquist_rate_audio() {
        let json = nyquist_rate("20000");
        assert!(json.contains("\"minSampleRate\":\"40000\""), "got: {json}");
        assert!(json.contains("\"bandwidth\":\"20000\""));
    }
}
