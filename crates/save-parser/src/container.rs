//! Outer container encoding of an ITRTG save file.
//!
//! Two platform variants wrap the same inner `base64(tree)`:
//!
//! - **Steam** ([`ContainerFormat::SteamGzip`]): base64 text with 2 junk
//!   characters prepended (`V2`); the decoded bytes are `[0..4]` little-endian
//!   u32 uncompressed length then a gzip stream of the inner `base64(tree)`.
//! - **Kongregate / web** ([`ContainerFormat::KongregateLzf`]): base64 with no
//!   junk prefix; the decoded bytes are an LZF stream (no header) of the inner
//!   `base64(tree)`. See [`crate::lzf`].
//!
//! Decoding auto-detects the variant. Re-encoding emits the Steam container
//! (with the `V2` prefix), which **both** builds accept on import — so an edited
//! web save can be loaded back into either. (A faithful LZF re-encoder is not
//! implemented; it isn't needed for that round trip.)

use anyhow::{Context, bail, ensure};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use std::io::{Read, Write};

/// Which platform's container a save was decoded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerFormat {
    /// Steam: `V2` + base64(`[len][gzip(base64(tree))]`).
    SteamGzip,
    /// Kongregate / web: base64(`LZF(base64(tree))`).
    KongregateLzf,
}

/// Decode the outer container layers, yielding the serialized tree plaintext.
/// Upper bound on the decompressed payload. Real saves are ~300 KB of
/// plaintext; the cap just turns a corrupt/hostile length prefix or gzip
/// bomb into a clean error instead of a giant allocation.
const MAX_DECOMPRESSED_LEN: usize = 64 * 1024 * 1024;

/// The plaintext tree plus the leading junk characters that preceded the
/// base64 container (`V2` in every observed save). [`encode_container`] needs
/// the prefix to reproduce a loadable save, so [`decode_container`] preserves
/// it rather than discarding it the way [`decode_to_plaintext`] does.
#[derive(Debug, Clone)]
pub struct DecodedContainer {
    pub prefix: String,
    pub plaintext: String,
    /// The platform container this save was decoded from. Re-encoding always
    /// emits Steam (gzip), which both builds accept — this records the origin
    /// for reporting, not the re-encode target.
    pub format: ContainerFormat,
}

/// Decode the outer container, returning both the plaintext tree and the
/// leading junk characters (so the save can be faithfully re-encoded).
pub fn decode_container(raw: &str) -> anyhow::Result<DecodedContainer> {
    // Keep only ASCII non-whitespace: the container is ASCII base64, and
    // dropping anything else (e.g. a UTF-8 BOM from a text editor) both
    // tolerates re-saved files and keeps the byte-offset slicing below safe.
    let compact: String = raw
        .chars()
        .filter(|c| c.is_ascii() && !c.is_whitespace())
        .collect();

    // The known format prepends exactly 2 junk characters, but tolerate a
    // clean base64 blob (0) or other small offsets in case the prefix length
    // is not fixed. A candidate only counts if it base64-decodes *and* the
    // payload carries the gzip magic where we expect it.
    let mut found = None;
    for skip in [2usize, 0, 1, 3] {
        if skip >= compact.len() {
            continue;
        }
        if let Ok(decoded) = B64.decode(&compact[skip..])
            && decoded.len() > 6
            && decoded[4] == 0x1f
            && decoded[5] == 0x8b
        {
            found = Some((compact[..skip].to_string(), decoded));
            break;
        }
    }

    if let Some((prefix, bytes)) = found {
        let expected_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        ensure!(
            expected_len <= MAX_DECOMPRESSED_LEN,
            "length prefix {} exceeds the {} byte sanity cap",
            expected_len,
            MAX_DECOMPRESSED_LEN
        );

        // `take` one byte past the expected size so an over-long stream (lying
        // prefix, gzip bomb) fails the length check below instead of inflating
        // to completion.
        let mut decompressed = Vec::with_capacity(expected_len);
        flate2::read::GzDecoder::new(&bytes[4..])
            .take(expected_len as u64 + 1)
            .read_to_end(&mut decompressed)
            .context("gzip decompression failed")?;
        ensure!(
            decompressed.len() == expected_len,
            "length prefix {} does not match decompressed size {}",
            expected_len,
            decompressed.len()
        );

        let plaintext = inner_base64_to_tree(&decompressed)?;
        return Ok(DecodedContainer {
            prefix,
            plaintext,
            format: ContainerFormat::SteamGzip,
        });
    }

    // Not a Steam (gzip) container — try the Kongregate/web variant:
    // base64( LZF( base64(tree) ) ), with no junk prefix. The tree-shape guard
    // keeps trivial inputs (empty string, stray base64) from "decoding" to an
    // empty or junk tree.
    if let Ok(compressed) = B64.decode(&compact)
        && let Ok(inner) = crate::lzf::decompress(&compressed)
        && let Ok(plaintext) = inner_base64_to_tree(&inner)
        && plaintext.contains(':')
        && plaintext.contains(';')
    {
        // Re-encode as Steam (`V2`), which the web build also imports.
        return Ok(DecodedContainer {
            prefix: "V2".to_string(),
            plaintext,
            format: ContainerFormat::KongregateLzf,
        });
    }

    bail!("not a recognized save container (neither Steam gzip nor Kongregate LZF)");
}

/// Decode the innermost `base64(tree)` layer (shared by both container
/// variants) into the plaintext `key:value;` tree.
///
/// The web "Save to file" export concatenates two `base64(tree)` blobs (the
/// live save and a backup) separated by non-base64 bytes, so we decode only the
/// leading run of standard-base64 characters — which is exactly the first,
/// complete tree. Steam saves are a single clean blob, so this is a no-op for
/// them.
fn inner_base64_to_tree(inner: &[u8]) -> anyhow::Result<String> {
    let inner = std::str::from_utf8(inner)
        .context("decompressed payload is not valid UTF-8/ASCII base64")?
        .trim();
    let valid_len = inner
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='))
        .unwrap_or(inner.len());
    let b64 = &inner[..valid_len];
    let plain_bytes = B64
        .decode(b64)
        .context("inner base64 layer failed to decode")?;
    String::from_utf8(plain_bytes).context("plaintext tree is not valid UTF-8")
}

/// Re-encode a plaintext tree into the outer container format, prepending the
/// given junk `prefix` (use the one [`decode_container`] returned).
///
/// This is the exact inverse of the decode steps: base64 the plaintext, gzip
/// it, prepend a little-endian u32 length of the *base64* layer, base64 the
/// result, then prepend the junk prefix. The gzip layer is not guaranteed to
/// be byte-identical to the game's original (compressors differ), but the game
/// re-imports by decoding, not by byte comparison, so a semantically faithful
/// container loads correctly.
pub fn encode_container(plaintext: &str, prefix: &str) -> String {
    let inner = B64.encode(plaintext.as_bytes());
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(inner.as_bytes())
        .expect("writing to an in-memory gzip buffer cannot fail");
    let gzipped = gz
        .finish()
        .expect("finishing an in-memory gzip buffer cannot fail");
    let mut payload = (inner.len() as u32).to_le_bytes().to_vec();
    payload.extend_from_slice(&gzipped);
    format!("{prefix}{}", B64.encode(payload))
}

pub fn decode_to_plaintext(raw: &str) -> anyhow::Result<String> {
    Ok(decode_container(raw)?.plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-encode plaintext the way the game does, prepending `junk`.
    fn encode(plaintext: &str, junk: &str) -> String {
        encode_container(plaintext, junk)
    }

    #[test]
    fn decode_container_preserves_the_junk_prefix() {
        let text = "a:1;b:2;";
        let decoded = decode_container(&encode(text, "V2")).unwrap();
        assert_eq!(decoded.prefix, "V2");
        assert_eq!(decoded.plaintext, text);
    }

    #[test]
    fn container_round_trips_through_encode() {
        let text = "a:1;b:Hello;c:66841.3595410302;";
        let decoded = decode_container(&encode(text, "V2")).unwrap();
        let reencoded = encode_container(&decoded.plaintext, &decoded.prefix);
        assert_eq!(decode_to_plaintext(&reencoded).unwrap(), text);
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
    fn tolerates_utf8_bom_and_non_ascii() {
        let text = "a:1;b:2;";
        // A BOM is not whitespace — it must not panic the byte-offset slicing.
        let enc = format!("\u{feff}{}", encode(text, "V2"));
        assert_eq!(decode_to_plaintext(&enc).unwrap(), text);
    }

    #[test]
    fn rejects_oversized_length_prefix() {
        let text = "a:1;";
        let good = encode(text, "");
        let mut bytes = B64.decode(&good).unwrap();
        // Claim a payload over the sanity cap.
        bytes[0..4].copy_from_slice(&(u32::MAX).to_le_bytes());
        assert!(decode_to_plaintext(&B64.encode(&bytes)).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_to_plaintext("definitely not a save").is_err());
        assert!(decode_to_plaintext("").is_err());
    }

    #[test]
    fn decodes_kongregate_lzf_container() {
        // Build `base64( LZF( base64(tree) ) )` with a literal-only LZF stream
        // (chunks of ≤ 32 bytes, each `ctrl = len - 1`), the way the web build
        // wraps a save — no compressor needed for the literal-only case.
        let tree = "a:1;b:2;c:Hello World;";
        // The web export appends a separator + a second (backup) tree blob;
        // only the first, complete tree should come back.
        let backup = B64.encode("a:9;b:9;".as_bytes());
        let inner = format!("{}-77-{backup}", B64.encode(tree.as_bytes()));
        let mut lzf = Vec::new();
        for chunk in inner.as_bytes().chunks(32) {
            lzf.push((chunk.len() - 1) as u8);
            lzf.extend_from_slice(chunk);
        }
        let container = B64.encode(&lzf);

        let decoded = decode_container(&container).unwrap();
        assert_eq!(decoded.plaintext, tree);
        assert_eq!(decoded.format, ContainerFormat::KongregateLzf);
        // Decodes as Kongregate but re-encodes as Steam (`V2`), which both
        // builds import — round-trip back to the same tree.
        assert_eq!(decoded.prefix, "V2");
        let reencoded = encode_container(&decoded.plaintext, &decoded.prefix);
        let again = decode_container(&reencoded).unwrap();
        assert_eq!(again.plaintext, tree);
        assert_eq!(again.format, ContainerFormat::SteamGzip);
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
