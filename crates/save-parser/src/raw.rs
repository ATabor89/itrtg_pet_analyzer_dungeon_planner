//! Lossless concrete syntax tree for the serialized save grammar.
//!
//! The analytic [`crate::tree::Node`] is deliberately heuristic and lossy: it
//! collapses both empty-field spellings — `k:;` (colon, then empty) and the
//! bare `k;` — to `Leaf("")`. Real saves use *both* (the god-power block `p`
//! writes `g:;`; the tracker block `x` writes `010;`), so a `Node` cannot be
//! serialized back to byte-identical plaintext. That is fine for the planner,
//! which only ever reads, but **re-serialization** (and the redaction built on
//! it) needs a faithful round-trip.
//!
//! [`Raw`] is that faithful representation. It parses with the exact same
//! base64-descent decisions as [`crate::tree::parse`] (reusing its helpers),
//! but records the empty-field form so [`Raw::serialize`] reproduces the
//! original plaintext exactly. Validated by the `tree_serialize_round_trips_*`
//! integration tests over the reference saves.
//!
//! Faithfulness rests on one more property: base64 is canonical, so decoding a
//! nested value and re-encoding our faithful inner serialization reproduces the
//! original base64 byte-for-byte. The round-trip tests are what confirm this
//! holds across every nested layer of the real saves.

use crate::tree;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

/// A value in the lossless tree.
///
/// The grammar stores a value in one of a few encodings, and faithful
/// re-serialization means recording which:
/// - a [`Raw::Scalar`] is verbatim text;
/// - a [`Raw::Struct`] is plain `key:value;` text (the root, or the decoded
///   contents of a [`Raw::Base64`]);
/// - a [`Raw::List`] is the raw `&`-joined base64 of its elements;
/// - a [`Raw::Base64`] is the base64 encoding of an inner value — the explicit
///   marker that distinguishes a base64-wrapped struct *or list* (which must be
///   re-encoded) from a list stored raw as a field value (which must not).
#[derive(Debug, Clone, PartialEq)]
pub enum Raw {
    /// A scalar value, kept as the exact (non-empty) text after the colon.
    Scalar(String),
    /// An ordered struct of `(key, field)` pairs.
    Struct(Vec<(String, Field)>),
    /// A `&`-joined list; each element is base64-encoded on serialization.
    List(Vec<Raw>),
    /// A value stored as the base64 encoding of the inner value.
    Base64(Box<Raw>),
}

/// The value side of a struct field, preserving how an empty field was spelled.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// `key:;` — a colon followed by an empty value.
    EmptyColon,
    /// `key;` — a bare key with no colon.
    EmptyBare,
    /// `key:value` — a non-empty value.
    Value(Raw),
}

/// Parse plaintext into the lossless [`Raw`] tree.
pub fn parse(text: &str) -> Raw {
    parse_value(text, 0)
}

fn parse_value(text: &str, depth: usize) -> Raw {
    if depth >= tree::MAX_DEPTH {
        return Raw::Scalar(text.to_string());
    }

    // `&`-joined list of base64 elements? (Mirrors `tree::parse_value`.)
    if text.contains('&') {
        let elems: Vec<&str> = text.split('&').collect();
        let decoded: Vec<Option<String>> =
            elems.iter().map(|e| tree::try_base64_text(e)).collect();
        if decoded.iter().all(|d| d.is_some()) {
            return Raw::List(
                decoded
                    .into_iter()
                    .map(|d| parse_value(&d.unwrap(), depth + 1))
                    .collect(),
            );
        }
    }

    // A single base64 blob that decodes to a struct or list? Record the
    // base64 boundary explicitly so it is re-encoded on the way out (a raw
    // `&`-list field value, by contrast, is *not* wrapped — see `List` above).
    if let Some(decoded) = tree::try_base64_text(text)
        && (tree::looks_like_struct(&decoded) || decoded.contains('&'))
    {
        return Raw::Base64(Box::new(parse_value(&decoded, depth + 1)));
    }

    if tree::looks_like_struct(text) {
        let fields = split_fields(text, depth);
        if fields.len() > 1 || text.ends_with(';') {
            return Raw::Struct(fields);
        }
    }

    Raw::Scalar(text.to_string())
}

/// Split a struct body into `(key, Field)` pairs, preserving empty-field form.
///
/// Two passes: the first recovers each field's raw spelling (including `;`-in-
/// value continuations, exactly as `tree::split_fields` does); the second
/// parses non-empty value strings into nested [`Raw`] values.
fn split_fields(text: &str, depth: usize) -> Vec<(String, Field)> {
    enum FormStr {
        EmptyColon,
        EmptyBare,
        Value(String),
    }

    let mut raw: Vec<(String, FormStr)> = Vec::new();
    for part in text.split(';') {
        if part.is_empty() {
            continue;
        }
        if let Some(klen) = tree::key_len(part) {
            let key = part[..klen].to_string();
            let val = &part[klen + 1..];
            if val.is_empty() {
                raw.push((key, FormStr::EmptyColon));
            } else {
                raw.push((key, FormStr::Value(val.to_string())));
            }
        } else if tree::is_bare_key(part) {
            raw.push((part.to_string(), FormStr::EmptyBare));
        } else if let Some(last) = raw.last_mut() {
            // A continuation of the previous value (a string that contained a
            // `;`). Re-attach the separator and segment. An empty field that
            // turns out to have had a continuation becomes a value of `;…`,
            // matching how `tree::split_fields` accumulates onto the prior "".
            match &mut last.1 {
                FormStr::Value(s) => {
                    s.push(';');
                    s.push_str(part);
                }
                FormStr::EmptyColon | FormStr::EmptyBare => {
                    last.1 = FormStr::Value(format!(";{part}"));
                }
            }
        }
        // A leading continuation with no preceding field is dropped, as in
        // `tree::split_fields`; `looks_like_struct` gates that out anyway.
    }

    raw.into_iter()
        .map(|(k, form)| {
            let field = match form {
                FormStr::EmptyColon => Field::EmptyColon,
                FormStr::EmptyBare => Field::EmptyBare,
                FormStr::Value(s) => Field::Value(parse_value(&s, depth + 1)),
            };
            (k, field)
        })
        .collect()
}

impl Raw {
    /// Serialize back to plaintext — the exact inverse of [`parse`].
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        self.serialize_into(&mut out);
        out
    }

    fn serialize_into(&self, out: &mut String) {
        match self {
            Raw::Scalar(s) => out.push_str(s),
            // The inner value, base64-encoded — restores the wrapper a nested
            // struct or a base64-wrapped list was stored behind.
            Raw::Base64(inner) => out.push_str(&B64.encode(inner.serialize())),
            Raw::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push('&');
                    }
                    out.push_str(&B64.encode(item.serialize()));
                }
            }
            Raw::Struct(fields) => {
                for (k, field) in fields {
                    out.push_str(k);
                    match field {
                        Field::EmptyColon => out.push_str(":;"),
                        Field::EmptyBare => out.push(';'),
                        // The value carries its own encoding (`Base64` wraps,
                        // `List`/`Scalar` are verbatim), so emit it directly.
                        Field::Value(v) => {
                            out.push(':');
                            v.serialize_into(out);
                            out.push(';');
                        }
                    }
                }
            }
        }
    }

    /// Peel any [`Raw::Base64`] wrappers, returning the inner value. A nested
    /// struct parses as `Base64(Struct(..))`; this exposes the struct so
    /// navigation does not have to know whether a value was base64-wrapped.
    pub fn peel(&self) -> &Raw {
        let mut node = self;
        while let Raw::Base64(inner) = node {
            node = inner;
        }
        node
    }

    /// Borrow a struct field's value by key (the first match), peeling any
    /// base64 wrapper. `None` if this is not a struct or the key is absent /
    /// empty-valued.
    pub fn get(&self, key: &str) -> Option<&Raw> {
        match self.peel() {
            Raw::Struct(fields) => fields
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, f)| match f {
                    Field::Value(v) => Some(v.peel()),
                    _ => None,
                }),
            _ => None,
        }
    }

    /// Replace a struct field's value with the given scalar text, returning
    /// the previous value's serialized form. Used by redaction to overwrite
    /// identity fields in place without disturbing field order or any other
    /// bytes.
    ///
    /// Returns `None` if this is not a struct, the key is absent, or the field
    /// is empty (`k:;` / `k;`) — i.e. only a non-empty value is replaced, and a
    /// `Some` return means a byte change actually happened.
    pub fn set_scalar(&mut self, key: &str, value: &str) -> Option<String> {
        let Raw::Struct(fields) = self else {
            return None;
        };
        let (_, field) = fields.iter_mut().find(|(k, _)| k == key)?;
        match field {
            Field::EmptyColon | Field::EmptyBare => None,
            Field::Value(v) => {
                let prev = v.serialize();
                *v = Raw::Scalar(value.to_string());
                Some(prev)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(s: &str) -> String {
        B64.encode(s.as_bytes())
    }

    /// `serialize(parse(text)) == text` for each grammar shape.
    fn assert_round_trips(text: &str) {
        assert_eq!(parse(text).serialize(), text, "round-trip mismatch");
    }

    #[test]
    fn round_trips_flat_struct() {
        assert_round_trips("a:1;b:True;c:Hello World;");
    }

    #[test]
    fn round_trips_both_empty_forms() {
        // The crux: colon-empty and bare-empty must survive distinctly. This
        // is the case the analytic `Node` cannot represent.
        assert_round_trips("a:1;g:;010;b:2;");
        let r = parse("a:1;g:;010;b:2;");
        assert_eq!(r.get("a").unwrap(), &Raw::Scalar("1".into()));
        match &r {
            Raw::Struct(f) => {
                assert_eq!(f[1], ("g".into(), Field::EmptyColon));
                assert_eq!(f[2], ("010".into(), Field::EmptyBare));
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn round_trips_numeric_and_bare_keys() {
        assert_round_trips("a:1;c;001:42;");
    }

    #[test]
    fn round_trips_nested_struct() {
        assert_round_trips(&format!("a:Salamander;w:{};", b64("a:4;b:101;")));
    }

    #[test]
    fn round_trips_list_of_structs() {
        assert_round_trips(&format!("x:{}&{};", b64("a:1;b:10;"), b64("a:2;b:20;")));
    }

    #[test]
    fn round_trips_scalar_with_colon() {
        assert_round_trips("a:1;g:Strongest Entity in the Universe?;");
    }

    #[test]
    fn round_trips_plain_int_list_leaf() {
        assert_round_trips("a:61&99&82&100;");
    }

    #[test]
    fn round_trips_scientific_notation() {
        assert_round_trips("a:66841.3595410302;b:7.37927073370121E+185;");
    }

    #[test]
    fn round_trips_non_ascii_nested() {
        assert_round_trips(&format!("b:{};", b64("a:Pigñata;E:24241;")));
    }

    #[test]
    fn round_trips_value_with_embedded_semicolon() {
        assert_round_trips("a:one;two three;b:2;");
    }

    #[test]
    fn set_scalar_replaces_value_in_place() {
        let mut r = parse("s:TestAccount;W:TestGod;c:1;");
        let prev = r.set_scalar("s", "REDACTED").unwrap();
        assert_eq!(prev, "TestAccount");
        assert_eq!(r.serialize(), "s:REDACTED;W:TestGod;c:1;");
        assert_eq!(r.set_scalar("missing", "x"), None);
    }
}
