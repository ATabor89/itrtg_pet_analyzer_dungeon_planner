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

    // Only commit a struct when the text is `;`-terminated. The serializer ends
    // every struct field with `;`, so any genuine (game-produced) struct ends
    // with `;`; requiring it keeps `serialize(parse(x)) == x` faithful even for
    // contrived base64-wrapped values that decode to bare-key text without a
    // trailing `;` (e.g. `base64("a;b")`), which would otherwise gain a `;`.
    if text.ends_with(';') && tree::looks_like_struct(text) {
        return Raw::Struct(split_fields(text, depth));
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

    /// Mutable twin of [`peel`](Self::peel): unwrap any [`Raw::Base64`]
    /// wrappers so a nested struct can be navigated/mutated.
    fn peel_mut(&mut self) -> &mut Raw {
        match self {
            Raw::Base64(inner) => inner.peel_mut(),
            other => other,
        }
    }

    /// Follow a dotted path (peeling base64 wrappers at each level) to the
    /// value it names. A segment is a struct key, or — when the current node is
    /// a [`Raw::List`] — a list selector: a 0-based numeric index (`X.Q.17.b`),
    /// or `field=value` to pick the first element whose `field` scalar equals
    /// `value` (`X.b.a=Salamander.E`, `X.Q.a=117.b`). `None` if any segment is
    /// missing, the index/selector matches nothing, or a segment traverses a
    /// scalar.
    pub fn get_path(&self, path: &[&str]) -> Option<&Raw> {
        let mut node = self;
        for key in path {
            let peeled = node.peel();
            node = if let Some((field, val)) = key.split_once('=') {
                // A `field=value` selector resolves against a list *or* a
                // single-element list stored as a lone struct (a 1-element
                // `&`-list has no separator, so it re-parses as a struct —
                // list_or_single semantics).
                select_one(peeled, field, val)?
            } else {
                match peeled {
                    Raw::List(items) => items.get(list_index(items, key)?)?.peel(),
                    Raw::Struct(_) => peeled.get(key)?,
                    _ => return None,
                }
            };
        }
        Some(node)
    }

    /// Replace the scalar at a dotted path with `value`, returning the previous
    /// value's serialized form. Base64 wrappers are peeled at each level, so
    /// `["p", "j"]` reaches available god power inside the base64-wrapped `p`
    /// block. A segment is a struct key, or a list selector when the current
    /// node is a [`Raw::List`] — a 0-based index (`["X","Q","17","b"]`) or
    /// `field=value` to select by content (`["X","b","a=Salamander","E"]`).
    /// This is the editing primitive behind the `save-edit` tool.
    ///
    /// Errors (never panics) if the path is empty, a segment is missing, an
    /// index/selector matches nothing, a non-terminal segment is a scalar, the
    /// target is an empty field (`k:;` / `k;`), or the path stops on a list
    /// element rather than a scalar inside it. The value is written verbatim
    /// (the caller supplies invariant-culture text — integers, `True`/`False`).
    pub fn set_scalar_path(&mut self, path: &[&str], value: &str) -> anyhow::Result<String> {
        let (key, rest) = path
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("empty path"))?;
        // A `field=value` selector resolves against a list *or* a lone struct
        // (a 1-element list — see `get_path`).
        if let Some((field, val)) = key.split_once('=') {
            return match self.peel_mut() {
                Raw::List(items) => {
                    let idx = items
                        .iter()
                        .position(|e| matches!(e.get(field), Some(Raw::Scalar(s)) if s == val))
                        .ok_or_else(|| anyhow::anyhow!("no list element matches {key:?}"))?;
                    if rest.is_empty() {
                        anyhow::bail!("path ends on list element [{idx}]; name a field inside it");
                    }
                    items[idx].set_scalar_path(rest, value)
                }
                s @ Raw::Struct(_) => {
                    if !matches!(s.get(field), Some(Raw::Scalar(x)) if x == val) {
                        anyhow::bail!("no element matches selector {key:?}");
                    }
                    if rest.is_empty() {
                        anyhow::bail!("path ends on the matched element; name a field inside it");
                    }
                    s.set_scalar_path(rest, value)
                }
                _ => anyhow::bail!("selector {key:?} applied to a non-list/struct"),
            };
        }
        match self.peel_mut() {
            Raw::Struct(fields) => {
                let (_, field) = fields
                    .iter_mut()
                    .find(|(k, _)| k == key)
                    .ok_or_else(|| anyhow::anyhow!("key {key:?} not found"))?;
                let Field::Value(v) = field else {
                    anyhow::bail!("key {key:?} is an empty field");
                };
                if rest.is_empty() {
                    let prev = v.serialize();
                    *v = Raw::Scalar(value.to_string());
                    Ok(prev)
                } else {
                    v.set_scalar_path(rest, value)
                }
            }
            Raw::List(items) => {
                let idx = list_index(items, key).ok_or_else(|| {
                    if key.contains('=') {
                        anyhow::anyhow!("no list element matches selector {key:?}")
                    } else {
                        anyhow::anyhow!("list index {key:?} is not valid (len {})", items.len())
                    }
                })?;
                if rest.is_empty() {
                    anyhow::bail!(
                        "path ends on list element [{idx}]; name a field inside it, e.g. '{idx}.b'"
                    );
                }
                items[idx].set_scalar_path(rest, value)
            }
            _ => anyhow::bail!("path segment {key:?} does not name a struct or list"),
        }
    }
}

impl Raw {
    /// Mutable navigation to the node a dotted path names (struct keys + list
    /// index/`field=value` selectors, peeling base64). Unlike
    /// [`set_scalar_path`](Self::set_scalar_path) this returns the node itself,
    /// so callers can mutate a whole sub-tree — e.g. append to a list. An empty
    /// path returns `self` (peeled). `None` if any segment is missing.
    pub fn get_path_mut(&mut self, path: &[&str]) -> Option<&mut Raw> {
        let Some((key, rest)) = path.split_first() else {
            return Some(self.peel_mut());
        };
        match self.peel_mut() {
            Raw::Struct(fields) => match fields.iter_mut().find(|(k, _)| k == key)?.1 {
                Field::Value(ref mut v) => v.get_path_mut(rest),
                _ => None,
            },
            Raw::List(items) => {
                let idx = list_index(items, key)?;
                items[idx].get_path_mut(rest)
            }
            _ => None,
        }
    }
}

/// Resolve a `field=value` selector against a list, or against a single-element
/// list stored as a lone struct (list_or_single semantics). Returns the matched
/// element (peeled).
fn select_one<'a>(node: &'a Raw, field: &str, val: &str) -> Option<&'a Raw> {
    match node {
        Raw::List(items) => items
            .iter()
            .find(|e| matches!(e.get(field), Some(Raw::Scalar(s)) if s == val))
            .map(Raw::peel),
        Raw::Struct(_) => {
            matches!(node.get(field), Some(Raw::Scalar(s)) if s == val).then_some(node)
        }
        _ => None,
    }
}

/// Resolve a list-path segment to an element index. A bare number is a 0-based
/// index (bounds-checked); `field=value` selects the first element whose `field`
/// scalar equals `value`. `None` if nothing matches.
fn list_index(items: &[Raw], seg: &str) -> Option<usize> {
    if let Some((field, val)) = seg.split_once('=') {
        items
            .iter()
            .position(|el| matches!(el.get(field), Some(Raw::Scalar(s)) if s == val))
    } else {
        let i: usize = seg.parse().ok()?;
        (i < items.len()).then_some(i)
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

    #[test]
    fn set_scalar_path_descends_into_base64_struct() {
        // `p` is a base64-wrapped nested struct, like the real god-power block.
        let text = format!("a:1;p:{};", b64("j:1297;025:100;"));
        let mut r = parse(&text);
        // Set a nested scalar; the rest of the tree must be untouched.
        let prev = r.set_scalar_path(&["p", "j"], "999999").unwrap();
        assert_eq!(prev, "1297");
        // Round-trips, and re-reading the path yields the new value.
        let r2 = parse(&r.serialize());
        assert_eq!(r2.get_path(&["p", "j"]), Some(&Raw::Scalar("999999".into())));
        // Sibling and outer fields are unchanged.
        assert_eq!(r2.get_path(&["p", "025"]), Some(&Raw::Scalar("100".into())));
        assert_eq!(r2.get("a"), Some(&Raw::Scalar("1".into())));
    }

    #[test]
    fn set_scalar_path_errors_are_descriptive() {
        let mut r = parse(&format!("p:{};", b64("j:1;")));
        assert!(r.set_scalar_path(&["p", "missing"], "x").is_err());
        assert!(r.set_scalar_path(&["nope"], "x").is_err());
        assert!(r.set_scalar_path(&[], "x").is_err());
        // Descending through a scalar (not a struct) is an error, not a panic.
        assert!(r.set_scalar_path(&["p", "j", "deeper"], "x").is_err());
    }

    #[test]
    fn set_scalar_path_indexes_into_lists() {
        // `Q` is an &-joined list of two structs, like the material inventory.
        let list = format!("{}&{}", b64("a:24;b:104;"), b64("a:25;b:124;"));
        let mut r = parse(&format!("c:1;Q:{list};"));
        // Set element 1's `b` by index.
        let prev = r.set_scalar_path(&["Q", "1", "b"], "99").unwrap();
        assert_eq!(prev, "124");
        let r2 = parse(&r.serialize());
        assert_eq!(r2.get_path(&["Q", "1", "b"]), Some(&Raw::Scalar("99".into())));
        // Sibling element untouched.
        assert_eq!(r2.get_path(&["Q", "0", "b"]), Some(&Raw::Scalar("104".into())));
        // Out-of-range and non-numeric indices error rather than panic; a path
        // that stops on the element (not a field inside it) also errors.
        assert!(r.set_scalar_path(&["Q", "9", "b"], "1").is_err());
        assert!(r.set_scalar_path(&["Q", "x", "b"], "1").is_err());
        assert!(r.set_scalar_path(&["Q", "0"], "1").is_err());
    }

    #[test]
    fn list_selector_picks_element_by_field() {
        // `Q` elements carry an id `a` and a count `b`; select by `a=<id>`.
        let list = format!("{}&{}", b64("a:117;b:50;"), b64("a:159;b:99;"));
        let mut r = parse(&format!("Q:{list};"));
        // Read by selector.
        assert_eq!(r.get_path(&["Q", "a=159", "b"]), Some(&Raw::Scalar("99".into())));
        // Write by selector; the other element is untouched.
        let prev = r.set_scalar_path(&["Q", "a=117", "b"], "500").unwrap();
        assert_eq!(prev, "50");
        let r2 = parse(&r.serialize());
        assert_eq!(r2.get_path(&["Q", "a=117", "b"]), Some(&Raw::Scalar("500".into())));
        assert_eq!(r2.get_path(&["Q", "a=159", "b"]), Some(&Raw::Scalar("99".into())));
        // A selector that matches nothing errors, not panics.
        assert!(r.set_scalar_path(&["Q", "a=999", "b"], "1").is_err());
    }

    #[test]
    fn struct_with_leading_empty_key_parses_and_round_trips() {
        // The Pet Village Tavern serializes with a leading empty-valued key:
        // `a;b:4;c;d:5553;…` (and `x:10&11&26` whose value contains `&`).
        // Previously `looks_like_struct` required the first field to have a
        // colon, so the whole thing stayed a raw scalar.
        let tavern = "a;b:4;c;d:5553;x:10&11&26;";
        let r = parse(&format!("024:{};z:1;", b64(tavern)));
        assert_eq!(r.get_path(&["024", "b"]), Some(&Raw::Scalar("4".into())));
        assert_eq!(r.get_path(&["024", "d"]), Some(&Raw::Scalar("5553".into())));
        // The `&`-valued field stays a scalar (not split into a list).
        assert_eq!(r.get_path(&["024", "x"]), Some(&Raw::Scalar("10&11&26".into())));
        assert_round_trips(&format!("024:{};z:1;", b64(tavern)));
    }

    #[test]
    fn empty_struct_base64_is_decoded_not_left_raw() {
        // An empty building serializes to base64("a;") = "YTs=" (4 chars). The
        // old min-length-8 guard left it as a raw scalar; now it decodes.
        assert_eq!(b64("a;"), "YTs=");
        let r = parse("f:YTs=;g:1;");
        // `f` now decodes and peels to a struct (with the lone empty key `a`),
        // instead of being left as the raw scalar "YTs=".
        assert!(matches!(r.get_path(&["f"]).unwrap().peel(), Raw::Struct(_)));
        assert_round_trips("f:YTs=;g:1;");
    }

    #[test]
    fn base64_non_terminated_struct_text_stays_faithful() {
        // A base64-wrapped value decoding to bare-key text WITHOUT a trailing `;`
        // (e.g. `a;b`) must NOT be committed as a struct (which would re-serialize
        // as `a;b;`). The `;`-termination guard keeps it faithful.
        for inner in ["a;b", "ab;cd", "a:1;b:2"] {
            let s = format!("k:{};z:1;", b64(inner));
            assert_round_trips(&s);
        }
        // A `;`-terminated one is a real struct and still parses as such.
        let s = format!("k:{};z:1;", b64("a;b:4;"));
        assert!(matches!(parse(&s).get_path(&["k"]).unwrap().peel(), Raw::Struct(_)));
        assert_round_trips(&s);
    }

    #[test]
    fn selector_resolves_single_element_list() {
        // ONE base64 struct in a field is a 1-element list (no `&` separator),
        // which re-parses as a lone struct — the selector must still find it.
        let mut r = parse(&format!("Q:{};x:1;", b64("a:9990;b:777;")));
        assert_eq!(r.get_path(&["Q", "a=9990", "b"]), Some(&Raw::Scalar("777".into())));
        assert_eq!(r.set_scalar_path(&["Q", "a=9990", "b"], "5").unwrap(), "777");
        let r2 = parse(&r.serialize());
        assert_eq!(r2.get_path(&["Q", "a=9990", "b"]), Some(&Raw::Scalar("5".into())));
        // Non-matching selector on the lone struct errors, not panics.
        assert!(r.set_scalar_path(&["Q", "a=1", "b"], "1").is_err());
    }
}
