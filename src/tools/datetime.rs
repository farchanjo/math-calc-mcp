//!
//! All public functions return `String` using the response envelope.
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

use crate::mcp::message::{ErrorCode, Response, error, error_with_detail};

const TOOL_CONVERT_TIMEZONE: &str = "CONVERT_TIMEZONE";
const TOOL_FORMAT_DATETIME: &str = "FORMAT_DATETIME";
const TOOL_CURRENT_DATE_TIME: &str = "CURRENT_DATE_TIME";
const TOOL_LIST_TIMEZONES: &str = "LIST_TIMEZONES";
const TOOL_DATETIME_DIFFERENCE: &str = "DATETIME_DIFFERENCE";

// --------------------------------------------------------------------------- //
//  Public API
// --------------------------------------------------------------------------- //

/// Convert a datetime string from one IANA timezone to another, returning ISO-zoned form.
#[must_use]
pub fn convert_timezone(datetime: &str, from_timezone: &str, to_timezone: &str) -> String {
    let from_zone = match resolve_zone(TOOL_CONVERT_TIMEZONE, from_timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let to_zone = match resolve_zone(TOOL_CONVERT_TIMEZONE, to_timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    match parse_datetime(TOOL_CONVERT_TIMEZONE, datetime, &from_zone) {
        Ok(source) => {
            let target = source.with_time_zone(to_zone);
            Response::ok(TOOL_CONVERT_TIMEZONE)
                .field("DATETIME", format_iso_zoned(&target))
                .field("FROM", from_timezone.to_string())
                .field("TO", to_timezone.to_string())
                .build()
        }
        Err(msg) => msg,
    }
}

/// Reformat a datetime string using explicit input/output format keywords or strftime patterns.
#[must_use]
pub fn format_datetime(
    datetime: &str,
    input_format: &str,
    output_format: &str,
    timezone: &str,
) -> String {
    let zone = match resolve_zone(TOOL_FORMAT_DATETIME, timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let parsed = match parse_with_format(TOOL_FORMAT_DATETIME, datetime, input_format, &zone) {
        Ok(zoned) => zoned,
        Err(msg) => return msg,
    };
    match format_output(TOOL_FORMAT_DATETIME, &parsed, output_format) {
        Ok(text) => Response::ok(TOOL_FORMAT_DATETIME).result(text).build(),
        Err(msg) => msg,
    }
}

/// Current datetime in the given IANA timezone, rendered using a format keyword or strftime pattern.
#[must_use]
pub fn current_datetime(timezone: &str, format: &str) -> String {
    let zone = match resolve_zone(TOOL_CURRENT_DATE_TIME, timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let now = Zoned::now().with_time_zone(zone);
    match format_output(TOOL_CURRENT_DATE_TIME, &now, format) {
        Ok(text) => Response::ok(TOOL_CURRENT_DATE_TIME).result(text).build(),
        Err(msg) => msg,
    }
}

/// List IANA timezone IDs, filtered by region prefix. Empty string or `"all"`
/// returns every zone. Output is a single `VALUES` field carrying a CSV.
#[must_use]
pub fn list_timezones(region: &str) -> String {
    let trimmed = region.trim();
    let region_label = if trimmed.is_empty() { "all" } else { trimmed };
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
        return error_with_detail(
            TOOL_LIST_TIMEZONES,
            ErrorCode::InvalidInput,
            "no timezones found for region",
            &format!("region={trimmed}"),
        );
    }

    matches.sort();
    Response::ok(TOOL_LIST_TIMEZONES)
        .field("REGION", region_label.to_string())
        .field("COUNT", matches.len().to_string())
        .field("VALUES", matches.join(","))
        .build()
}

/// Compute the positive difference between two datetimes parsed in `timezone`.
#[must_use]
pub fn datetime_difference(datetime1: &str, datetime2: &str, timezone: &str) -> String {
    let zone = match resolve_zone(TOOL_DATETIME_DIFFERENCE, timezone) {
        Ok(zone) => zone,
        Err(msg) => return msg,
    };
    let first = match parse_datetime(TOOL_DATETIME_DIFFERENCE, datetime1, &zone) {
        Ok(zoned) => zoned,
        Err(msg) => return msg,
    };
    let second = match parse_datetime(TOOL_DATETIME_DIFFERENCE, datetime2, &zone) {
        Ok(zoned) => zoned,
        Err(msg) => return msg,
    };
    compute_difference(&first, &second)
}

// --------------------------------------------------------------------------- //
//  Zone resolution
// --------------------------------------------------------------------------- //

fn resolve_zone(tool: &str, id: &str) -> Result<TimeZone, String> {
    TimeZone::get(id).map_err(|_| {
        error_with_detail(
            tool,
            ErrorCode::InvalidInput,
            "timezone is not a recognized IANA zone",
            &format!("timezone={id}"),
        )
    })
}

// --------------------------------------------------------------------------- //
//  Parsing
// --------------------------------------------------------------------------- //

fn datetime_parse_error(tool: &str, raw: &str) -> String {
    error_with_detail(
        tool,
        ErrorCode::ParseError,
        "cannot parse datetime",
        &format!("datetime={raw}"),
    )
}

const LOCALE_PATTERNS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S",
    "%d/%m/%Y %H:%M:%S",
    "%m/%d/%Y %H:%M:%S",
    "%d/%m/%Y",
    "%m/%d/%Y",
];

/// True when the string is a plausible Unix epoch count.
///
/// * A leading `-` is unambiguous — no ISO/locale format starts with `-`, so
///   any negative integer is treated as a pre-1970 epoch value.
/// * Unsigned values need at least 10 ASCII digits to pass, which keeps
///   short strings like `"2024"` (ISO year) or `"20241225"` (YYYYMMDD) on
///   the date-parser path.
fn looks_like_epoch(raw: &str) -> bool {
    if let Some(body) = raw.strip_prefix('-') {
        return !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit());
    }
    raw.len() >= 10 && raw.bytes().all(|b| b.is_ascii_digit())
}

fn parse_epoch_number(raw: &str, zone: &TimeZone) -> Option<Zoned> {
    // Distinguish seconds from milliseconds by magnitude rather than asking
    // the caller: epoch-seconds > 1e10 would already be year 2286+, so any
    // 13-digit non-negative value is almost certainly ms. Use the absolute
    // value so negative (pre-epoch) seconds work too.
    let abs_len = raw.strip_prefix('-').unwrap_or(raw).len();
    let as_millis = abs_len >= 13;
    let parsed: i64 = raw.parse().ok()?;
    let timestamp = if as_millis {
        Timestamp::from_millisecond(parsed).ok()?
    } else {
        Timestamp::from_second(parsed).ok()?
    };
    Some(timestamp.to_zoned(zone.clone()))
}

/// DST-strict civil-to-zoned conversion. Errors if the civil datetime falls
/// inside a spring-forward gap (does not exist in the zone) or an autumn-fold
/// overlap (is ambiguous between two offsets). Jiff's default
/// `civil.to_zoned(zone)` silently picks an offset via the `Compatible`
/// strategy, which hides real input bugs — e.g. `2020-03-08T02:30` in
/// `America/New_York` is an invented time yet was accepted.
fn civil_to_zoned_strict(
    tool: &str,
    civil: DateTime,
    zone: &TimeZone,
    raw: &str,
) -> Result<Zoned, String> {
    zone.to_ambiguous_zoned(civil).unambiguous().map_err(|_| {
        error_with_detail(
            tool,
            ErrorCode::InvalidInput,
            "datetime is ambiguous in this timezone (DST gap or fold)",
            &format!("datetime={raw}"),
        )
    })
}

/// Best-effort parse accepting ISO zoned/offset/local forms plus a few common locale patterns.
fn parse_datetime(tool: &str, datetime: &str, zone: &TimeZone) -> Result<Zoned, String> {
    if let Ok(zoned) = Zoned::from_str(datetime) {
        return Ok(zoned);
    }
    if let Ok(ts) = Timestamp::from_str(datetime) {
        return Ok(ts.to_zoned(zone.clone()));
    }
    // Bare epoch numbers ("1234567890", "1234567890123") — the docstring
    // advertises `epoch` as an accepted shape, but only `Timestamp::from_str`
    // was consulted, which rejects integer-only strings.
    if looks_like_epoch(datetime)
        && let Some(zoned) = parse_epoch_number(datetime, zone)
    {
        return Ok(zoned);
    }
    if let Ok(civil) = DateTime::from_str(datetime) {
        return civil_to_zoned_strict(tool, civil, zone, datetime);
    }

    for pattern in LOCALE_PATTERNS {
        if let Ok(civil) = DateTime::strptime(pattern, datetime) {
            return civil_to_zoned_strict(tool, civil, zone, datetime);
        }
    }

    if let Ok(date) = Date::from_str(datetime) {
        let civil = date.to_datetime(jiff::civil::Time::midnight());
        return civil_to_zoned_strict(tool, civil, zone, datetime);
    }

    Err(datetime_parse_error(tool, datetime))
}

fn parse_with_format(
    tool: &str,
    datetime: &str,
    input_format: &str,
    zone: &TimeZone,
) -> Result<Zoned, String> {
    // Same reasoning as `format_output`: match keywords case-insensitively
    // but pass the ORIGINAL pattern to strptime so `%Y`/`%H`/`%S` aren't
    // silently turned into their lowercase (and very different) variants.
    let keyword = input_format.to_ascii_lowercase();
    match keyword.as_str() {
        "iso" | "iso-zoned" | "iso-offset" | "iso-local" => parse_datetime(tool, datetime, zone),
        "epoch" => {
            let secs: i64 = datetime
                .trim()
                .parse()
                .map_err(|_| datetime_parse_error(tool, datetime))?;
            Timestamp::from_second(secs)
                .map(|ts| ts.to_zoned(zone.clone()))
                .map_err(|_| datetime_parse_error(tool, datetime))
        }
        "epochmillis" => {
            let millis: i64 = datetime
                .trim()
                .parse()
                .map_err(|_| datetime_parse_error(tool, datetime))?;
            Timestamp::from_millisecond(millis)
                .map(|ts| ts.to_zoned(zone.clone()))
                .map_err(|_| datetime_parse_error(tool, datetime))
        }
        "rfc1123" => {
            // Symmetric with the `rfc1123` output branch in `format_output`.
            // Without this case the input fell through to `strptime`, which
            // interpreted "rfc1123" as a literal pattern and always failed,
            // breaking the round-trip documented in the tool description.
            // `rfc2822::parse` returns a `Zoned` carrying the offset from
            // the input; converting to `zone` keeps the semantics of the
            // caller-supplied zone argument.
            rfc2822::parse(datetime)
                .map(|zoned| zoned.with_time_zone(zone.clone()))
                .map_err(|_| datetime_parse_error(tool, datetime))
        }
        _ => {
            if let Ok(zoned) = Zoned::strptime(input_format, datetime) {
                return Ok(zoned);
            }
            DateTime::strptime(input_format, datetime).map_or_else(
                |_| {
                    Err(error_with_detail(
                        tool,
                        ErrorCode::InvalidInput,
                        "format pattern rejected the datetime",
                        &format!("format={input_format}"),
                    ))
                },
                |civil| civil_to_zoned_strict(tool, civil, zone, datetime),
            )
        }
    }
}

// --------------------------------------------------------------------------- //
//  Output formatting
// --------------------------------------------------------------------------- //

fn format_output(tool: &str, zoned: &Zoned, format: &str) -> Result<String, String> {
    // Match keywords case-insensitively but NEVER pass the lowercased
    // pattern to `strftime` — `%Y`→`%y` (2-digit year), `%H`→`%h` (month
    // abbrev), `%S`→`%s` (epoch), etc. would all silently corrupt output.
    let keyword = format.to_ascii_lowercase();
    Ok(match keyword.as_str() {
        "iso" | "iso-zoned" => format_iso_zoned(zoned),
        "iso-offset" => format_iso_offset(zoned),
        "iso-local" => zoned.datetime().to_string(),
        "epoch" => zoned.timestamp().as_second().to_string(),
        "epochmillis" => zoned.timestamp().as_millisecond().to_string(),
        "rfc1123" => rfc2822::to_string(zoned).map_err(|_| {
            error_with_detail(
                tool,
                ErrorCode::InvalidInput,
                "format pattern rejected the datetime",
                "format=rfc1123",
            )
        })?,
        _ => {
            // Fallback is strftime; a pattern without any `%` tokens would
            // render as the literal text and silently lose the datetime.
            // Reject these explicitly so callers do not see their placeholder
            // echoed back as a "successful" result.
            if !format.contains('%') {
                return Err(error_with_detail(
                    tool,
                    ErrorCode::InvalidInput,
                    "output format is not a recognized keyword or strftime pattern",
                    &format!("format={format}"),
                ));
            }
            zoned.strftime(format).to_string()
        }
    })
}

fn format_iso_zoned(zoned: &Zoned) -> String {
    zoned.to_string()
}

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
        Err(e) => {
            return error(
                TOOL_DATETIME_DIFFERENCE,
                ErrorCode::InvalidInput,
                &e.to_string(),
            );
        }
    };

    let years = span.get_years();
    let months = span.get_months();
    let days = span.get_days();
    let hours = span.get_hours();
    let minutes = span.get_minutes();
    let seconds = span.get_seconds();
    let total_seconds = later.timestamp().as_second() - earlier.timestamp().as_second();

    Response::ok(TOOL_DATETIME_DIFFERENCE)
        .field("YEARS", years.to_string())
        .field("MONTHS", months.to_string())
        .field("DAYS", days.to_string())
        .field("HOURS", hours.to_string())
        .field("MINUTES", minutes.to_string())
        .field("SECONDS", seconds.to_string())
        .field("TOTAL_SECONDS", total_seconds.to_string())
        .build()
}

// --------------------------------------------------------------------------- //
//  Tests
// --------------------------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_utc_to_tokyo() {
        let out = convert_timezone("2026-04-22T00:00:00Z", "UTC", "Asia/Tokyo");
        assert!(
            out.starts_with("CONVERT_TIMEZONE: OK | DATETIME: "),
            "got {out}"
        );
        assert!(out.contains("2026-04-22T09:00:00"), "got {out}");
        assert!(out.contains("[Asia/Tokyo]"), "got {out}");
        assert!(out.contains("| FROM: UTC | TO: Asia/Tokyo"), "got {out}");
    }

    #[test]
    fn convert_invalid_source_zone() {
        assert_eq!(
            convert_timezone("2026-04-22T00:00:00Z", "Not/AZone", "UTC"),
            "CONVERT_TIMEZONE: ERROR\nREASON: [INVALID_INPUT] timezone is not a recognized IANA zone\nDETAIL: timezone=Not/AZone"
        );
    }

    #[test]
    fn convert_parse_error() {
        assert_eq!(
            convert_timezone("not-a-date", "UTC", "UTC"),
            "CONVERT_TIMEZONE: ERROR\nREASON: [PARSE_ERROR] cannot parse datetime\nDETAIL: datetime=not-a-date"
        );
    }

    #[test]
    fn convert_accepts_bare_epoch_seconds() {
        // `1234567890` is Unix epoch 2009-02-13 23:31:30 UTC.
        let out = convert_timezone("1234567890", "UTC", "UTC");
        assert!(out.starts_with("CONVERT_TIMEZONE: OK"), "got: {out}");
        assert!(out.contains("2009-02-13T23:31:30"), "got: {out}");
    }

    #[test]
    fn convert_accepts_bare_epoch_milliseconds() {
        // 13-digit values are disambiguated as epoch-ms.
        let out = convert_timezone("1234567890123", "UTC", "UTC");
        assert!(out.starts_with("CONVERT_TIMEZONE: OK"), "got: {out}");
        assert!(out.contains("2009-02-13T23:31:30.123"), "got: {out}");
    }

    #[test]
    fn convert_accepts_negative_epoch_seconds() {
        // Pre-1970: `-86400` is 1969-12-31 00:00:00 UTC.
        let out = convert_timezone("-86400", "UTC", "UTC");
        assert!(out.starts_with("CONVERT_TIMEZONE: OK"), "got: {out}");
        assert!(out.contains("1969-12-31T00:00:00"), "got: {out}");
    }

    #[test]
    fn convert_still_rejects_short_digits_as_dates() {
        // `"2024"` stays on the ISO-year path (fails because it's not a
        // full datetime); it must NOT be silently read as epoch 2024.
        let out = convert_timezone("2024", "UTC", "UTC");
        assert!(out.starts_with("CONVERT_TIMEZONE: ERROR"), "got: {out}");
    }

    #[test]
    fn format_epoch_input_to_iso() {
        let out = format_datetime("0", "epoch", "iso", "UTC");
        assert!(
            out.starts_with("FORMAT_DATETIME: OK | RESULT: 1970-01-01T00:00:00"),
            "got {out}"
        );
        assert!(out.contains("[UTC]"), "got {out}");
    }

    #[test]
    fn format_iso_local_strips_zone() {
        assert_eq!(
            format_datetime("2026-04-22T10:00:00Z", "iso", "iso-local", "UTC"),
            "FORMAT_DATETIME: OK | RESULT: 2026-04-22T10:00:00"
        );
    }

    #[test]
    fn format_iso_offset_has_offset_no_zone() {
        let out = format_datetime("2026-04-22T10:00:00Z", "iso", "iso-offset", "UTC");
        assert!(
            out.starts_with("FORMAT_DATETIME: OK | RESULT: "),
            "got {out}"
        );
        assert!(!out.contains('['), "got {out}");
        assert!(out.contains("+00:00") || out.contains('Z'), "got {out}");
    }

    #[test]
    fn format_rfc1123_output() {
        let out = format_datetime("2026-03-04T12:00:00Z", "iso", "rfc1123", "UTC");
        assert!(
            out.starts_with("FORMAT_DATETIME: OK | RESULT: "),
            "got {out}"
        );
        assert!(out.contains("Mar 2026"), "got {out}");
        assert!(out.contains("12:00:00"), "got {out}");
    }

    #[test]
    fn format_rfc1123_input_roundtrips() {
        // Regression: the `rfc1123` keyword was documented as an input
        // format and recognised as an output format, but the parse path
        // fell through to `strptime("rfc1123", …)` and always failed —
        // the round-trip was impossible to close. Both GMT/`+0000` forms
        // of an RFC 1123 timestamp must parse.
        let out = format_datetime("Wed, 22 Apr 2026 15:30:00 +0000", "rfc1123", "iso", "UTC");
        assert!(out.starts_with("FORMAT_DATETIME: OK"), "got {out}");
        assert!(out.contains("2026-04-22T15:30:00"), "got {out}");

        let out_gmt = format_datetime("Wed, 22 Apr 2026 15:30:00 GMT", "rfc1123", "iso", "UTC");
        assert!(out_gmt.starts_with("FORMAT_DATETIME: OK"), "got {out_gmt}");

        // End-to-end round-trip: ISO → RFC1123 → ISO returns the original.
        let rfc = format_datetime("2026-03-04T12:00:00Z", "iso", "rfc1123", "UTC");
        let value = rfc
            .strip_prefix("FORMAT_DATETIME: OK | RESULT: ")
            .and_then(|rest| rest.split(" | ").next())
            .unwrap_or_else(|| panic!("could not extract RFC1123 value from {rfc}"));
        let back = format_datetime(value, "rfc1123", "iso", "UTC");
        assert!(back.contains("2026-03-04T12:00:00"), "got {back}");
    }

    #[test]
    fn format_strftime_uppercase_conversions_preserved() {
        // Regression: `format.to_ascii_lowercase()` used to mangle the
        // strftime pattern before forwarding it to jiff, turning %Y into
        // %y (2-digit), %H into %h (month abbrev), %S into %s (epoch), etc.
        assert_eq!(
            format_datetime("1000000000", "epoch", "%Y-%m-%d", "UTC"),
            "FORMAT_DATETIME: OK | RESULT: 2001-09-09"
        );
        assert_eq!(
            format_datetime("1000000000", "epoch", "%H:%M:%S", "UTC"),
            "FORMAT_DATETIME: OK | RESULT: 01:46:40"
        );
        assert_eq!(
            format_datetime("1000000000", "epoch", "%d/%m/%Y %H:%M", "UTC"),
            "FORMAT_DATETIME: OK | RESULT: 09/09/2001 01:46"
        );
        let full_names = format_datetime("1000000000", "epoch", "%A, %B %d, %Y", "UTC");
        assert!(
            full_names.contains("Sunday") && full_names.contains("September"),
            "got {full_names}"
        );
    }

    #[test]
    fn current_datetime_strftime_year_is_four_digits() {
        // Regression: `%Y` used to be lowercased to `%y` in the tool layer.
        let out = current_datetime("UTC", "%Y");
        let year_section = out.rsplit_once("| RESULT: ").expect("has RESULT").1.trim();
        assert_eq!(year_section.len(), 4, "expected 4-digit year, got {out}");
    }

    #[test]
    fn current_datetime_uses_requested_zone() {
        let out = current_datetime("America/Sao_Paulo", "iso");
        assert!(
            out.starts_with("CURRENT_DATE_TIME: OK | RESULT: "),
            "got {out}"
        );
        assert!(out.contains("[America/Sao_Paulo]"), "got {out}");
    }

    #[test]
    fn current_datetime_invalid_zone() {
        assert_eq!(
            current_datetime("Not/AZone", "iso"),
            "CURRENT_DATE_TIME: ERROR\nREASON: [INVALID_INPUT] timezone is not a recognized IANA zone\nDETAIL: timezone=Not/AZone"
        );
    }

    #[test]
    fn list_timezones_europe() {
        let out = list_timezones("Europe");
        assert!(
            out.starts_with("LIST_TIMEZONES: OK | REGION: Europe | COUNT: "),
            "got {out}"
        );
        assert!(
            out.contains("Europe/Paris"),
            "missing Europe/Paris in {out}"
        );
        assert!(out.contains("| VALUES: "), "got {out}");
    }

    #[test]
    fn list_timezones_all_keyword() {
        let out = list_timezones("all");
        assert!(
            out.starts_with("LIST_TIMEZONES: OK | REGION: all | COUNT: "),
            "got {out}"
        );
        // Sanity check: at least hundreds of zones in the CSV.
        let csv_start = out.find("| VALUES: ").expect("VALUES segment present");
        let csv = &out[csv_start + "| VALUES: ".len()..];
        assert!(csv.split(',').count() > 100, "only a few zones: {out}");
    }

    #[test]
    fn list_timezones_unknown_region_errors() {
        assert_eq!(
            list_timezones("Pluto"),
            "LIST_TIMEZONES: ERROR\nREASON: [INVALID_INPUT] no timezones found for region\nDETAIL: region=Pluto"
        );
    }

    #[test]
    fn datetime_difference_one_year_two_months() {
        let out = datetime_difference("2024-01-15T00:00:00", "2025-03-15T00:00:00", "UTC");
        assert!(
            out.starts_with("DATETIME_DIFFERENCE: OK | YEARS: 1 | MONTHS: 2 | DAYS: 0"),
            "got {out}"
        );
        assert!(out.contains("| TOTAL_SECONDS: "), "got {out}");
    }

    #[test]
    fn datetime_difference_negative_order_normalized() {
        let out = datetime_difference("2025-03-15T00:00:00", "2024-01-15T00:00:00", "UTC");
        assert!(
            out.starts_with("DATETIME_DIFFERENCE: OK | YEARS: 1 | MONTHS: 2 | DAYS: 0"),
            "got {out}"
        );
    }

    #[test]
    fn datetime_difference_invalid_zone() {
        assert_eq!(
            datetime_difference("2024-01-01T00:00:00", "2025-01-01T00:00:00", "Not/AZone"),
            "DATETIME_DIFFERENCE: ERROR\nREASON: [INVALID_INPUT] timezone is not a recognized IANA zone\nDETAIL: timezone=Not/AZone"
        );
    }

    #[test]
    fn format_datetime_rejects_placeholder_without_tokens() {
        // Regression: an output format without any `%` token used to be echoed
        // as the literal text (e.g. outputFormat="invalid_format" returned
        // RESULT: invalid_format). Should fail with INVALID_INPUT instead.
        let out = format_datetime("2026-04-22T10:30:00Z", "iso", "invalid_format", "UTC");
        assert!(out.contains("FORMAT_DATETIME: ERROR"), "got {out}");
        assert!(out.contains("INVALID_INPUT"), "got {out}");
        assert!(
            out.contains("output format is not a recognized keyword or strftime pattern"),
            "got {out}"
        );
        assert!(out.contains("format=invalid_format"), "got {out}");
    }

    #[test]
    fn format_datetime_rejects_empty_format() {
        let out = format_datetime("2026-04-22T10:30:00Z", "iso", "", "UTC");
        assert!(out.contains("FORMAT_DATETIME: ERROR"), "got {out}");
        assert!(out.contains("INVALID_INPUT"), "got {out}");
    }

    #[test]
    fn format_datetime_accepts_strftime_with_tokens() {
        // Must still accept strftime patterns that contain `%` tokens.
        let out = format_datetime("2026-04-22T10:30:00Z", "iso", "%Y-%m-%d", "UTC");
        assert_eq!(out, "FORMAT_DATETIME: OK | RESULT: 2026-04-22");
    }

    #[test]
    fn convert_rejects_dst_gap_in_spring_forward() {
        // 2020-03-08T02:30 does not exist in America/New_York (clocks skip
        // from 02:00 to 03:00). Jiff's default `.to_zoned()` silently picks
        // the later offset; we surface the ambiguity so callers can catch
        // the bad input.
        let out = convert_timezone("2020-03-08T02:30:00", "America/New_York", "UTC");
        assert!(out.contains("CONVERT_TIMEZONE: ERROR"), "got {out}");
        assert!(out.contains("DST gap or fold"), "got {out}");
    }

    #[test]
    fn convert_rejects_dst_fold_in_autumn_back() {
        // 2024-11-03T01:30 America/New_York is ambiguous (occurs at both
        // -04:00 and -05:00). Must be flagged.
        let out = convert_timezone("2024-11-03T01:30:00", "America/New_York", "UTC");
        assert!(out.contains("CONVERT_TIMEZONE: ERROR"), "got {out}");
        assert!(out.contains("DST gap or fold"), "got {out}");
    }

    #[test]
    fn convert_accepts_unambiguous_local_time() {
        // A well-defined civil datetime must still round-trip.
        let out = convert_timezone("2024-06-15T12:00:00", "America/New_York", "UTC");
        assert!(out.starts_with("CONVERT_TIMEZONE: OK"), "got {out}");
    }
}
