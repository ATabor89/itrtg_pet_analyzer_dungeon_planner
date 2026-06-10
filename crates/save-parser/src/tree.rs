//! Generic parser for the serialized `key:value;` tree inside a save.
//!
//! Grammar (see `reference/save_file_deserialization/FINDINGS.md`):
//! - A struct is `key:value;key:value;...`. Keys are 1–4 ASCII alphanumerics
//!   (`a`..`z`, `A`..`Z`, then zero-padded numerics like `001` for fields
//!   added in later game versions).
//! - A key followed by `;` with no colon (e.g. the `h` in `g:0;h;i:1;`) is a
//!   field with an empty value.
//! - Nested structs are base64-encoded and stored as the value.
//! - Lists are `&`-joined base64 elements.
//! - Scalars: invariant-culture numbers (possibly scientific notation),
//!   `True`/`False`, or plain strings.
//!
//! The grammar is ambiguous at the edges (a string value could in principle
//! look like base64), so the parser is heuristic by design: it never fails,
//! and anything it cannot confidently interpret stays a [`Node::Leaf`] with
//! the raw text.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

/// One node of the parsed save tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A scalar value (number, bool, string), kept as raw text.
    Leaf(String),
    /// A struct: ordered `(key, value)` pairs.
    Struct(Vec<(String, Node)>),
    /// A `&`-joined list.
    List(Vec<Node>),
}

/// Maximum recursion depth (nested base64 layers). The real save nests ~6
/// deep; this is purely a safety net against pathological input.
const MAX_DEPTH: usize = 32;

/// Parse serialized tree plaintext into a [`Node`].
pub fn parse(text: &str) -> Node {
    parse_value(text, 0)
}

fn parse_value(text: &str, depth: usize) -> Node {
    if depth >= MAX_DEPTH {
        return Node::Leaf(text.to_string());
    }

    // `&`-joined list of base64 elements? (Plain `&`-joined number lists like
    // `61&99&82` stay leaves — their elements don't pass the base64 check.)
    if text.contains('&') {
        let elems: Vec<&str> = text.split('&').collect();
        let decoded: Vec<Option<String>> = elems.iter().map(|e| try_base64_text(e)).collect();
        if decoded.iter().all(|d| d.is_some()) {
            return Node::List(
                decoded
                    .into_iter()
                    .map(|d| parse_value(&d.unwrap(), depth + 1))
                    .collect(),
            );
        }
    }

    // A single base64 blob that decodes to a struct or list?
    if let Some(decoded) = try_base64_text(text)
        && (looks_like_struct(&decoded) || decoded.contains('&'))
    {
        return parse_value(&decoded, depth + 1);
    }

    if looks_like_struct(text) {
        let fields = split_fields(text);
        // A single field with no trailing separator is more likely a scalar
        // containing a colon (e.g. a time string) than a one-field struct.
        if fields.len() > 1 || text.ends_with(';') {
            return Node::Struct(
                fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), parse_value(&v, depth + 1)))
                    .collect(),
            );
        }
    }

    Node::Leaf(text.to_string())
}

/// Does `part` start with a `key:`? Returns the key length if so.
fn key_len(part: &str) -> Option<usize> {
    let colon = part.find(':')?;
    if (1..=4).contains(&colon) && part[..colon].bytes().all(|b| b.is_ascii_alphanumeric()) {
        Some(colon)
    } else {
        None
    }
}

fn is_bare_key(part: &str) -> bool {
    (1..=4).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn looks_like_struct(text: &str) -> bool {
    // First `;`-segment must start with `key:`.
    let first = text.split(';').next().unwrap_or("");
    key_len(first).is_some()
}

/// Split a struct body into `(key, value)` pairs.
///
/// Split on `;`; a part starting with `key:` opens a new field, a bare
/// 1–4 char alphanumeric part is an empty-valued field, and anything else is
/// a continuation of the previous value (a string that contained `;`).
fn split_fields(text: &str) -> Vec<(&str, String)> {
    let mut fields: Vec<(&str, String)> = Vec::new();
    for part in text.split(';') {
        if part.is_empty() {
            continue;
        }
        if let Some(klen) = key_len(part) {
            fields.push((&part[..klen], part[klen + 1..].to_string()));
        } else if is_bare_key(part) {
            fields.push((part, String::new()));
        } else if let Some(last) = fields.last_mut() {
            last.1.push(';');
            last.1.push_str(part);
        }
        // A continuation with no preceding field is dropped; it cannot occur
        // when `looks_like_struct` gated entry.
    }
    fields
}

/// Decode `s` as base64 if it plausibly is a nested payload: minimum length,
/// strict charset, valid UTF-8, and no control characters.
fn try_base64_text(s: &str) -> Option<String> {
    if s.len() < 8 || !s.len().is_multiple_of(4) {
        return None;
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
    {
        return None;
    }
    let bytes = B64.decode(s).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    if text
        .chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r')
    {
        return None;
    }
    Some(text)
}

impl Node {
    /// Field lookup on a struct node (exact, case-sensitive key).
    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Struct(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Chained [`Node::get`] through several struct levels.
    pub fn get_path(&self, keys: &[&str]) -> Option<&Node> {
        keys.iter().try_fold(self, |node, key| node.get(key))
    }

    /// List elements, or an empty slice for non-lists.
    pub fn items(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            _ => &[],
        }
    }

    /// Like [`Node::items`], but treats a lone struct as a one-element list.
    /// A serialized single-element list has no `&` separator, so it parses as
    /// a plain struct — callers reading list-typed save fields want both.
    pub fn list_or_single(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            Node::Struct(_) => std::slice::from_ref(self),
            Node::Leaf(_) => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Leaf(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_str()?.parse().ok()
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_str()?.parse().ok()
    }

    pub fn as_u32(&self) -> Option<u32> {
        self.as_str()?.parse().ok()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_str()?.parse().ok()
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.as_str()? {
            "True" => Some(true),
            "False" => Some(false),
            _ => None,
        }
    }

    /// Parse a leaf like `61&99&82` (or a single `61`) as an integer list.
    /// All-or-nothing: one unparseable element yields `None` rather than a
    /// partial list, so callers never see a silently shortened team.
    pub fn as_int_list(&self) -> Option<Vec<u32>> {
        self.as_str()?
            .split('&')
            .map(|p| p.parse().ok())
            .collect()
    }

    /// Render the tree in the indented `key = value` format used by the
    /// exploration dumps in `reference/save_file_deserialization/`.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        self.dump_into(&mut out, "root", 0);
        out
    }

    fn dump_into(&self, out: &mut String, label: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        match self {
            Node::Leaf(s) => {
                out.push_str(&format!("{indent}{label} = {s}\n"));
            }
            Node::Struct(fields) => {
                out.push_str(&format!("{indent}{label} =\n"));
                for (k, v) in fields {
                    v.dump_into(out, k, depth + 1);
                }
            }
            Node::List(items) => {
                out.push_str(&format!("{indent}{label} = <list of {}>\n", items.len()));
                for (i, item) in items.iter().enumerate() {
                    item.dump_into(out, &format!("[{i}]"), depth + 1);
                }
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

    #[test]
    fn parses_flat_struct() {
        let node = parse("a:1;b:True;c:Hello World;");
        assert_eq!(node.get("a").unwrap().as_i64(), Some(1));
        assert_eq!(node.get("b").unwrap().as_bool(), Some(true));
        assert_eq!(node.get("c").unwrap().as_str(), Some("Hello World"));
    }

    #[test]
    fn parses_numeric_keys_and_bare_keys() {
        let node = parse("a:1;c;001:42;");
        assert_eq!(node.get("c").unwrap().as_str(), Some(""));
        assert_eq!(node.get("001").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn parses_nested_struct() {
        let inner = "a:4;b:101;";
        let text = format!("a:Salamander;w:{};", b64(inner));
        let node = parse(&text);
        assert_eq!(node.get_path(&["w", "b"]).unwrap().as_i64(), Some(101));
    }

    #[test]
    fn parses_list_of_structs() {
        let text = format!("{}&{}", b64("a:1;b:10;"), b64("a:2;b:20;"));
        let node = parse(&text);
        assert_eq!(node.items().len(), 2);
        assert_eq!(node.items()[1].get("b").unwrap().as_i64(), Some(20));
    }

    #[test]
    fn plain_int_list_stays_leaf() {
        // Short numeric elements must not be mistaken for base64.
        let node = parse("a:61&99&82&100;");
        let ids = node.get("a").unwrap().as_int_list().unwrap();
        assert_eq!(ids, vec![61, 99, 82, 100]);
    }

    #[test]
    fn scalar_with_colon_is_not_a_struct() {
        let node = parse("a:1;g:Strongest Entity in the Universe?;");
        assert_eq!(
            node.get("g").unwrap().as_str(),
            Some("Strongest Entity in the Universe?")
        );
    }

    #[test]
    fn value_containing_semicolon_joins_previous_field() {
        let node = parse("a:one;two three;b:2;");
        assert_eq!(node.get("a").unwrap().as_str(), Some("one;two three"));
        assert_eq!(node.get("b").unwrap().as_i64(), Some(2));
    }

    #[test]
    fn non_ascii_text_survives() {
        let inner = "a:Pigñata;E:24241;";
        let text = format!("b:{};", b64(inner));
        let node = parse(&text);
        assert_eq!(node.get_path(&["b", "a"]).unwrap().as_str(), Some("Pigñata"));
    }

    #[test]
    fn float_and_scientific_notation() {
        let node = parse("a:66841.3595410302;b:7.37927073370121E+185;");
        assert!((node.get("a").unwrap().as_f64().unwrap() - 66841.3595410302).abs() < 1e-9);
        assert!(node.get("b").unwrap().as_f64().unwrap() > 1e185);
    }

    #[test]
    fn recursion_depth_is_capped() {
        // Nest base64 structs past MAX_DEPTH; the parser must degrade to a
        // leaf instead of recursing forever.
        let mut text = "a:1;".to_string();
        for _ in 0..(MAX_DEPTH + 4) {
            text = format!("a:{};", b64(&text));
        }
        let mut node = &parse(&text);
        let mut depth = 0;
        while let Some(inner) = node.get("a") {
            node = inner;
            depth += 1;
            if matches!(node, Node::Leaf(_)) {
                break;
            }
        }
        assert!(matches!(node, Node::Leaf(_)));
        assert!(depth <= MAX_DEPTH + 1);
    }

    #[test]
    fn dump_renders_nested() {
        let text = format!("a:1;w:{};", b64("b:2;"));
        let dump = parse(&text).dump();
        assert!(dump.contains("a = 1"));
        assert!(dump.contains("  w =\n"));
        assert!(dump.contains("    b = 2"));
    }
}
