//! Port of `NetworkCalculatorTool.java` — subnet, IP conversion, VLSM,
//! summarization, IPv6 compress/expand, and transfer-time / throughput helpers.
//!
//! Every public function returns `String`. Failures are surfaced inline as
//! `"Error: {message}"` messages whose text matches the Java source verbatim so
//! downstream MCP clients observe identical output.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use bigdecimal::{BigDecimal, Context, RoundingMode};
use num_bigint::BigInt;
use num_traits::{One, Zero};

use crate::engine::bigdecimal_ext::{DECIMAL128_PRECISION, strip_plain};
use crate::engine::unit_registry::{UnitCategory, convert as convert_unit, find_unit};

// ------------------------------------------------------------------ //
//  Constants (mirror Java)
// ------------------------------------------------------------------ //

const IPV4_BITS: u32 = 32;
const IPV6_BITS: u32 = 128;
const OCTET_COUNT: usize = 4;
const IPV6_GROUP_COUNT: usize = 8;
const CIDR_31: u32 = 31;
const BITS_PER_BYTE: i64 = 8;
const MIN_COMPRESS_LEN: usize = 2;
const SCALE: i64 = 20;
const ERROR_PREFIX: &str = "Error: ";
const TRUE_STR: &str = "true";
const FALSE_STR: &str = "false";

// ------------------------------------------------------------------ //
//  Error type
// ------------------------------------------------------------------ //

#[derive(Debug, thiserror::Error)]
enum NetError {
    #[error("{0}")]
    Msg(String),
}

impl NetError {
    fn new(msg: impl Into<String>) -> Self {
        Self::Msg(msg.into())
    }
}

fn err_str(e: NetError) -> String {
    format!("{ERROR_PREFIX}{e}")
}

// ------------------------------------------------------------------ //
//  Arithmetic helpers — DECIMAL128 precision + HALF_UP rounding
// ------------------------------------------------------------------ //

fn decimal128_ctx() -> Context {
    Context::default()
        .with_prec(DECIMAL128_PRECISION)
        .expect("DECIMAL128_PRECISION is non-zero")
        .with_rounding_mode(RoundingMode::HalfUp)
}

fn mul_ctx(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    decimal128_ctx().multiply(a, b)
}

fn div_scaled(a: &BigDecimal, b: &BigDecimal) -> BigDecimal {
    let quotient = a / b;
    quotient.with_scale_round(SCALE, RoundingMode::HalfUp)
}

fn strip(value: &BigDecimal) -> String {
    strip_plain(value)
}

// ------------------------------------------------------------------ //
//  Public API
// ------------------------------------------------------------------ //

/// Calculate subnet details. Returns JSON with network, broadcast, mask,
/// wildcard, first/last host, usable hosts, and IP class.
#[must_use]
pub fn subnet_calculator(address: &str, cidr: i32) -> String {
    let result = if is_ipv6(address) {
        subnet_v6(address, cidr)
    } else {
        subnet_v4(address, cidr)
    };
    unwrap_or_err(result)
}

/// Convert an IP to its binary representation.
#[must_use]
pub fn ip_to_binary(address: &str) -> String {
    let result = if is_ipv6(address) {
        ipv6_to_binary(address)
    } else {
        ipv4_to_binary(address)
    };
    unwrap_or_err(result)
}

/// Convert a binary IP representation back to decimal notation.
#[must_use]
pub fn binary_to_ip(binary: &str) -> String {
    let result = if binary.contains(':') {
        binary_to_ipv6(binary)
    } else {
        binary_to_ipv4(binary)
    };
    unwrap_or_err(result)
}

/// Convert an IP address to its unsigned decimal integer representation.
#[must_use]
pub fn ip_to_decimal(address: &str) -> String {
    let result = if is_ipv6(address) {
        parse_ipv6(address).map(|v| v.to_string())
    } else {
        parse_ipv4(address).map(|v| v.to_string())
    };
    unwrap_or_err(result)
}

/// Convert an unsigned decimal integer string to an IP address.
#[must_use]
pub fn decimal_to_ip(decimal: &str, version: i32) -> String {
    unwrap_or_err(convert_decimal_to_ip(decimal, version))
}

/// Whether an IP address is within the given subnet.
#[must_use]
pub fn ip_in_subnet(address: &str, network: &str, cidr: i32) -> String {
    let result = if is_ipv6(address) {
        check_ipv6_in_subnet(address, network, cidr)
    } else {
        check_ipv4_in_subnet(address, network, cidr)
    };
    unwrap_or_err(result)
}

/// VLSM subnet allocation. `host_counts_json` is a JSON array of host counts.
#[must_use]
pub fn vlsm_subnets(network_cidr: &str, host_counts_json: &str) -> String {
    unwrap_or_err(compute_vlsm(network_cidr, host_counts_json))
}

/// Summarize (supernet) a list of subnets into a single CIDR block.
#[must_use]
pub fn summarize_subnets(subnets_json: &str) -> String {
    unwrap_or_err(compute_summary(subnets_json))
}

/// Expand a compressed IPv6 address to its full 8-group form.
#[must_use]
pub fn expand_ipv6(address: &str) -> String {
    let result = parse_ipv6(address).map(|v| big_int_to_ipv6_full(&v));
    unwrap_or_err(result)
}

/// Compress an IPv6 address to its shortest canonical form using `::`.
#[must_use]
pub fn compress_ipv6(address: &str) -> String {
    let result = parse_ipv6(address).map(|v| compress_ipv6_groups(&big_int_to_ipv6_full(&v)));
    unwrap_or_err(result)
}

/// Estimate file transfer time. Returns JSON with seconds, minutes, hours.
#[must_use]
pub fn transfer_time(
    file_size: &str,
    file_size_unit: &str,
    bandwidth: &str,
    bandwidth_unit: &str,
) -> String {
    unwrap_or_err(compute_transfer_time(
        file_size,
        file_size_unit,
        bandwidth,
        bandwidth_unit,
    ))
}

/// Calculate data throughput given data size and time.
#[must_use]
pub fn throughput(
    data_size: &str,
    data_size_unit: &str,
    time: &str,
    time_unit: &str,
    output_unit: &str,
) -> String {
    unwrap_or_err(compute_throughput(
        data_size,
        data_size_unit,
        time,
        time_unit,
        output_unit,
    ))
}

/// Effective TCP throughput via bandwidth-delay product. Returns Mbps.
#[must_use]
pub fn tcp_throughput(bandwidth_mbps: &str, rtt_ms: &str, window_size_kb: &str) -> String {
    unwrap_or_err(compute_tcp_throughput(
        bandwidth_mbps,
        rtt_ms,
        window_size_kb,
    ))
}

// ------------------------------------------------------------------ //
//  Result adapter
// ------------------------------------------------------------------ //

fn unwrap_or_err(result: Result<String, NetError>) -> String {
    match result {
        Ok(value) => value,
        Err(e) => err_str(e),
    }
}

// ------------------------------------------------------------------ //
//  Decimal-to-IP dispatch
// ------------------------------------------------------------------ //

fn convert_decimal_to_ip(decimal: &str, version: i32) -> Result<String, NetError> {
    if version == 6 {
        let big = BigInt::from_str(decimal)
            .map_err(|_| NetError::new(format!("Invalid decimal: {decimal}")))?;
        Ok(big_int_to_ipv6_full(&big))
    } else if version == 4 {
        let val = decimal
            .parse::<i64>()
            .map_err(|_| NetError::new(format!("Invalid decimal: {decimal}")))?;
        long_to_ipv4(val)
    } else {
        Err(NetError::new("Version must be 4 or 6"))
    }
}

// ------------------------------------------------------------------ //
//  IPv4 helpers
// ------------------------------------------------------------------ //

fn parse_ipv4(address: &str) -> Result<i64, NetError> {
    // Match Java behavior: reject anything that isn't exactly 4 decimal octets
    // in range 0..=255. `Ipv4Addr::from_str` accepts the same set.
    let parts: Vec<&str> = address.split('.').collect();
    if parts.len() != OCTET_COUNT {
        return Err(NetError::new(format!("Invalid IPv4 address: {address}")));
    }
    let mut value: i64 = 0;
    for part in &parts {
        let octet: i32 = part
            .parse()
            .map_err(|_| NetError::new(format!("Invalid IPv4 address: {address}")))?;
        if !(0..=255).contains(&octet) {
            return Err(NetError::new(format!("Octet out of range: {octet}")));
        }
        value = (value << 8) | i64::from(octet);
    }
    // Sanity-check against std parser to keep parity with Java's Integer.parseInt strictness.
    if Ipv4Addr::from_str(address).is_err() {
        return Err(NetError::new(format!("Invalid IPv4 address: {address}")));
    }
    Ok(value)
}

fn long_to_ipv4(value: i64) -> Result<String, NetError> {
    const IPV4_MAX: i64 = 0xFFFF_FFFF;
    if !(0..=IPV4_MAX).contains(&value) {
        return Err(NetError::new(format!("Value out of IPv4 range: {value}")));
    }
    Ok(format!(
        "{}.{}.{}.{}",
        (value >> 24) & 0xFF,
        (value >> 16) & 0xFF,
        (value >> 8) & 0xFF,
        value & 0xFF
    ))
}

fn cidr_to_mask_v4(cidr: u32) -> i64 {
    if cidr == 0 {
        0
    } else {
        (0xFFFF_FFFFu64 << (IPV4_BITS - cidr) & 0xFFFF_FFFFu64) as i64
    }
}

fn ip_class(ip_value: i64) -> &'static str {
    let first_octet = ((ip_value >> 24) & 0xFF) as i32;
    if first_octet <= 127 {
        "A"
    } else if first_octet <= 191 {
        "B"
    } else if first_octet <= 223 {
        "C"
    } else if first_octet <= 239 {
        "D"
    } else {
        "E"
    }
}

// ------------------------------------------------------------------ //
//  IPv6 helpers
// ------------------------------------------------------------------ //

fn is_ipv6(address: &str) -> bool {
    address.contains(':')
}

fn parse_ipv6(address: &str) -> Result<BigInt, NetError> {
    // Use std to validate + extract the 128-bit integer.
    let parsed = Ipv6Addr::from_str(address)
        .map_err(|_| NetError::new(format!("Invalid IPv6 address: {address}")))?;
    let bits: u128 = parsed.to_bits();
    Ok(BigInt::from(bits))
}

fn big_int_to_ipv6_full(value: &BigInt) -> String {
    // Render as 32 hex digits, padded, then split into 8 groups of 4.
    let (_, mag) = value.to_bytes_be();
    let mut raw = String::with_capacity(32);
    for byte in &mag {
        raw.push_str(&format!("{byte:02x}"));
    }
    let hex = if raw.len() >= 32 {
        raw[raw.len() - 32..].to_string()
    } else {
        let mut s = String::with_capacity(32);
        s.push_str(&"0".repeat(32 - raw.len()));
        s.push_str(&raw);
        s
    };
    let mut out = String::with_capacity(39);
    for idx in 0..IPV6_GROUP_COUNT {
        if idx > 0 {
            out.push(':');
        }
        let start = idx * 4;
        out.push_str(&hex[start..start + 4]);
    }
    out
}

fn cidr_to_mask_v6(cidr: u32) -> BigInt {
    let all_ones = (BigInt::one() << IPV6_BITS) - BigInt::one();
    if cidr == 0 {
        BigInt::zero()
    } else {
        let shifted = &all_ones >> cidr;
        let inverted = !shifted;
        inverted & &all_ones
    }
}

// ------------------------------------------------------------------ //
//  Binary helpers
// ------------------------------------------------------------------ //

fn to_binary8(octet: u32) -> String {
    format!("{octet:08b}")
}

fn to_binary16(group: u32) -> String {
    format!("{group:016b}")
}

// ------------------------------------------------------------------ //
//  Validation
// ------------------------------------------------------------------ //

fn validate_cidr(cidr: i32, ipv6: bool) -> Result<u32, NetError> {
    let max: i32 = if ipv6 { 128 } else { 32 };
    if cidr < 0 || cidr > max {
        return Err(NetError::new(format!("CIDR must be 0-{max}, got {cidr}")));
    }
    Ok(cidr as u32)
}

// ------------------------------------------------------------------ //
//  IP-in-subnet checks
// ------------------------------------------------------------------ //

fn check_ipv6_in_subnet(address: &str, network: &str, cidr: i32) -> Result<String, NetError> {
    let cidr = validate_cidr(cidr, true)?;
    let ip_val = parse_ipv6(address)?;
    let net_val = parse_ipv6(network)?;
    let mask = cidr_to_mask_v6(cidr);
    Ok(if (&ip_val & &mask) == (&net_val & &mask) {
        TRUE_STR.to_string()
    } else {
        FALSE_STR.to_string()
    })
}

fn check_ipv4_in_subnet(address: &str, network: &str, cidr: i32) -> Result<String, NetError> {
    let cidr = validate_cidr(cidr, false)?;
    let ip_val = parse_ipv4(address)?;
    let net_val = parse_ipv4(network)?;
    let mask = cidr_to_mask_v4(cidr);
    Ok(if (ip_val & mask) == (net_val & mask) {
        TRUE_STR.to_string()
    } else {
        FALSE_STR.to_string()
    })
}

// ------------------------------------------------------------------ //
//  Subnet calculation (v4)
// ------------------------------------------------------------------ //

fn subnet_v4(address: &str, cidr: i32) -> Result<String, NetError> {
    let cidr = validate_cidr(cidr, false)?;
    let ip_val = parse_ipv4(address)?;
    let mask = cidr_to_mask_v4(cidr);
    let network = ip_val & mask;
    let wildcard = !mask & 0xFFFF_FFFF_i64;
    let broadcast = network | wildcard;
    build_subnet_v4_json(network, broadcast, mask, wildcard, cidr, ip_val)
}

fn build_subnet_v4_json(
    network: i64,
    broadcast: i64,
    mask: i64,
    wildcard: i64,
    cidr: u32,
    ip_value: i64,
) -> Result<String, NetError> {
    let (first_host, last_host, usable_hosts) = if cidr == IPV4_BITS {
        (network, network, 0_i64)
    } else if cidr == CIDR_31 {
        (network, broadcast, 2_i64)
    } else {
        (network + 1, broadcast - 1, broadcast - network - 1)
    };
    Ok(format!(
        "{{\"network\":\"{}\",\"broadcast\":\"{}\",\"mask\":\"{}\",\"wildcard\":\"{}\",\"firstHost\":\"{}\",\"lastHost\":\"{}\",\"usableHosts\":{},\"ipClass\":\"{}\"}}",
        long_to_ipv4(network)?,
        long_to_ipv4(broadcast)?,
        long_to_ipv4(mask)?,
        long_to_ipv4(wildcard)?,
        long_to_ipv4(first_host)?,
        long_to_ipv4(last_host)?,
        usable_hosts,
        ip_class(ip_value)
    ))
}

// ------------------------------------------------------------------ //
//  Subnet calculation (v6)
// ------------------------------------------------------------------ //

fn subnet_v6(address: &str, cidr: i32) -> Result<String, NetError> {
    let cidr = validate_cidr(cidr, true)?;
    let ip_val = parse_ipv6(address)?;
    let mask = cidr_to_mask_v6(cidr);
    let network = &ip_val & &mask;
    let host_bits = IPV6_BITS - cidr;
    Ok(build_subnet_v6_json(&network, &mask, host_bits))
}

fn build_subnet_v6_json(network: &BigInt, mask: &BigInt, host_bits: u32) -> String {
    let host_range = if host_bits == 0 {
        BigInt::zero()
    } else {
        (BigInt::one() << host_bits) - BigInt::one()
    };
    let last = network | &host_range;
    let (first_host, last_host, usable_hosts) = if host_bits == 0 {
        (network.clone(), network.clone(), BigInt::zero())
    } else if host_bits == 1 {
        (network.clone(), last.clone(), BigInt::from(2u32))
    } else {
        (
            network + BigInt::one(),
            &last - BigInt::one(),
            &host_range - BigInt::one(),
        )
    };
    format!(
        "{{\"network\":\"{}\",\"mask\":\"{}\",\"firstHost\":\"{}\",\"lastHost\":\"{}\",\"usableHosts\":{}}}",
        big_int_to_ipv6_full(network),
        big_int_to_ipv6_full(mask),
        big_int_to_ipv6_full(&first_host),
        big_int_to_ipv6_full(&last_host),
        usable_hosts
    )
}

// ------------------------------------------------------------------ //
//  Binary conversion methods
// ------------------------------------------------------------------ //

fn ipv4_to_binary(address: &str) -> Result<String, NetError> {
    let value = parse_ipv4(address)?;
    Ok(format!(
        "{}.{}.{}.{}",
        to_binary8(((value >> 24) & 0xFF) as u32),
        to_binary8(((value >> 16) & 0xFF) as u32),
        to_binary8(((value >> 8) & 0xFF) as u32),
        to_binary8((value & 0xFF) as u32)
    ))
}

fn ipv6_to_binary(address: &str) -> Result<String, NetError> {
    let full = big_int_to_ipv6_full(&parse_ipv6(address)?);
    let mut out = String::with_capacity(143);
    for (idx, group) in full.split(':').enumerate() {
        if idx > 0 {
            out.push(':');
        }
        let parsed = u32::from_str_radix(group, 16)
            .map_err(|_| NetError::new(format!("Invalid IPv6 address: {address}")))?;
        out.push_str(&to_binary16(parsed));
    }
    Ok(out)
}

fn binary_to_ipv4(binary: &str) -> Result<String, NetError> {
    let parts: Vec<&str> = binary.split('.').collect();
    if parts.len() != OCTET_COUNT {
        return Err(NetError::new("Expected 4 dot-separated 8-bit groups"));
    }
    let mut value: i64 = 0;
    for part in &parts {
        let group = i64::from_str_radix(part, 2)
            .map_err(|_| NetError::new("Expected 4 dot-separated 8-bit groups"))?;
        value = (value << 8) | group;
    }
    long_to_ipv4(value)
}

fn binary_to_ipv6(binary: &str) -> Result<String, NetError> {
    let parts: Vec<&str> = binary.split(':').collect();
    if parts.len() != IPV6_GROUP_COUNT {
        return Err(NetError::new("Expected 8 colon-separated 16-bit groups"));
    }
    let mut value = BigInt::zero();
    for part in &parts {
        let group = u32::from_str_radix(part, 2)
            .map_err(|_| NetError::new("Expected 8 colon-separated 16-bit groups"))?;
        value = (value << 16) | BigInt::from(group);
    }
    Ok(big_int_to_ipv6_full(&value))
}

// ------------------------------------------------------------------ //
//  IPv6 compress
// ------------------------------------------------------------------ //

fn compress_ipv6_groups(full: &str) -> String {
    let groups: Vec<&str> = full.split(':').collect();
    let mut best_start: isize = -1;
    let mut best_len: usize = 0;
    let mut cur_start: isize = -1;
    let mut cur_len: usize = 0;

    for (idx, group) in groups.iter().enumerate() {
        if *group == "0000" {
            if cur_start < 0 {
                cur_start = idx as isize;
                cur_len = 1;
            } else {
                cur_len += 1;
            }
        } else {
            if cur_len > best_len {
                best_start = cur_start;
                best_len = cur_len;
            }
            cur_start = -1;
            cur_len = 0;
        }
    }
    if cur_len > best_len {
        best_start = cur_start;
        best_len = cur_len;
    }
    build_compressed(&groups, best_start, best_len)
}

fn build_compressed(groups: &[&str], best_start: isize, best_len: usize) -> String {
    if best_len < MIN_COMPRESS_LEN {
        join_trimmed(groups, 0, groups.len())
    } else {
        let start = best_start as usize;
        let left = join_trimmed(groups, 0, start);
        let right = join_trimmed(groups, start + best_len, groups.len());
        format!("{left}::{right}")
    }
}

fn join_trimmed(groups: &[&str], from: usize, end: usize) -> String {
    let mut out = String::new();
    for group in &groups[from..end] {
        if !out.is_empty() {
            out.push(':');
        }
        out.push_str(&trim_leading_zeros(group));
    }
    out
}

fn trim_leading_zeros(group: &str) -> String {
    // Match Java's `^0+(?!$)` — strip leading zeros unless the group is all zeros.
    let trimmed = group.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

// ------------------------------------------------------------------ //
//  VLSM
// ------------------------------------------------------------------ //

fn compute_vlsm(network_cidr: &str, host_counts_json: &str) -> Result<String, NetError> {
    let cidr_parts: Vec<&str> = network_cidr.split('/').collect();
    if cidr_parts.len() != 2 {
        return Err(NetError::new(format!(
            "Invalid IPv4 address: {network_cidr}"
        )));
    }
    let base_network = parse_ipv4(cidr_parts[0])?;
    let base_cidr: i32 = cidr_parts[1]
        .parse()
        .map_err(|_| NetError::new(format!("CIDR must be 0-32, got {}", cidr_parts[1])))?;
    let base_cidr_u = validate_cidr(base_cidr, false)?;
    let base_mask = cidr_to_mask_v4(base_cidr_u);
    let base_end = base_network | (!base_mask & 0xFFFF_FFFF_i64);

    let mut counts = parse_int_array(host_counts_json)?;
    counts.sort_by(|a, b| b.cmp(a));

    let mut pointer = base_network;
    let mut out = String::from("[");

    for (idx, &needed) in counts.iter().enumerate() {
        let host_bits = ceil_log2(needed + 2);
        let subnet_cidr_i = IPV4_BITS as i32 - host_bits;
        validate_vlsm_fit(needed, subnet_cidr_i, base_cidr)?;
        let subnet_cidr = subnet_cidr_i as u32;

        let sub_mask = cidr_to_mask_v4(subnet_cidr);
        let sub_broadcast = pointer | (!sub_mask & 0xFFFF_FFFF_i64);
        if sub_broadcast > base_end {
            return Err(NetError::new("Address space exhausted"));
        }

        if idx > 0 {
            out.push(',');
        }
        append_vlsm_entry(&mut out, pointer, subnet_cidr, sub_broadcast)?;
        pointer = sub_broadcast + 1;
    }
    out.push(']');
    Ok(out)
}

fn validate_vlsm_fit(needed: i32, subnet_cidr: i32, base_cidr: i32) -> Result<(), NetError> {
    if subnet_cidr < base_cidr {
        return Err(NetError::new(format!(
            "Cannot fit {needed} hosts in /{base_cidr}"
        )));
    }
    Ok(())
}

fn append_vlsm_entry(
    out: &mut String,
    network: i64,
    cidr: u32,
    broadcast: i64,
) -> Result<(), NetError> {
    let usable = broadcast - network - 1;
    out.push_str("{\"network\":\"");
    out.push_str(&long_to_ipv4(network)?);
    out.push_str("\",\"cidr\":");
    out.push_str(&cidr.to_string());
    out.push_str(",\"firstHost\":\"");
    out.push_str(&long_to_ipv4(network + 1)?);
    out.push_str("\",\"lastHost\":\"");
    out.push_str(&long_to_ipv4(broadcast - 1)?);
    out.push_str("\",\"usableHosts\":");
    out.push_str(&usable.to_string());
    out.push('}');
    Ok(())
}

fn ceil_log2(value: i32) -> i32 {
    let mut bits: i32 = 0;
    let mut remaining = value - 1;
    while remaining > 0 {
        remaining >>= 1;
        bits += 1;
    }
    bits
}

// ------------------------------------------------------------------ //
//  Summarize subnets
// ------------------------------------------------------------------ //

fn compute_summary(subnets_json: &str) -> Result<String, NetError> {
    let cidr_list = parse_string_array(subnets_json)?;
    if cidr_list.is_empty() {
        return Err(NetError::new("Empty subnet list"));
    }
    let mut min_network: i64 = 0xFFFF_FFFF;
    let mut max_broadcast: i64 = 0;
    for cidr in &cidr_list {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err(NetError::new(format!("Invalid IPv4 address: {cidr}")));
        }
        let network = parse_ipv4(parts[0])?;
        let prefix: i32 = parts[1]
            .parse()
            .map_err(|_| NetError::new(format!("CIDR must be 0-32, got {}", parts[1])))?;
        let prefix_u = validate_cidr(prefix, false)?;
        let mask = cidr_to_mask_v4(prefix_u);
        let broadcast = network | (!mask & 0xFFFF_FFFF_i64);
        if network < min_network {
            min_network = network;
        }
        if broadcast > max_broadcast {
            max_broadcast = broadcast;
        }
    }
    let range = (max_broadcast - min_network + 1) as i32;
    let super_bits = ceil_log2(range);
    let super_cidr = IPV4_BITS as i32 - super_bits;
    let super_mask = cidr_to_mask_v4(super_cidr as u32);
    let super_network = min_network & super_mask;
    Ok(format!("{}/{}", long_to_ipv4(super_network)?, super_cidr))
}

// ------------------------------------------------------------------ //
//  Transfer time / throughput
// ------------------------------------------------------------------ //

fn require_category(code: &str, category: UnitCategory, label: &str) -> Result<(), NetError> {
    match find_unit(code) {
        Some(unit) if unit.category == category => Ok(()),
        Some(unit) => Err(NetError::new(format!(
            "Unit '{}' is not in category {} (expected for {})",
            unit.code,
            category.as_str(),
            label
        ))),
        None => Err(NetError::new(format!("Unknown unit: {code}"))),
    }
}

fn unit_convert(value: &BigDecimal, from: &str, to: &str) -> Result<BigDecimal, NetError> {
    convert_unit(value, from, to).map_err(|e| NetError::new(e.to_string()))
}

fn parse_decimal(input: &str, label: &str) -> Result<BigDecimal, NetError> {
    BigDecimal::from_str(input)
        .map_err(|_| NetError::new(format!("Invalid {label} value: {input}")))
}

fn compute_transfer_time(
    file_size: &str,
    file_size_unit: &str,
    bandwidth: &str,
    bandwidth_unit: &str,
) -> Result<String, NetError> {
    let size_unit = file_size_unit.to_ascii_lowercase();
    let bw_unit = bandwidth_unit.to_ascii_lowercase();
    require_category(&size_unit, UnitCategory::DataStorage, "fileSizeUnit")?;
    require_category(&bw_unit, UnitCategory::DataRate, "bandwidthUnit")?;

    let size_value = parse_decimal(file_size, "fileSize")?;
    let bandwidth_value = parse_decimal(bandwidth, "bandwidth")?;

    let size_bytes = unit_convert(&size_value, &size_unit, "byte")?;
    let size_bits = mul_ctx(&size_bytes, &BigDecimal::from(BITS_PER_BYTE));
    let bps = unit_convert(&bandwidth_value, &bw_unit, "bps")?;

    let seconds = div_scaled(&size_bits, &bps);
    let minutes = div_scaled(&seconds, &BigDecimal::from(60));
    let hours = div_scaled(&seconds, &BigDecimal::from(3600));

    Ok(format!(
        "{{\"seconds\":\"{}\",\"minutes\":\"{}\",\"hours\":\"{}\"}}",
        strip(&seconds),
        strip(&minutes),
        strip(&hours)
    ))
}

fn compute_throughput(
    data_size: &str,
    data_size_unit: &str,
    time: &str,
    time_unit: &str,
    output_unit: &str,
) -> Result<String, NetError> {
    let size_unit = data_size_unit.to_ascii_lowercase();
    let tu = time_unit.to_ascii_lowercase();
    let out_unit = output_unit.to_ascii_lowercase();

    require_category(&size_unit, UnitCategory::DataStorage, "dataSizeUnit")?;
    require_category(&tu, UnitCategory::Time, "timeUnit")?;
    require_category(&out_unit, UnitCategory::DataRate, "outputUnit")?;

    let size_value = parse_decimal(data_size, "dataSize")?;
    let time_value = parse_decimal(time, "time")?;

    let size_bytes = unit_convert(&size_value, &size_unit, "byte")?;
    let size_bits = mul_ctx(&size_bytes, &BigDecimal::from(BITS_PER_BYTE));
    let seconds = unit_convert(&time_value, &tu, "s")?;
    let bps = div_scaled(&size_bits, &seconds);
    let result = unit_convert(&bps, "bps", &out_unit)?;
    Ok(strip(&result))
}

fn compute_tcp_throughput(
    bandwidth_mbps: &str,
    rtt_ms: &str,
    window_size_kb: &str,
) -> Result<String, NetError> {
    let million = BigDecimal::from(1_000_000);
    let thousand = BigDecimal::from(1_000);
    let kilo_bits = BigDecimal::from(8192);

    let bw = parse_decimal(bandwidth_mbps, "bandwidthMbps")?;
    let rtt = parse_decimal(rtt_ms, "rttMs")?;
    let window = parse_decimal(window_size_kb, "windowSizeKb")?;

    let bw_bps = mul_ctx(&bw, &million);
    let rtt_sec = div_scaled(&rtt, &thousand);
    let window_bits = mul_ctx(&window, &kilo_bits);
    let max_by_window = div_scaled(&window_bits, &rtt_sec);
    let effective = if bw_bps <= max_by_window {
        bw_bps
    } else {
        max_by_window
    };
    let effective_mbps = div_scaled(&effective, &million);
    Ok(strip(&effective_mbps))
}

// ------------------------------------------------------------------ //
//  JSON array parsing helpers
// ------------------------------------------------------------------ //

fn parse_int_array(json: &str) -> Result<Vec<i32>, NetError> {
    match serde_json::from_str::<Vec<i32>>(json) {
        Ok(v) => Ok(v),
        Err(_) => {
            // Match Java's lenient manual parser.
            let trimmed = json.trim();
            if trimmed.len() < 2 || !trimmed.starts_with('[') || !trimmed.ends_with(']') {
                return Err(NetError::new(format!("Invalid JSON array: {json}")));
            }
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Ok(Vec::new());
            }
            inner
                .split(',')
                .map(|el| {
                    el.trim()
                        .parse::<i32>()
                        .map_err(|_| NetError::new(format!("Invalid integer: {}", el.trim())))
                })
                .collect()
        }
    }
}

fn parse_string_array(json: &str) -> Result<Vec<String>, NetError> {
    match serde_json::from_str::<Vec<String>>(json) {
        Ok(v) => Ok(v),
        Err(_) => {
            let trimmed = json.trim();
            if trimmed.len() < 2 || !trimmed.starts_with('[') || !trimmed.ends_with(']') {
                return Err(NetError::new(format!("Invalid JSON array: {json}")));
            }
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Ok(Vec::new());
            }
            Ok(inner
                .split(',')
                .map(|el| el.trim().replace('"', ""))
                .collect())
        }
    }
}

// ------------------------------------------------------------------ //
//  Tests
// ------------------------------------------------------------------ //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subnet_calc_192_168_1_0_slash_24() {
        let json = subnet_calculator("192.168.1.0", 24);
        assert!(json.contains("\"network\":\"192.168.1.0\""));
        assert!(json.contains("\"broadcast\":\"192.168.1.255\""));
        assert!(json.contains("\"mask\":\"255.255.255.0\""));
        assert!(json.contains("\"wildcard\":\"0.0.0.255\""));
        assert!(json.contains("\"firstHost\":\"192.168.1.1\""));
        assert!(json.contains("\"lastHost\":\"192.168.1.254\""));
        assert!(json.contains("\"usableHosts\":254"));
        assert!(json.contains("\"ipClass\":\"C\""));
    }

    #[test]
    fn subnet_calc_cidr_31_point_to_point() {
        let json = subnet_calculator("10.0.0.0", 31);
        assert!(json.contains("\"network\":\"10.0.0.0\""));
        assert!(json.contains("\"broadcast\":\"10.0.0.1\""));
        assert!(json.contains("\"firstHost\":\"10.0.0.0\""));
        assert!(json.contains("\"lastHost\":\"10.0.0.1\""));
        assert!(json.contains("\"usableHosts\":2"));
        assert!(json.contains("\"ipClass\":\"A\""));
    }

    #[test]
    fn subnet_calc_ipv6() {
        let json = subnet_calculator("2001:db8::", 64);
        assert!(json.contains("\"network\":\"2001:0db8:0000:0000:0000:0000:0000:0000\""));
        assert!(json.contains("\"mask\":\"ffff:ffff:ffff:ffff:0000:0000:0000:0000\""));
        // 2^64 - 2 for usable hosts
        assert!(json.contains("\"usableHosts\":18446744073709551614"));
    }

    #[test]
    fn ip_to_binary_ipv4_roundtrip() {
        let bin = ip_to_binary("192.168.1.1");
        assert_eq!(bin, "11000000.10101000.00000001.00000001");
        let back = binary_to_ip(&bin);
        assert_eq!(back, "192.168.1.1");
    }

    #[test]
    fn ip_to_binary_ipv6_roundtrip() {
        let bin = ip_to_binary("::1");
        // 7 groups of 16 zero bits, then '...0001'
        assert!(bin.ends_with("0000000000000001"));
        assert_eq!(bin.matches(':').count(), 7);
        let back = binary_to_ip(&bin);
        assert_eq!(back, "0000:0000:0000:0000:0000:0000:0000:0001");
    }

    #[test]
    fn ip_to_decimal_ipv4_roundtrip() {
        let dec = ip_to_decimal("192.168.1.1");
        assert_eq!(dec, "3232235777");
        let back = decimal_to_ip(&dec, 4);
        assert_eq!(back, "192.168.1.1");
    }

    #[test]
    fn ip_to_decimal_ipv6_roundtrip() {
        let dec = ip_to_decimal("::1");
        assert_eq!(dec, "1");
        let back = decimal_to_ip(&dec, 6);
        assert_eq!(back, "0000:0000:0000:0000:0000:0000:0000:0001");
    }

    #[test]
    fn ip_in_subnet_cases() {
        assert_eq!(ip_in_subnet("192.168.1.50", "192.168.1.0", 24), "true");
        assert_eq!(ip_in_subnet("192.168.2.1", "192.168.1.0", 24), "false");
        assert_eq!(ip_in_subnet("2001:db8::1", "2001:db8::", 64), "true");
        assert_eq!(ip_in_subnet("2001:dc8::1", "2001:db8::", 64), "false");
    }

    #[test]
    fn vlsm_basic_slash_24() {
        let out = vlsm_subnets("192.168.1.0/24", "[50, 25, 10]");
        // Largest (50) → /26 (64 hosts) at .0, next (25) → /27 at .64, (10) → /28 at .96
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
        assert!(out.contains("\"network\":\"192.168.1.0\""));
        assert!(out.contains("\"cidr\":26"));
        assert!(out.contains("\"network\":\"192.168.1.64\""));
        assert!(out.contains("\"cidr\":27"));
        assert!(out.contains("\"network\":\"192.168.1.96\""));
        assert!(out.contains("\"cidr\":28"));
    }

    #[test]
    fn vlsm_cannot_fit() {
        let out = vlsm_subnets("192.168.1.0/28", "[100]");
        assert!(out.starts_with("Error: Cannot fit 100 hosts in /28"));
    }

    #[test]
    fn summarize_two_slash_25_to_slash_24() {
        let out = summarize_subnets("[\"192.168.0.0/25\",\"192.168.0.128/25\"]");
        assert_eq!(out, "192.168.0.0/24");
    }

    #[test]
    fn summarize_adjacent_slash_22() {
        let out = summarize_subnets(
            "[\"192.168.0.0/24\",\"192.168.1.0/24\",\"192.168.2.0/24\",\"192.168.3.0/24\"]",
        );
        assert_eq!(out, "192.168.0.0/22");
    }

    #[test]
    fn expand_compress_ipv6_roundtrip() {
        let expanded = expand_ipv6("::1");
        assert_eq!(expanded, "0000:0000:0000:0000:0000:0000:0000:0001");
        let compressed = compress_ipv6(&expanded);
        assert_eq!(compressed, "::1");
    }

    #[test]
    fn compress_ipv6_middle_run() {
        let compressed = compress_ipv6("2001:0db8:0000:0000:0000:0000:0000:0001");
        assert_eq!(compressed, "2001:db8::1");
    }

    #[test]
    fn transfer_time_1gb_at_100mbps() {
        let json = transfer_time("1", "gb", "100", "mbps");
        // 1 GB = 8589934592 bits. /100_000_000 = 85.89934592 s
        assert!(json.contains("\"seconds\":\"85.89934592\""));
        assert!(json.contains("\"minutes\":"));
        assert!(json.contains("\"hours\":"));
    }

    #[test]
    fn throughput_100mb_10s_to_mbps() {
        let out = throughput("100", "mb", "10", "s", "mbps");
        // 100 MB = 838860800 bits; /10 = 83886080 bps = 83.88608 Mbps (stripped)
        assert_eq!(out, "83.88608");
    }

    #[test]
    fn tcp_throughput_window_limited() {
        // bw = 1000 Mbps = 1e9 bps; window = 64 KB = 64*8192=524288 bits;
        // rtt = 100 ms = 0.1 s; max_by_window = 5242880 bps = 5.24288 Mbps
        let out = tcp_throughput("1000", "100", "64");
        assert_eq!(out, "5.24288");
    }

    #[test]
    fn tcp_throughput_bw_limited() {
        // bw = 10 Mbps = 1e7 bps; window = 1024 KB, rtt = 10 ms -> huge max_by_window
        let out = tcp_throughput("10", "10", "1024");
        assert_eq!(out, "10");
    }

    #[test]
    fn error_bad_ip() {
        assert_eq!(
            ip_to_decimal("999.999.999.999"),
            "Error: Octet out of range: 999"
        );
    }

    #[test]
    fn error_bad_cidr() {
        assert_eq!(
            subnet_calculator("192.168.1.0", 33),
            "Error: CIDR must be 0-32, got 33"
        );
    }

    #[test]
    fn error_wrong_version() {
        assert_eq!(decimal_to_ip("1", 5), "Error: Version must be 4 or 6");
    }

    #[test]
    fn error_empty_summary_list() {
        assert_eq!(summarize_subnets("[]"), "Error: Empty subnet list");
    }

    #[test]
    fn error_binary_to_ipv4_group_count() {
        assert_eq!(
            binary_to_ip("1010.1010"),
            "Error: Expected 4 dot-separated 8-bit groups"
        );
    }
}
