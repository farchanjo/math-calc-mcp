//! Hashing and encoding utilities.
//!
//! Covers `MD5`, `SHA-1`, `SHA-256`, `SHA-512`, `Base64`, `URL`, `hex`, and
//! `CRC32`. All implementations come from the pure-Rust `RustCrypto` suite, so
//! the zero-C-dependency invariant is preserved.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use md5::Digest;

use crate::mcp::message::{ErrorCode, Response, error_with_detail};

const TOOL_HASH_MD5: &str = "HASH_MD5";
const TOOL_HASH_SHA1: &str = "HASH_SHA1";
const TOOL_HASH_SHA256: &str = "HASH_SHA256";
const TOOL_HASH_SHA512: &str = "HASH_SHA512";
const TOOL_BASE64_ENCODE: &str = "BASE64_ENCODE";
const TOOL_BASE64_DECODE: &str = "BASE64_DECODE";
const TOOL_URL_ENCODE: &str = "URL_ENCODE";
const TOOL_URL_DECODE: &str = "URL_DECODE";
const TOOL_HEX_ENCODE: &str = "HEX_ENCODE";
const TOOL_CRC32: &str = "CRC32";

#[must_use]
pub fn hash_md5(input: &str) -> String {
    let digest = md5::Md5::digest(input.as_bytes());
    Response::ok(TOOL_HASH_MD5)
        .result(hex::encode(digest))
        .build()
}

#[must_use]
pub fn hash_sha1(input: &str) -> String {
    let digest = sha1::Sha1::digest(input.as_bytes());
    Response::ok(TOOL_HASH_SHA1)
        .result(hex::encode(digest))
        .build()
}

#[must_use]
pub fn hash_sha256(input: &str) -> String {
    let digest = sha2::Sha256::digest(input.as_bytes());
    Response::ok(TOOL_HASH_SHA256)
        .result(hex::encode(digest))
        .build()
}

#[must_use]
pub fn hash_sha512(input: &str) -> String {
    let digest = sha2::Sha512::digest(input.as_bytes());
    Response::ok(TOOL_HASH_SHA512)
        .result(hex::encode(digest))
        .build()
}

#[must_use]
pub fn base64_encode(input: &str) -> String {
    let encoded = BASE64_STANDARD.encode(input.as_bytes());
    Response::ok(TOOL_BASE64_ENCODE).result(encoded).build()
}

#[must_use]
pub fn base64_decode(input: &str) -> String {
    BASE64_STANDARD.decode(input.trim().as_bytes()).map_or_else(
        |e| {
            error_with_detail(
                TOOL_BASE64_DECODE,
                ErrorCode::ParseError,
                "input is not valid base64",
                &format!("error={e}"),
            )
        },
        |bytes| {
            String::from_utf8(bytes).map_or_else(
                |_| {
                    error_with_detail(
                        TOOL_BASE64_DECODE,
                        ErrorCode::InvalidInput,
                        "decoded bytes are not valid UTF-8",
                        "use the hex output if you need raw bytes",
                    )
                },
                |s| Response::ok(TOOL_BASE64_DECODE).result(s).build(),
            )
        },
    )
}

/// Return the byte offset of the first malformed percent-escape, or `None`
/// if every `%` in the input is followed by two ASCII hex digits. Used by
/// [`url_decode`] to reject input that the permissive `urlencoding` crate
/// would otherwise pass through unchanged.
fn find_invalid_percent_escape(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let first = bytes.get(i + 1);
            let second = bytes.get(i + 2);
            match (first, second) {
                (Some(a), Some(b)) if a.is_ascii_hexdigit() && b.is_ascii_hexdigit() => {
                    i += 3;
                    continue;
                }
                _ => return Some(i),
            }
        }
        i += 1;
    }
    None
}

#[must_use]
pub fn url_encode(input: &str) -> String {
    let encoded = urlencoding::encode(input);
    Response::ok(TOOL_URL_ENCODE)
        .result(encoded.into_owned())
        .build()
}

#[must_use]
pub fn url_decode(input: &str) -> String {
    // Validate percent-escapes before delegating: the `urlencoding` crate is
    // permissive and silently leaves malformed `%XY` triples in place, which
    // hides real input errors from the caller. RFC 3986 §2.1 requires two
    // hex digits; anything else is a parse error.
    if let Some(offset) = find_invalid_percent_escape(input) {
        return error_with_detail(
            TOOL_URL_DECODE,
            ErrorCode::ParseError,
            "invalid percent-escape in input",
            &format!("offset={offset}"),
        );
    }
    match urlencoding::decode(input) {
        Ok(decoded) => Response::ok(TOOL_URL_DECODE)
            .result(decoded.into_owned())
            .build(),
        Err(e) => error_with_detail(
            TOOL_URL_DECODE,
            ErrorCode::ParseError,
            "input is not valid percent-encoded UTF-8",
            &format!("error={e}"),
        ),
    }
}

#[must_use]
pub fn hex_encode(input: &str) -> String {
    Response::ok(TOOL_HEX_ENCODE)
        .result(hex::encode(input.as_bytes()))
        .build()
}

#[must_use]
pub fn crc32(input: &str) -> String {
    let value = crc32fast::hash(input.as_bytes());
    Response::ok(TOOL_CRC32)
        .field("DECIMAL", value.to_string())
        .field("HEX", format!("{value:08x}"))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_known_vector() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        assert!(hash_md5("").contains("d41d8cd98f00b204e9800998ecf8427e"));
        assert!(hash_md5("abc").contains("900150983cd24fb0d6963f7d28e17f72"));
    }

    #[test]
    fn sha1_known_vector() {
        // SHA1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        assert!(hash_sha1("abc").contains("a9993e364706816aba3e25717850c26c9cd0d89d"));
    }

    #[test]
    fn sha256_known_vector() {
        // SHA256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert!(
            hash_sha256("abc")
                .contains("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }

    #[test]
    fn sha512_known_vector() {
        // SHA512("abc") begins with ddaf35a193617aba...
        assert!(hash_sha512("abc").contains("ddaf35a193617aba"));
    }

    #[test]
    fn base64_round_trip() {
        let encoded = base64_encode("Hello, world!");
        assert!(encoded.contains("SGVsbG8sIHdvcmxkIQ=="));
        let decoded = base64_decode("SGVsbG8sIHdvcmxkIQ==");
        assert!(decoded.contains("Hello, world!"));
    }

    #[test]
    fn base64_decode_invalid_errors() {
        let out = base64_decode("not!valid$base64");
        assert!(out.starts_with("BASE64_DECODE: ERROR"));
    }

    #[test]
    fn url_encode_special_chars() {
        let out = url_encode("hello world!");
        assert!(out.contains("hello%20world%21"));
    }

    #[test]
    fn url_decode_round_trip() {
        let out = url_decode("hello%20world%21");
        assert!(out.contains("hello world!"));
    }

    #[test]
    fn url_decode_rejects_invalid_percent_escape() {
        // %ZZ is not a valid hex escape; must not silently pass through.
        let out = url_decode("hello%ZZ");
        assert!(out.starts_with("URL_DECODE: ERROR"));
        assert!(out.contains("invalid percent-escape"));
    }

    #[test]
    fn url_decode_rejects_trailing_percent() {
        let out = url_decode("incomplete%");
        assert!(out.starts_with("URL_DECODE: ERROR"));
    }

    #[test]
    fn url_decode_rejects_single_digit_percent() {
        let out = url_decode("mid%A");
        assert!(out.starts_with("URL_DECODE: ERROR"));
    }

    #[test]
    fn hex_encode_basic() {
        assert!(hex_encode("ABC").contains("414243"));
    }

    #[test]
    fn crc32_known_vector() {
        // CRC32("123456789") = 0xCBF43926
        let out = crc32("123456789");
        assert!(out.contains("HEX: cbf43926"), "got {out}");
    }

    #[test]
    fn empty_inputs_are_supported() {
        assert!(hash_md5("").contains("RESULT: "));
        assert!(base64_encode("").contains("RESULT: "));
        assert!(url_encode("").contains("RESULT: "));
        assert!(hex_encode("").contains("RESULT: "));
    }
}
