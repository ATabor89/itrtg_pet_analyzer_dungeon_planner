//! Outer container encoding of an ITRTG save file.
//!
//! Layout (outer → inner):
//! 1. Base64 text with 2 extra characters prepended (purpose unknown — the
//!    one sample starts with `V2`). We try a few strip offsets so a format
//!    variation doesn't hard-break us.
//! 2. Decoded bytes: `[0..4]` little-endian u32 = uncompressed length,
//!    `[4..]` = gzip stream.
//! 3. Gunzipped bytes are ASCII base64 once more.
//! 4. Decoding that yields the plaintext `key:value;` tree.

use anyhow::{Context, bail, ensure};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use std::io::Read;

/// Decode the outer container layers, yielding the serialized tree plaintext.
pub fn decode_to_plaintext(raw: &str) -> anyhow::Result<String> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();

    // The known format prepends exactly 2 junk characters, but tolerate a
    // clean base64 blob (0) or other small offsets in case the prefix length
    // is not fixed. A candidate only counts if it base64-decodes *and* the
    // payload carries the gzip magic where we expect it.
    let mut bytes = None;
    for skip in [2usize, 0, 1, 3] {
        if skip >= compact.len() {
            continue;
        }
        if let Ok(decoded) = B64.decode(&compact[skip..])
            && decoded.len() > 6
            && decoded[4] == 0x1f
            && decoded[5] == 0x8b
        {
            bytes = Some(decoded);
            break;
        }
    }
    let Some(bytes) = bytes else {
        bail!("not a recognized save container (no length-prefixed gzip payload found)");
    };

    let expected_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;

    let mut decompressed = Vec::with_capacity(expected_len);
    flate2::read::GzDecoder::new(&bytes[4..])
        .read_to_end(&mut decompressed)
        .context("gzip decompression failed")?;
    ensure!(
        decompressed.len() == expected_len,
        "length prefix {} does not match decompressed size {}",
        expected_len,
        decompressed.len()
    );

    let inner = std::str::from_utf8(&decompressed)
        .context("decompressed payload is not valid UTF-8/ASCII base64")?;
    let plain_bytes = B64
        .decode(inner.trim())
        .context("inner base64 layer failed to decode")?;
    String::from_utf8(plain_bytes).context("plaintext tree is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Re-encode plaintext the way the game does, prepending `junk`.
    fn encode(plaintext: &str, junk: &str) -> String {
        let inner = B64.encode(plaintext.as_bytes());
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(inner.as_bytes()).unwrap();
        let gzipped = gz.finish().unwrap();
        let mut payload = (inner.len() as u32).to_le_bytes().to_vec();
        payload.extend_from_slice(&gzipped);
        format!("{junk}{}", B64.encode(payload))
    }

    #[test]
    fn round_trips_with_two_junk_chars() {
        let text = "a:1;b:Hello;";
        assert_eq!(decode_to_plaintext(&encode(text, "V2")).unwrap(), text);
    }

    #[test]
    fn round_trips_without_junk_chars() {
        let text = "a:1;";
        assert_eq!(decode_to_plaintext(&encode(text, "")).unwrap(), text);
    }

    #[test]
    fn tolerates_whitespace_and_newlines() {
        let text = "a:1;b:2;";
        let mut enc = encode(text, "V2");
        enc.insert(10, '\n');
        enc.insert(4, ' ');
        assert_eq!(decode_to_plaintext(&enc).unwrap(), text);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_to_plaintext("definitely not a save").is_err());
        assert!(decode_to_plaintext("").is_err());
    }

    #[test]
    fn rejects_wrong_length_prefix() {
        let text = "a:1;";
        let good = encode(text, "");
        // Corrupt the length prefix (first 4 bytes of the decoded payload).
        let mut bytes = B64.decode(&good).unwrap();
        bytes[0] ^= 0xff;
        assert!(decode_to_plaintext(&B64.encode(&bytes)).is_err());
    }
}
