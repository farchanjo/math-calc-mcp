//! Classical physics formulas — kinematics, gravity, optics, thermodynamics.
//!
//! All inputs are decimal strings (SI base units unless noted). Constants use
//! the SI 2019 exact values: `G = 6.67430e-11`, `c = 299_792_458`,
//! `h = 6.62607015e-34`, `R = 8.314_462_618`, `σ = 5.670_374_419e-8`.
//! Outputs are formatted via Rust's debug `f64` representation.

use crate::mcp::message::{ErrorCode, Response, error_with_detail};
use crate::tools::numeric::guard_finite;

const TOOL_KINEMATICS: &str = "KINEMATICS";
const TOOL_PROJECTILE_MOTION: &str = "PROJECTILE_MOTION";
const TOOL_NEWTONS_FORCE: &str = "NEWTONS_FORCE";
const TOOL_GRAVITATIONAL_FORCE: &str = "GRAVITATIONAL_FORCE";
const TOOL_DOPPLER_EFFECT: &str = "DOPPLER_EFFECT";
const TOOL_WAVE_LENGTH: &str = "WAVE_LENGTH";
const TOOL_PLANCK_ENERGY: &str = "PLANCK_ENERGY";
const TOOL_IDEAL_GAS_LAW: &str = "IDEAL_GAS_LAW";
const TOOL_HEAT_TRANSFER: &str = "HEAT_TRANSFER";
const TOOL_STEFAN_BOLTZMANN: &str = "STEFAN_BOLTZMANN";
const TOOL_ESCAPE_VELOCITY: &str = "ESCAPE_VELOCITY";
const TOOL_ORBITAL_VELOCITY: &str = "ORBITAL_VELOCITY";

// SI 2019 exact constants (CODATA recommended values).
const G: f64 = 6.674_30e-11;
const H: f64 = 6.626_070_15e-34;
const R_GAS: f64 = 8.314_462_618;
const SIGMA: f64 = 5.670_374_419e-8;

fn parse(tool: &str, label: &str, input: &str) -> Result<f64, String> {
    input.trim().parse::<f64>().map_err(|_| {
        error_with_detail(
            tool,
            ErrorCode::ParseError,
            "value is not a valid number",
            &format!("{label}={input}"),
        )
    })
}

fn fmt(value: f64) -> String {
    format!("{value:?}")
}

/// Wrap a scalar result with an overflow guard before shipping it.
/// Tools that just emit a single `RESULT` field go through here so
/// `RESULT: inf` can never escape the envelope.
fn ok_result(tool: &str, value: f64) -> String {
    match guard_finite(tool, "result", value) {
        Ok(v) => Response::ok(tool).result(fmt(v)).build(),
        Err(e) => e,
    }
}

/// 1D constant-acceleration kinematics: from initial velocity, acceleration,
/// and time, return final velocity and displacement.
#[must_use]
pub fn kinematics(initial_velocity: &str, acceleration: &str, time: &str) -> String {
    let v0 = match parse(TOOL_KINEMATICS, "initialVelocity", initial_velocity) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let a = match parse(TOOL_KINEMATICS, "acceleration", acceleration) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let t = match parse(TOOL_KINEMATICS, "time", time) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if t < 0.0 {
        return error_with_detail(
            TOOL_KINEMATICS,
            ErrorCode::DomainError,
            "time must be non-negative",
            &format!("time={t}"),
        );
    }
    // Compute in BigDecimal so textbook inputs like `v0=10, a=-9.81, t=1`
    // return the exact `0.19` a student would write — the f64 path prints
    // `0.1899999999999995` because fma rounds `10 + -9.81` one ULP low.
    // BigDecimal preserves the input precision end-to-end; f64 is kept as
    // a fallback (the guard above already rejected inf/NaN).
    let (v_final_display, disp_display) =
        kinematics_bigdecimal(initial_velocity, acceleration, time)
            .unwrap_or_else(|| (fmt(a.mul_add(t, v0)), fmt(v0.mul_add(t, 0.5 * a * t * t))));
    if let Err(e) = guard_finite(TOOL_KINEMATICS, "finalVelocity", a.mul_add(t, v0)) {
        return e;
    }
    if let Err(e) = guard_finite(
        TOOL_KINEMATICS,
        "displacement",
        v0.mul_add(t, 0.5 * a * t * t),
    ) {
        return e;
    }
    Response::ok(TOOL_KINEMATICS)
        .field("FINAL_VELOCITY", v_final_display)
        .field("DISPLACEMENT", disp_display)
        .build()
}

/// `BigDecimal` re-computation of `(v0 + a·t, v0·t + ½·a·t²)` so the
/// displayed output matches the caller's input precision instead of
/// printing f64 fma drift (`0.1899999999999995` in place of `0.19`).
/// Returns `None` only when any input fails to parse as a `BigDecimal` — in
/// which case the caller falls back to the f64 path.
fn kinematics_bigdecimal(
    initial_velocity: &str,
    acceleration: &str,
    time: &str,
) -> Option<(String, String)> {
    use bigdecimal::{BigDecimal, RoundingMode};
    use std::str::FromStr;
    let v0 = BigDecimal::from_str(initial_velocity.trim()).ok()?;
    let a = BigDecimal::from_str(acceleration.trim()).ok()?;
    let t = BigDecimal::from_str(time.trim()).ok()?;
    let half = BigDecimal::from_str("0.5").expect("literal parses");
    let t2 = &t * &t;
    let v_final = &v0 + &a * &t;
    let displacement = &v0 * &t + &half * &a * &t2;
    // Trim trailing zeros so `0.1 + (-0.1)` returns `0` instead of `0.0`.
    // 20-digit scale matches the `divide` tool's precision budget.
    let shape = |value: &BigDecimal| -> String {
        use num_traits::Zero;
        let rounded = value.with_scale_round(20, RoundingMode::HalfUp);
        if rounded.is_zero() {
            return "0.0".to_string();
        }
        let text = rounded.normalized().to_plain_string();
        // Mimic the f64 debug formatter's `.0` tail for whole numbers.
        if !text.contains('.') {
            return format!("{text}.0");
        }
        text
    };
    Some((shape(&v_final), shape(&displacement)))
}

/// Projectile motion (no air resistance). Inputs: launch speed (m/s),
/// angle (degrees), gravity (default 9.81). Returns range, peak height,
/// and time of flight.
#[must_use]
pub fn projectile_motion(speed: &str, angle_degrees: &str, gravity: &str) -> String {
    let v = match parse(TOOL_PROJECTILE_MOTION, "speed", speed) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let theta = match parse(TOOL_PROJECTILE_MOTION, "angleDegrees", angle_degrees) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let g = match parse(TOOL_PROJECTILE_MOTION, "gravity", gravity) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if v < 0.0 || g <= 0.0 {
        return error_with_detail(
            TOOL_PROJECTILE_MOTION,
            ErrorCode::DomainError,
            "speed must be non-negative and gravity positive",
            &format!("speed={v}, gravity={g}"),
        );
    }
    if !(0.0..=180.0).contains(&theta) {
        // Angles in [0°, 180°] span every physically sensible launch above
        // the horizon (0 = horizontal forward, 90 = straight up, 180 =
        // horizontal backward). Negative or > 180° angles imply a downward
        // launch, which yields a negative time-of-flight — reject instead.
        return error_with_detail(
            TOOL_PROJECTILE_MOTION,
            ErrorCode::DomainError,
            "angle must be in [0, 180] degrees",
            &format!("angleDegrees={theta}"),
        );
    }
    let rad = theta.to_radians();
    let vx = v * rad.cos();
    let vy = v * rad.sin();
    let t_flight = 2.0 * vy / g;
    let range_raw = vx * t_flight;
    // sin(180°) is 1.22e-16 in f64, so the nominal range at θ=90° leaks FP
    // noise like 1.25e-15 instead of the physically exact 0. Clamp anything
    // below a quarter-ulp of the inputs to 0 so "vertical launch" reports 0.
    let range_floor = (v * v / g) * f64::EPSILON * 8.0;
    let range = if range_raw.abs() <= range_floor {
        0.0
    } else {
        range_raw
    };
    let peak = vy * vy / (2.0 * g);
    for (label, val) in [
        ("range", range),
        ("peakHeight", peak),
        ("timeOfFlight", t_flight),
    ] {
        if let Err(e) = guard_finite(TOOL_PROJECTILE_MOTION, label, val) {
            return e;
        }
    }
    Response::ok(TOOL_PROJECTILE_MOTION)
        .field("RANGE", fmt(range))
        .field("PEAK_HEIGHT", fmt(peak))
        .field("TIME_OF_FLIGHT", fmt(t_flight))
        .build()
}

#[must_use]
pub fn newtons_force(mass: &str, acceleration: &str) -> String {
    let m = match parse(TOOL_NEWTONS_FORCE, "mass", mass) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let a = match parse(TOOL_NEWTONS_FORCE, "acceleration", acceleration) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m < 0.0 {
        return error_with_detail(
            TOOL_NEWTONS_FORCE,
            ErrorCode::DomainError,
            "mass must be non-negative",
            &format!("mass={m}"),
        );
    }
    ok_result(TOOL_NEWTONS_FORCE, m * a)
}

/// Newton's law of universal gravitation: `F = G m1 m2 / r²`.
#[must_use]
pub fn gravitational_force(m1: &str, m2: &str, distance: &str) -> String {
    let m1v = match parse(TOOL_GRAVITATIONAL_FORCE, "m1", m1) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let m2v = match parse(TOOL_GRAVITATIONAL_FORCE, "m2", m2) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let r = match parse(TOOL_GRAVITATIONAL_FORCE, "distance", distance) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m1v < 0.0 || m2v < 0.0 {
        return error_with_detail(
            TOOL_GRAVITATIONAL_FORCE,
            ErrorCode::DomainError,
            "masses must be non-negative",
            &format!("m1={m1v}, m2={m2v}"),
        );
    }
    if r <= 0.0 {
        return error_with_detail(
            TOOL_GRAVITATIONAL_FORCE,
            ErrorCode::DomainError,
            "distance must be positive",
            &format!("distance={r}"),
        );
    }
    ok_result(TOOL_GRAVITATIONAL_FORCE, G * m1v * m2v / (r * r))
}

/// Classical (non-relativistic) Doppler shift for sound.
///
/// `mode` is one of `source` (source moving), `observer` (observer moving), or
/// `both`. Speeds in m/s; positive values mean the moving party approaches.
#[must_use]
pub fn doppler_effect(
    source_freq: &str,
    sound_speed: &str,
    source_velocity: &str,
    observer_velocity: &str,
) -> String {
    let f0 = match parse(TOOL_DOPPLER_EFFECT, "sourceFreq", source_freq) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let c_sound = match parse(TOOL_DOPPLER_EFFECT, "soundSpeed", sound_speed) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let vs = match parse(TOOL_DOPPLER_EFFECT, "sourceVelocity", source_velocity) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let vo = match parse(TOOL_DOPPLER_EFFECT, "observerVelocity", observer_velocity) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if c_sound <= 0.0 {
        return error_with_detail(
            TOOL_DOPPLER_EFFECT,
            ErrorCode::DomainError,
            "soundSpeed must be positive",
            &format!("soundSpeed={c_sound}"),
        );
    }
    let denom = c_sound - vs;
    // Bitwise zero check avoids clippy's `float_cmp` lint — `denom` is
    // exactly `0.0` iff the IEEE-754 subtraction produced a true zero,
    // which is the only case that makes the formula singular. Comparing
    // the raw bit pattern masks `0.0` vs `-0.0` and sidesteps the
    // "fuzzy" float comparison.
    if denom.to_bits() & !(1u64 << 63) == 0 {
        return error_with_detail(
            TOOL_DOPPLER_EFFECT,
            ErrorCode::DomainError,
            "source moves at sound speed — formula undefined",
            &format!("sourceVelocity={vs}, soundSpeed={c_sound}"),
        );
    }
    // Classical Doppler is only defined for sub-sonic source speeds. With
    // `|vs| > c_sound` the denominator flips sign, producing a negative
    // "apparent frequency" — a mathematical artefact, not a physical
    // observable (what really happens is a shock wave / sonic boom, which
    // the classical formula cannot describe). Surface it as a domain
    // error instead of returning `f ≈ -230 Hz` silently.
    if vs.abs() > c_sound {
        return error_with_detail(
            TOOL_DOPPLER_EFFECT,
            ErrorCode::DomainError,
            "classical Doppler is undefined for supersonic source — use a shock-wave model instead",
            &format!("sourceVelocity={vs}, soundSpeed={c_sound}"),
        );
    }
    let f = f0 * (c_sound + vo) / denom;
    ok_result(TOOL_DOPPLER_EFFECT, f)
}

/// `λ = c / f` (or any wave: λ = v/f). Returns wavelength in meters when c is
/// the speed of light; otherwise in the same length unit as the speed.
#[must_use]
pub fn wave_length(frequency: &str, wave_speed: &str) -> String {
    let f = match parse(TOOL_WAVE_LENGTH, "frequency", frequency) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let v = match parse(TOOL_WAVE_LENGTH, "waveSpeed", wave_speed) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if f <= 0.0 {
        return error_with_detail(
            TOOL_WAVE_LENGTH,
            ErrorCode::DomainError,
            "frequency must be positive",
            &format!("frequency={f}"),
        );
    }
    if v <= 0.0 {
        return error_with_detail(
            TOOL_WAVE_LENGTH,
            ErrorCode::DomainError,
            "waveSpeed must be positive",
            &format!("waveSpeed={v}"),
        );
    }
    ok_result(TOOL_WAVE_LENGTH, v / f)
}

/// Photon energy `E = hf` in joules.
#[must_use]
pub fn planck_energy(frequency: &str) -> String {
    let f = match parse(TOOL_PLANCK_ENERGY, "frequency", frequency) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if f < 0.0 {
        return error_with_detail(
            TOOL_PLANCK_ENERGY,
            ErrorCode::DomainError,
            "frequency must be non-negative",
            &format!("frequency={f}"),
        );
    }
    ok_result(TOOL_PLANCK_ENERGY, H * f)
}

fn non_zero_or_err(label: &str, value: f64, solving: &str) -> Result<f64, String> {
    if value == 0.0 {
        Err(error_with_detail(
            TOOL_IDEAL_GAS_LAW,
            ErrorCode::DivisionByZero,
            &format!("{label} must be non-zero when solving for {solving}"),
            &format!("{label}=0"),
        ))
    } else {
        Ok(value)
    }
}

fn positive_or_err(label: &str, value: f64) -> Result<f64, String> {
    if value <= 0.0 {
        Err(error_with_detail(
            TOOL_IDEAL_GAS_LAW,
            ErrorCode::DomainError,
            &format!("{label} must be positive"),
            &format!("{label}={value}"),
        ))
    } else {
        Ok(value)
    }
}

fn non_negative_or_err(label: &str, value: f64) -> Result<f64, String> {
    if value < 0.0 {
        Err(error_with_detail(
            TOOL_IDEAL_GAS_LAW,
            ErrorCode::DomainError,
            &format!("{label} must be non-negative"),
            &format!("{label}={value}"),
        ))
    } else {
        Ok(value)
    }
}

fn solved_response(solving: &str, value: f64) -> String {
    if let Err(e) = guard_finite(TOOL_IDEAL_GAS_LAW, solving, value) {
        return e;
    }
    Response::ok(TOOL_IDEAL_GAS_LAW)
        .field("SOLVED_FOR", solving.to_string())
        .field("VALUE", fmt(value))
        .build()
}

/// Ideal gas law `PV = nRT`. `solveFor` is one of `P`, `V`, `n`, `T`.
/// Provide the three known quantities; the unknown is computed.
#[must_use]
pub fn ideal_gas_law(
    pressure: &str,
    volume: &str,
    moles: &str,
    temperature: &str,
    solve_for: &str,
) -> String {
    let key = solve_for.trim().to_uppercase();
    match key.as_str() {
        "P" => match (
            parse(TOOL_IDEAL_GAS_LAW, "volume", volume)
                .and_then(|v| non_zero_or_err("volume", v, "P"))
                .and_then(|v| positive_or_err("volume", v)),
            parse(TOOL_IDEAL_GAS_LAW, "moles", moles).and_then(|v| non_negative_or_err("moles", v)),
            parse(TOOL_IDEAL_GAS_LAW, "temperature", temperature)
                .and_then(|v| positive_or_err("temperature", v)),
        ) {
            (Ok(v), Ok(n), Ok(t)) => solved_response("P", n * R_GAS * t / v),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
        },
        "V" => match (
            parse(TOOL_IDEAL_GAS_LAW, "pressure", pressure)
                .and_then(|v| non_zero_or_err("pressure", v, "V"))
                .and_then(|v| positive_or_err("pressure", v)),
            parse(TOOL_IDEAL_GAS_LAW, "moles", moles).and_then(|v| non_negative_or_err("moles", v)),
            parse(TOOL_IDEAL_GAS_LAW, "temperature", temperature)
                .and_then(|v| positive_or_err("temperature", v)),
        ) {
            (Ok(p), Ok(n), Ok(t)) => solved_response("V", n * R_GAS * t / p),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
        },
        "N" | "MOLES" => match (
            parse(TOOL_IDEAL_GAS_LAW, "pressure", pressure)
                .and_then(|v| positive_or_err("pressure", v)),
            parse(TOOL_IDEAL_GAS_LAW, "volume", volume).and_then(|v| positive_or_err("volume", v)),
            parse(TOOL_IDEAL_GAS_LAW, "temperature", temperature)
                .and_then(|v| non_zero_or_err("temperature", v, "n"))
                .and_then(|v| positive_or_err("temperature", v)),
        ) {
            (Ok(p), Ok(v), Ok(t)) => solved_response("n", p * v / (R_GAS * t)),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
        },
        "T" => match (
            parse(TOOL_IDEAL_GAS_LAW, "pressure", pressure)
                .and_then(|v| positive_or_err("pressure", v)),
            parse(TOOL_IDEAL_GAS_LAW, "volume", volume).and_then(|v| positive_or_err("volume", v)),
            parse(TOOL_IDEAL_GAS_LAW, "moles", moles)
                .and_then(|v| non_zero_or_err("moles", v, "T"))
                .and_then(|v| positive_or_err("moles", v)),
        ) {
            (Ok(p), Ok(v), Ok(n)) => solved_response("T", p * v / (n * R_GAS)),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => e,
        },
        other => error_with_detail(
            TOOL_IDEAL_GAS_LAW,
            ErrorCode::InvalidInput,
            "solveFor must be one of P, V, n, T",
            &format!("solveFor={other}"),
        ),
    }
}

/// Conduction heat-transfer rate via Fourier's law:
/// `Q = k * A * ΔT / thickness`.
#[must_use]
pub fn heat_transfer(
    thermal_conductivity: &str,
    area: &str,
    delta_temp: &str,
    thickness: &str,
) -> String {
    let k = match parse(
        TOOL_HEAT_TRANSFER,
        "thermalConductivity",
        thermal_conductivity,
    ) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let a = match parse(TOOL_HEAT_TRANSFER, "area", area) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let dt = match parse(TOOL_HEAT_TRANSFER, "deltaTemp", delta_temp) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let l = match parse(TOOL_HEAT_TRANSFER, "thickness", thickness) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if k < 0.0 {
        return error_with_detail(
            TOOL_HEAT_TRANSFER,
            ErrorCode::DomainError,
            "thermalConductivity must be non-negative",
            &format!("thermalConductivity={k}"),
        );
    }
    if a < 0.0 {
        return error_with_detail(
            TOOL_HEAT_TRANSFER,
            ErrorCode::DomainError,
            "area must be non-negative",
            &format!("area={a}"),
        );
    }
    if l <= 0.0 {
        return error_with_detail(
            TOOL_HEAT_TRANSFER,
            ErrorCode::DomainError,
            "thickness must be positive",
            &format!("thickness={l}"),
        );
    }
    ok_result(TOOL_HEAT_TRANSFER, k * a * dt / l)
}

/// Stefan-Boltzmann law: `P = σ * ε * A * T⁴`. Returns radiated power (W).
#[must_use]
pub fn stefan_boltzmann(emissivity: &str, area: &str, temperature_k: &str) -> String {
    let eps = match parse(TOOL_STEFAN_BOLTZMANN, "emissivity", emissivity) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let a = match parse(TOOL_STEFAN_BOLTZMANN, "area", area) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let t = match parse(TOOL_STEFAN_BOLTZMANN, "temperatureK", temperature_k) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !(0.0..=1.0).contains(&eps) {
        return error_with_detail(
            TOOL_STEFAN_BOLTZMANN,
            ErrorCode::OutOfRange,
            "emissivity must be in [0, 1]",
            &format!("emissivity={eps}"),
        );
    }
    // Area must be non-negative — sister tool `heat_transfer` already
    // enforces this, and without the check `P = σεAT⁴` silently returns
    // a negative "radiated power" for a negative surface, which is
    // physically meaningless.
    if a < 0.0 {
        return error_with_detail(
            TOOL_STEFAN_BOLTZMANN,
            ErrorCode::DomainError,
            "area must be non-negative",
            &format!("area={a}"),
        );
    }
    if t < 0.0 {
        return error_with_detail(
            TOOL_STEFAN_BOLTZMANN,
            ErrorCode::DomainError,
            "absolute temperature must be non-negative",
            &format!("temperatureK={t}"),
        );
    }
    ok_result(TOOL_STEFAN_BOLTZMANN, SIGMA * eps * a * t.powi(4))
}

/// Escape velocity from a body of mass M at radius r: `v = √(2GM/r)`.
#[must_use]
pub fn escape_velocity(mass: &str, radius: &str) -> String {
    let m = match parse(TOOL_ESCAPE_VELOCITY, "mass", mass) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let r = match parse(TOOL_ESCAPE_VELOCITY, "radius", radius) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if r <= 0.0 || m < 0.0 {
        return error_with_detail(
            TOOL_ESCAPE_VELOCITY,
            ErrorCode::DomainError,
            "mass must be non-negative and radius positive",
            &format!("mass={m}, radius={r}"),
        );
    }
    ok_result(TOOL_ESCAPE_VELOCITY, (2.0 * G * m / r).sqrt())
}

/// Circular orbital velocity at radius r: `v = √(GM/r)`.
#[must_use]
pub fn orbital_velocity(mass: &str, radius: &str) -> String {
    let m = match parse(TOOL_ORBITAL_VELOCITY, "mass", mass) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let r = match parse(TOOL_ORBITAL_VELOCITY, "radius", radius) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if r <= 0.0 || m < 0.0 {
        return error_with_detail(
            TOOL_ORBITAL_VELOCITY,
            ErrorCode::DomainError,
            "mass must be non-negative and radius positive",
            &format!("mass={m}, radius={r}"),
        );
    }
    ok_result(TOOL_ORBITAL_VELOCITY, (G * m / r).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_field(out: &str, key: &str, expected: f64, tol: f64) {
        let primary = format!(" | {key}: ");
        let header = format!(": OK | {key}: ");
        let part = out
            .split(&primary)
            .nth(1)
            .or_else(|| out.split(&header).nth(1))
            .unwrap_or_else(|| panic!("field {key} not found in `{out}`"));
        let value_str: String = part
            .chars()
            .take_while(|c| *c != ' ' && *c != '\n')
            .collect();
        let v: f64 = value_str.parse().expect("parse");
        assert!(
            (v - expected).abs() < tol,
            "{key}: expected ~{expected} (tol {tol}), got {v}"
        );
    }

    #[test]
    fn kinematics_simple() {
        // v0=0, a=10, t=2 → vf=20, d=20
        let out = kinematics("0", "10", "2");
        approx_field(&out, "FINAL_VELOCITY", 20.0, 1e-9);
        approx_field(&out, "DISPLACEMENT", 20.0, 1e-9);
    }

    #[test]
    fn projectile_45deg_optimal_range() {
        // v=10, θ=45°, g=9.81 → range = v²/g = 100/9.81 ≈ 10.19
        let out = projectile_motion("10", "45", "9.81");
        approx_field(&out, "RANGE", 100.0 / 9.81, 1e-3);
    }

    #[test]
    fn projectile_negative_speed_errors() {
        let out = projectile_motion("-1", "30", "9.81");
        assert!(out.starts_with("PROJECTILE_MOTION: ERROR"));
    }

    #[test]
    fn newtons_force_basic() {
        // F = ma = 5*2 = 10
        let out = newtons_force("5", "2");
        assert!(out.contains("RESULT: 10.0"), "got {out}");
    }

    #[test]
    fn gravitational_force_two_kg_one_meter() {
        // F = G*1*1/1² = G
        let out = gravitational_force("1", "1", "1");
        approx_field(&out, "RESULT", G, 1e-15);
    }

    #[test]
    fn gravitational_zero_distance_errors() {
        let out = gravitational_force("1", "1", "0");
        assert!(out.starts_with("GRAVITATIONAL_FORCE: ERROR"));
    }

    #[test]
    fn doppler_observer_approaching() {
        // Source at rest, observer moves toward it at half sound speed
        // f = f0 * (c + vo) / c
        let out = doppler_effect("440", "340", "0", "170");
        approx_field(&out, "RESULT", 440.0 * 510.0 / 340.0, 1e-6);
    }

    #[test]
    fn doppler_supersonic_source_is_domain_error() {
        // Classical Doppler breaks once the source exceeds the speed of
        // sound — the denominator `c - v_s` flips sign and the formula
        // returns a negative "apparent frequency" that has no physical
        // meaning (what actually happens is a shock wave). Regression:
        // used to return `-229.71 Hz` silently for `v_s = 1000 m/s,
        // c = 343 m/s`.
        let out = doppler_effect("440", "343", "1000", "0");
        assert!(out.starts_with("DOPPLER_EFFECT: ERROR"), "got {out}");
        assert!(
            out.contains("classical Doppler is undefined for supersonic source"),
            "got {out}"
        );
        // Sub-sonic recession still works (source moving away but slower
        // than sound).
        let ok = doppler_effect("440", "343", "100", "0");
        assert!(ok.starts_with("DOPPLER_EFFECT: OK"), "got {ok}");
    }

    #[test]
    fn doppler_supersonic_receding_source_is_domain_error() {
        // Same guard for a source moving away supersonically — `|v_s| > c`
        // is the classical-breakdown condition regardless of sign.
        let out = doppler_effect("440", "343", "-500", "0");
        assert!(out.starts_with("DOPPLER_EFFECT: ERROR"), "got {out}");
    }

    #[test]
    fn wave_length_speed_of_light_at_1_mhz() {
        // λ = c/f = 3e8 / 1e6 = 300
        let out = wave_length("1000000", "300000000");
        approx_field(&out, "RESULT", 300.0, 1e-6);
    }

    #[test]
    fn planck_energy_basic() {
        // E = h * f for f=1 Hz = h
        let out = planck_energy("1");
        approx_field(&out, "RESULT", H, 1e-40);
    }

    #[test]
    fn ideal_gas_solve_for_v() {
        // PV = nRT → V = nRT/P; n=1, T=273.15, P=101325 → V ≈ 0.02241 m³
        let out = ideal_gas_law("101325", "0", "1", "273.15", "V");
        approx_field(&out, "VALUE", 0.022_413_968, 1e-6);
    }

    #[test]
    fn ideal_gas_invalid_solve_for_errors() {
        let out = ideal_gas_law("1", "1", "1", "1", "Z");
        assert!(out.starts_with("IDEAL_GAS_LAW: ERROR"));
    }

    #[test]
    fn heat_transfer_basic() {
        // k=10, A=1, ΔT=20, L=2 → Q = 10*1*20/2 = 100
        let out = heat_transfer("10", "1", "20", "2");
        approx_field(&out, "RESULT", 100.0, 1e-9);
    }

    #[test]
    fn stefan_boltzmann_blackbody() {
        // ε=1, A=1, T=300 → P = σ * 1 * 300^4 ≈ 459.3 W
        let out = stefan_boltzmann("1", "1", "300");
        approx_field(&out, "RESULT", SIGMA * 300.0_f64.powi(4), 1e-6);
    }

    #[test]
    fn stefan_boltzmann_emissivity_out_of_range() {
        let out = stefan_boltzmann("2", "1", "300");
        assert!(out.starts_with("STEFAN_BOLTZMANN: ERROR"));
    }

    #[test]
    fn stefan_boltzmann_rejects_negative_area() {
        // Regression: without the area guard, `σεAT⁴` silently returned a
        // negative "radiated power" for a negative surface area — physically
        // meaningless. Sister tool `heat_transfer` already enforces this.
        let out = stefan_boltzmann("0.5", "-1", "300");
        assert!(out.starts_with("STEFAN_BOLTZMANN: ERROR"), "got {out}");
        assert!(out.contains("area must be non-negative"), "got {out}");
    }

    #[test]
    fn escape_velocity_earth_approx() {
        // M_earth = 5.972e24 kg, r = 6.371e6 m → v ≈ 11186 m/s
        let out = escape_velocity("5.972e24", "6.371e6");
        approx_field(&out, "RESULT", 11186.0, 5.0);
    }

    #[test]
    fn orbital_velocity_iss_altitude() {
        // M_earth, r ≈ 6.78e6 → v ≈ 7660 m/s
        let out = orbital_velocity("5.972e24", "6.78e6");
        approx_field(&out, "RESULT", 7660.0, 50.0);
    }

    #[test]
    fn kinematics_rejects_negative_time() {
        let out = kinematics("0", "9.81", "-5");
        assert!(
            out.starts_with("KINEMATICS: ERROR"),
            "expected error, got {out}"
        );
        assert!(out.contains("time must be non-negative"));
    }

    #[test]
    fn kinematics_accepts_negative_acceleration() {
        // Free-fall upward: a=-9.81 is a valid vector; t>=0 only
        let out = kinematics("0", "-9.81", "10");
        assert!(out.starts_with("KINEMATICS: OK"));
    }

    #[test]
    fn newtons_force_rejects_negative_mass() {
        let out = newtons_force("-5", "10");
        assert!(out.starts_with("NEWTONS_FORCE: ERROR"));
        assert!(out.contains("mass must be non-negative"));
    }

    #[test]
    fn newtons_force_zero_mass_ok() {
        let out = newtons_force("0", "10");
        assert!(out.contains("RESULT: 0.0"));
    }

    #[test]
    fn gravitational_force_rejects_negative_mass() {
        let out = gravitational_force("-1000", "1000", "1");
        assert!(out.starts_with("GRAVITATIONAL_FORCE: ERROR"));
        assert!(out.contains("masses must be non-negative"));
    }

    #[test]
    fn planck_energy_rejects_negative_frequency() {
        let out = planck_energy("-1000");
        assert!(out.starts_with("PLANCK_ENERGY: ERROR"));
        assert!(out.contains("frequency must be non-negative"));
    }

    #[test]
    fn planck_energy_zero_frequency_ok() {
        let out = planck_energy("0");
        assert!(out.contains("RESULT: 0.0"));
    }

    #[test]
    fn wave_length_rejects_negative_frequency() {
        let out = wave_length("-1000", "340");
        assert!(out.starts_with("WAVE_LENGTH: ERROR"));
        assert!(out.contains("frequency must be positive"));
    }

    #[test]
    fn wave_length_rejects_negative_wave_speed() {
        let out = wave_length("1000", "-340");
        assert!(out.starts_with("WAVE_LENGTH: ERROR"));
        assert!(out.contains("waveSpeed must be positive"));
    }

    #[test]
    fn heat_transfer_rejects_negative_conductivity() {
        let out = heat_transfer("-5", "1", "20", "1");
        assert!(out.starts_with("HEAT_TRANSFER: ERROR"));
        assert!(out.contains("thermalConductivity must be non-negative"));
    }

    #[test]
    fn heat_transfer_rejects_negative_area() {
        let out = heat_transfer("10", "-1", "20", "1");
        assert!(out.starts_with("HEAT_TRANSFER: ERROR"));
        assert!(out.contains("area must be non-negative"));
    }

    #[test]
    fn ideal_gas_rejects_negative_pressure_when_solving_v() {
        let out = ideal_gas_law("-100", "1", "1", "300", "V");
        assert!(out.starts_with("IDEAL_GAS_LAW: ERROR"));
        assert!(out.contains("pressure must be positive"));
    }

    #[test]
    fn ideal_gas_rejects_negative_volume_when_solving_n() {
        let out = ideal_gas_law("100", "-1", "1", "300", "n");
        assert!(out.starts_with("IDEAL_GAS_LAW: ERROR"));
        assert!(out.contains("volume must be positive"));
    }

    #[test]
    fn ideal_gas_rejects_zero_temperature_when_solving_p() {
        let out = ideal_gas_law("100", "1", "1", "0", "P");
        assert!(out.starts_with("IDEAL_GAS_LAW: ERROR"));
        assert!(out.contains("temperature must be positive"));
    }

    #[test]
    fn ideal_gas_rejects_negative_moles_when_solving_p() {
        let out = ideal_gas_law("100", "1", "-1", "300", "P");
        assert!(out.starts_with("IDEAL_GAS_LAW: ERROR"));
        assert!(out.contains("moles must be non-negative"));
    }

    #[test]
    fn projectile_rejects_negative_angle() {
        let out = projectile_motion("10", "-45", "9.81");
        assert!(out.starts_with("PROJECTILE_MOTION: ERROR"));
        assert!(out.contains("angle must be in [0, 180]"));
    }

    #[test]
    fn projectile_rejects_angle_above_180() {
        let out = projectile_motion("10", "200", "9.81");
        assert!(out.starts_with("PROJECTILE_MOTION: ERROR"));
    }

    #[test]
    fn projectile_backward_launch_signed_range() {
        // 135° is the mirror of 45°: same peak/time, range flipped to negative.
        let back = projectile_motion("10", "135", "9.81");
        let fwd = projectile_motion("10", "45", "9.81");
        assert!(back.starts_with("PROJECTILE_MOTION: OK"));
        // RANGE at 135° ≈ -1 · RANGE at 45° (same magnitude, opposite sign).
        assert!(
            back.contains("RANGE: -"),
            "expected backward range, got {back}"
        );
        assert!(fwd.contains("RANGE: ") && !fwd.contains("RANGE: -"));
    }

    #[test]
    fn projectile_vertical_launch_zero_range() {
        // At 90° the projectile goes straight up — range must be exactly 0.
        let out = projectile_motion("10", "90", "9.81");
        assert!(out.contains("RANGE: 0.0"), "got {out}");
    }
}
