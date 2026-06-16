//! LZF decompression (liblzf / CLZF2 format).
//!
//! The **Kongregate / web build** wraps its save as `base64( LZF( base64(tree) ) )`
//! — the same inner `base64(tree)` the Steam build uses, but compressed with
//! Marc Lehmann's LZF (the C# `CLZF2` port the community save-editor uses)
//! instead of gzip. LZF has no header or length prefix: the stream is a series
//! of back-to-back chunks, each introduced by one control byte.
//!
//! - `ctrl < 32` — a **literal run**: copy the next `ctrl + 1` bytes verbatim.
//! - `ctrl >= 32` — a **back-reference**: length `ctrl >> 5` (with `7` meaning
//!   "read one more byte and add it"), and a backward distance built from the
//!   low 5 bits of `ctrl` (high 5 bits of the offset) plus the next byte. The
//!   copied run is `length + 2` bytes and may overlap the output cursor (so it
//!   is copied one byte at a time).
//!
//! Only decompression is implemented — we re-encode edited web saves in the
//! Steam (gzip) container, which both builds accept, so an LZF *compressor* is
//! not needed yet.

use anyhow::{Context, ensure};

/// Upper bound on the decompressed output — a corrupt/hostile stream can't be
/// inflated without bound. Matches the container's decompression cap.
const MAX_OUTPUT_LEN: usize = 64 * 1024 * 1024;

/// Decompress an LZF stream (no header / length prefix).
pub fn decompress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let ctrl = input[i] as usize;
        i += 1;
        if ctrl < 32 {
            // Literal run of ctrl + 1 bytes.
            let run = ctrl + 1;
            ensure!(i + run <= input.len(), "lzf: literal run overruns input");
            ensure!(
                out.len() + run <= MAX_OUTPUT_LEN,
                "lzf: output exceeds the {MAX_OUTPUT_LEN} byte cap"
            );
            out.extend_from_slice(&input[i..i + run]);
            i += run;
        } else {
            // Back-reference.
            let mut len = ctrl >> 5;
            if len == 7 {
                ensure!(i < input.len(), "lzf: truncated long-match length");
                len += input[i] as usize;
                i += 1;
            }
            ensure!(i < input.len(), "lzf: truncated back-reference offset");
            // distance = ((ctrl & 0x1f) << 8) + next_byte + 1, counted back
            // from the current output cursor.
            let distance = ((ctrl & 0x1f) << 8) + input[i] as usize + 1;
            i += 1;
            let mut ref_idx = out
                .len()
                .checked_sub(distance)
                .context("lzf: back-reference points before the start of output")?;
            let copy = len + 2;
            ensure!(
                out.len() + copy <= MAX_OUTPUT_LEN,
                "lzf: output exceeds the {MAX_OUTPUT_LEN} byte cap"
            );
            // Byte-by-byte so an overlapping (self-referential) run works.
            for _ in 0..copy {
                let b = out[ref_idx];
                out.push(b);
                ref_idx += 1;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_run_only() {
        // ctrl = 4 → copy the next 5 literal bytes.
        let stream = [4u8, b'H', b'e', b'l', b'l', b'o'];
        assert_eq!(decompress(&stream).unwrap(), b"Hello");
    }

    #[test]
    fn multiple_literal_runs() {
        // Two runs concatenate: "abc" then "de".
        let stream = [2u8, b'a', b'b', b'c', 1u8, b'd', b'e'];
        assert_eq!(decompress(&stream).unwrap(), b"abcde");
    }

    #[test]
    fn back_reference_repeats_earlier_bytes() {
        // Literal "ab", then a back-reference: ctrl=0x20 → len(ctrl>>5)=1, so
        // copy len+2 = 3 bytes; distance = ((0x20&0x1f)<<8) + 1 + 1 = 2, i.e.
        // start 2 bytes back ("ab"), copied 3 bytes with overlap → "aba".
        let stream = [1u8, b'a', b'b', 0x20u8, 1u8];
        assert_eq!(decompress(&stream).unwrap(), b"ababa");
    }

    #[test]
    fn long_match_reads_extra_length_byte() {
        // Literal "x", then a max-coded match (len nibble 7) with +3, distance 1
        // → copy 7+3+2 = 12 bytes from 1 back (all 'x').
        let stream = [0u8, b'x', (7u8 << 5), 3u8, 0u8];
        assert_eq!(decompress(&stream).unwrap(), b"xxxxxxxxxxxxx"); // 1 + 12
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert_eq!(decompress(&[]).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn rejects_truncated_and_underflowing_streams() {
        assert!(decompress(&[5u8, b'a']).is_err()); // literal run overruns
        assert!(decompress(&[0x20u8, 0u8]).is_err()); // back-ref before start
        assert!(decompress(&[(7u8 << 5)]).is_err()); // truncated long length
    }
}
