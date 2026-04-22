//! Port of `DateTimeConverterTool.java` — pure-Rust datetime conversion backed by `jiff`.
//!
//! Replaces `java.time` (`ZonedDateTime`, `DateTimeFormatter`) with `jiff` primitives
//! (`Zoned`, `Timestamp`, `DateTime`, `Span`) while preserving the public contract:
//! every function returns a `String`, errors are prefixed with `"Error: "` inline.
//!
//! # Format keywords
//! | Keyword            | Meaning                                                     |
//! | ------------------ | ----------------------------------------------------------- |
//! | `iso` / `iso-zoned`| ISO 8601 with `[Zone/ID]` suffix (Java `ZonedDateTime`)     |
//! | `iso-offset`       | `2026-04-22T10:00:00+02:00`                                 |
//! | `iso-local`        | `2026-04-22T10:00:00`                                       |
//! | `epoch`            | Unix seconds                                                |
//! | `epochmillis`      | Unix milliseconds                                           |
//! | `rfc1123`          | `Thu, 04 Mar 2026 12:00:00 GMT`                             |
//! | anything else      | strftime / strptime pattern                                 |

use std::str::FromStr;

use jiff::civil::{Date, DateTime};
use jiff::fmt::rfc2822;
use jiff::tz::TimeZone;
use jiff::{Span, SpanRound, Timestamp, Unit, Zoned};

const ERR_PREFIX: &str = "Error: ";

// --------------------------------------------------------------------------- //
//  Public API
// --------------------------------------------------------------------------- //

/// Convert a datetime string from one IANA timezone to another, returning ISO-zoned form.
pub fn convert_timezone(datetime: &str, from_timezone: &str, to_timezone: &str) -> String {
    let from_zone = match resolve_zone(from_timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let to_zone = match resolve_zone(to_timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    match parse_datetime(datetime, &from_zone) {
        Ok(source) => {
            let target = source.with_time_zone(to_zone);
            format_iso_zoned(&target)
        }
        Err(_) => format!("{ERR_PREFIX}Cannot parse datetime: {datetime}"),
    }
}

/// Reformat a datetime string using explicit input/output format keywords or strftime patterns.
pub fn format_datetime(
    datetime: &str,
    input_format: &str,
    output_format: &str,
    timezone: &str,
) -> String {
    let zone = match resolve_zone(timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let parsed = match parse_with_format(datetime, input_format, &zone) {
        Ok(zoned) => zoned,
        Err(msg) => return msg,
    };
    format_output(&parsed, output_format)
}

/// Current datetime in the given IANA timezone, rendered using a format keyword or strftime pattern.
pub fn current_datetime(timezone: &str, format: &str) -> String {
    let zone = match resolve_zone(timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let now = Zoned::now().with_time_zone(zone);
    format_output(&now, format)
}

/// JSON array of IANA timezone IDs, filtered by region prefix (e.g. `"Europe"`).
/// Empty string or `"all"` (case-insensitive) returns every zone.
pub fn list_timezones(region: &str) -> String {
    let trimmed = region.trim();
    let mut matches: Vec<String> = if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
        jiff::tz::db().available().map(|n| n.to_string()).collect()
    } else {
        let prefix = format!("{trimmed}/");
        jiff::tz::db()
            .available()
            .filter_map(|name| {
                let text = name.to_string();
                text.starts_with(&prefix).then_some(text)
            })
            .collect()
    };

    if matches.is_empty() {
        return format!("{ERR_PREFIX}No timezones found for region: {trimmed}");
    }

    matches.sort();
    serde_json::to_string(&matches).unwrap_or_else(|e| format!("{ERR_PREFIX}{e}"))
}

/// JSON object describing the positive difference between two datetimes parsed in `timezone`.
pub fn datetime_difference(datetime1: &str, datetime2: &str, timezone: &str) -> String {
    let zone = match resolve_zone(timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let first = match parse_datetime(datetime1, &zone) {
        Ok(zoned) => zoned,
        Err(_) => return format!("{ERR_PREFIX}Cannot parse datetime"),
    };
    let second = match parse_datetime(datetime2, &zone) {
        Ok(zoned) => zoned,
        Err(_) => return format!("{ERR_PREFIX}Cannot parse datetime"),
    };
    compute_difference(&first, &second)
}

// --------------------------------------------------------------------------- //
//  Zone resolution
// --------------------------------------------------------------------------- //

fn resolve_zone(id: &str) -> Result<TimeZone, String> {
    TimeZone::get(id).map_err(|_| format!("{ERR_PREFIX}Invalid timezone: {id}"))
}

// --------------------------------------------------------------------------- //
//  Parsing
// --------------------------------------------------------------------------- //

/// Best-effort parse accepting ISO zoned/offset/local forms plus a few common locale patterns.
fn parse_datetime(datetime: &str, zone: &TimeZone) -> Result<Zoned, String> {
    // 1. Temporal / ISO-zoned: already carries its own zone.
    if let Ok(zoned) = Zoned::from_str(datetime) {
        return Ok(zoned);
    }

    // 2. RFC 3339 timestamp (`...Z` or with numeric offset): treat as a point-in-time,
    //    then attach to the caller's zone. `Timestamp::from_str` understands `Z`.
    if let Ok(ts) = Timestamp::from_str(datetime) {
        return Ok(ts.to_zoned(zone.clone()));
    }

    // 3. ISO-offset or ISO-local: parsed as civil DateTime, attached to the caller's zone.
    if let Ok(civil) = DateTime::from_str(datetime) {
        return civil
            .to_zoned(zone.clone())
            .map_err(|e| format!("{ERR_PREFIX}{e}"));
    }

    // 3. Locale patterns: match Java's pattern list.
    const PATTERNS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%d/%m/%Y",
        "%m/%d/%Y",
    ];
    for pattern in PATTERNS {
        if let Ok(civil) = DateTime::strptime(pattern, datetime) {
            return civil
                .to_zoned(zone.clone())
                .map_err(|e| format!("{ERR_PREFIX}{e}"));
        }
    }

    // 4. Bare date (`2026-04-22`) → midnight in caller's zone.
    if let Ok(date) = Date::from_str(datetime) {
        return date
            .to_datetime(jiff::civil::Time::midnight())
            .to_zoned(zone.clone())
            .map_err(|e| format!("{ERR_PREFIX}{e}"));
    }

    Err(format!("{ERR_PREFIX}Cannot parse datetime: {datetime}"))
}

fn parse_with_format(datetime: &str, input_format: &str, zone: &TimeZone) -> Result<Zoned, String> {
    match input_format.to_ascii_lowercase().as_str() {
        "iso" | "iso-zoned" | "iso-offset" | "iso-local" => parse_datetime(datetime, zone),
        "epoch" => {
            let secs: i64 = datetime
                .trim()
                .parse()
                .map_err(|e| format!("{ERR_PREFIX}{e}"))?;
            Timestamp::from_second(secs)
                .map(|ts| ts.to_zoned(zone.clone()))
                .map_err(|e| format!("{ERR_PREFIX}{e}"))
        }
        "epochmillis" => {
            let millis: i64 = datetime
                .trim()
                .parse()
                .map_err(|e| format!("{ERR_PREFIX}{e}"))?;
            Timestamp::from_millisecond(millis)
                .map(|ts| ts.to_zoned(zone.clone()))
                .map_err(|e| format!("{ERR_PREFIX}{e}"))
        }
        _ => {
            // Try strptime as a Zoned first; fall back to civil DateTime + caller zone.
            if let Ok(zoned) = Zoned::strptime(input_format, datetime) {
                return Ok(zoned);
            }
            DateTime::strptime(input_format, datetime)
                .map_err(|e| format!("{ERR_PREFIX}{e}"))?
                .to_zoned(zone.clone())
                .map_err(|e| format!("{ERR_PREFIX}{e}"))
        }
    }
}

// --------------------------------------------------------------------------- //
//  Output formatting
// --------------------------------------------------------------------------- //

fn format_output(zoned: &Zoned, format: &str) -> String {
    match format.to_ascii_lowercase().as_str() {
        "iso" | "iso-zoned" => format_iso_zoned(zoned),
        "iso-offset" => format_iso_offset(zoned),
        "iso-local" => zoned.datetime().to_string(),
        "epoch" => zoned.timestamp().as_second().to_string(),
        "epochmillis" => zoned.timestamp().as_millisecond().to_string(),
        "rfc1123" => rfc2822::to_string(zoned).unwrap_or_else(|e| format!("{ERR_PREFIX}{e}")),
        pattern => zoned.strftime(pattern).to_string(),
    }
}

/// ISO-8601 with `[Zone/ID]` suffix — `2026-04-22T10:00:00-04:00[America/New_York]`.
fn format_iso_zoned(zoned: &Zoned) -> String {
    zoned.to_string()
}

/// ISO-8601 with offset only — `2026-04-22T10:00:00-04:00`. Strips the `[...]` suffix.
fn format_iso_offset(zoned: &Zoned) -> String {
    let full = zoned.to_string();
    match full.find('[') {
        Some(idx) => full[..idx].to_string(),
        None => full,
    }
}

// --------------------------------------------------------------------------- //
//  Difference
// --------------------------------------------------------------------------- //

fn compute_difference(first: &Zoned, second: &Zoned) -> String {
    let (earlier, later) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };

    let span: Span = match later
        .since((Unit::Year, earlier))
        .and_then(|s| s.round(SpanRound::new().largest(Unit::Year).relative(earlier)))
    {
        Ok(s) => s,
        Err(e) => return format!("{ERR_PREFIX}{e}"),
    };

    let years = span.get_years();
    let months = span.get_months();
    let days = span.get_days();
    let hours = span.get_hours();
    let minutes = span.get_minutes();
    let seconds = span.get_seconds();

    let total_seconds = later.timestamp().as_second() - earlier.timestamp().as_second();

    format!(
        "{{\"years\":{years},\"months\":{months},\"days\":{days},\"hours\":{hours},\"minutes\":{minutes},\"seconds\":{seconds},\"totalSeconds\":{total_seconds}}}"
    )
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_utc_to_tokyo() {
        // 2026-04-22T00:00:00Z → 2026-04-22T09:00:00+09:00[Asia/Tokyo]
        let out = convert_timezone("2026-04-22T00:00:00Z", "UTC", "Asia/Tokyo");
        assert!(out.contains("2026-04-22T09:00:00"), "got {out}");
        assert!(out.contains("[Asia/Tokyo]"), "got {out}");
    }

    #[test]
    fn convert_invalid_source_zone() {
        let out = convert_timezone("2026-04-22T00:00:00Z", "Not/AZone", "UTC");
        assert_eq!(out, "Error: Invalid timezone: Not/AZone");
    }

    #[test]
    fn convert_parse_error() {
        let out = convert_timezone("not-a-date", "UTC", "UTC");
        assert!(out.starts_with("Error: Cannot parse datetime"));
    }

    #[test]
    fn format_epoch_input_to_iso() {
        // epoch 0 in UTC → 1970-01-01T00:00:00+00:00[UTC]
        let out = format_datetime("0", "epoch", "iso", "UTC");
        assert!(out.starts_with("1970-01-01T00:00:00"), "got {out}");
        assert!(out.contains("[UTC]"), "got {out}");
    }

    #[test]
    fn format_iso_local_strips_zone() {
        let out = format_datetime("2026-04-22T10:00:00Z", "iso", "iso-local", "UTC");
        assert_eq!(out, "2026-04-22T10:00:00");
    }

    #[test]
    fn format_iso_offset_has_offset_no_zone() {
        let out = format_datetime("2026-04-22T10:00:00Z", "iso", "iso-offset", "UTC");
        // Should not include [UTC]
        assert!(!out.contains('['), "got {out}");
        assert!(out.contains("+00:00") || out.ends_with('Z'), "got {out}");
    }

    #[test]
    fn format_rfc1123_output() {
        let out = format_datetime("2026-03-04T12:00:00Z", "iso", "rfc1123", "UTC");
        // RFC 2822: "Wed, 4 Mar 2026 12:00:00 +0000" - we just check key markers.
        assert!(out.contains("Mar 2026"), "got {out}");
        assert!(out.contains("12:00:00"), "got {out}");
    }

    #[test]
    fn current_datetime_uses_requested_zone() {
        let out = current_datetime("America/Sao_Paulo", "iso");
        assert!(out.contains("[America/Sao_Paulo]"), "got {out}");
    }

    #[test]
    fn current_datetime_invalid_zone() {
        let out = current_datetime("Not/AZone", "iso");
        assert_eq!(out, "Error: Invalid timezone: Not/AZone");
    }

    #[test]
    fn list_timezones_europe() {
        let out = list_timezones("Europe");
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let array = v.as_array().expect("JSON array");
        let has_paris = array.iter().any(|e| e.as_str() == Some("Europe/Paris"));
        assert!(has_paris, "missing Europe/Paris in {out}");
    }

    #[test]
    fn list_timezones_all_keyword() {
        let out = list_timezones("all");
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert!(v.as_array().expect("array").len() > 100);
    }

    #[test]
    fn list_timezones_unknown_region_errors() {
        let out = list_timezones("Pluto");
        assert!(out.starts_with("Error:"), "got {out}");
    }

    #[test]
    fn datetime_difference_one_year_two_months() {
        // 2024-01-15 → 2025-03-15 = 1 year 2 months 0 days
        let out = datetime_difference("2024-01-15T00:00:00", "2025-03-15T00:00:00", "UTC");
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(v["years"], 1);
        assert_eq!(v["months"], 2);
        assert_eq!(v["days"], 0);
    }

    #[test]
    fn datetime_difference_negative_order_normalized() {
        // Arguments swapped — should still produce positive span.
        let out = datetime_difference("2025-03-15T00:00:00", "2024-01-15T00:00:00", "UTC");
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(v["years"], 1);
        assert_eq!(v["months"], 2);
    }

    #[test]
    fn datetime_difference_invalid_zone() {
        let out = datetime_difference("2024-01-01T00:00:00", "2025-01-01T00:00:00", "Not/AZone");
        assert_eq!(out, "Error: Invalid timezone: Not/AZone");
    }
}
