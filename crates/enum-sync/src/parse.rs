//! Extract `enum` definitions from decompiled C# source.
//!
//! The game's `Assembly-CSharp.dll` is obfuscated, but ILSpy decompiles enums
//! to flat, readable blocks:
//!
//! ```text
//! public enum DEONLDHEFBF
//! {
//!     Mouse = 0,
//!     Rabbit = 1,
//!     ...
//! }
//! ```
//!
//! Member names and integer values survive obfuscation (the game serializes
//! them); only the *type* name is randomized, and it rotates per build. So we
//! parse every enum here and let the caller identify which is which by content
//! (see `registry`/`match_enum`), never by type name.

use std::path::{Path, PathBuf};

/// A parsed C# enum: its (obfuscated) type name and its members in declaration
/// order, each resolved to an explicit integer value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEnum {
    pub type_name: String,
    /// `(member_name, value)` in declaration order.
    pub members: Vec<(String, i64)>,
}

impl ParsedEnum {
    /// Value → member name (last write wins on the rare duplicate value).
    pub fn by_value(&self) -> std::collections::BTreeMap<i64, &str> {
        self.members
            .iter()
            .map(|(n, v)| (*v, n.as_str()))
            .collect()
    }
}

/// Recursively collect every `.cs` file under `dir`.
pub fn collect_cs_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_into(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_into(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_into(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("cs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Parse every enum in a directory of decompiled `.cs` files.
pub fn parse_dir(dir: &Path) -> anyhow::Result<Vec<ParsedEnum>> {
    let mut enums = Vec::new();
    for file in collect_cs_files(dir)? {
        let src = std::fs::read_to_string(&file)?;
        enums.extend(extract_enums(&src));
    }
    Ok(enums)
}

/// Extract all `enum Name { ... }` blocks from a single source string.
///
/// Tolerates `public`/`internal` modifiers and a `: <base-type>` underlying
/// type, and resolves implicit (auto-incremented) member values. Skips the word
/// "enum" wherever it appears in prose without a following `Identifier { ... }`.
pub fn extract_enums(src: &str) -> Vec<ParsedEnum> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut search = 0usize;

    while let Some(rel) = src[search..].find("enum") {
        let kw = search + rel;
        search = kw + 4;

        // Require "enum" to be a standalone word.
        if kw > 0 && is_ident_byte(bytes[kw - 1]) {
            continue;
        }
        let after_kw = kw + 4;
        if after_kw >= bytes.len() || is_ident_byte(bytes[after_kw]) {
            continue;
        }

        // Identifier (the type name).
        let mut i = skip_ws(bytes, after_kw);
        let name_start = i;
        while i < bytes.len() && is_ident_byte(bytes[i]) {
            i += 1;
        }
        if i == name_start {
            continue;
        }
        let type_name = src[name_start..i].to_string();

        // Optionally `: BaseType`, then the opening brace — but only across
        // whitespace / `:` / identifier chars, so prose like "enum member that"
        // never reaches a stray `{`.
        let mut j = i;
        let mut ok = true;
        loop {
            j = skip_ws(bytes, j);
            if j >= bytes.len() {
                ok = false;
                break;
            }
            match bytes[j] {
                b'{' => break,
                b':' => j += 1,
                c if is_ident_byte(c) => {
                    while j < bytes.len() && is_ident_byte(bytes[j]) {
                        j += 1;
                    }
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }

        // Body up to the matching close brace (enum bodies are flat).
        let body_start = j + 1;
        let Some(close_rel) = src[body_start..].find('}') else {
            continue;
        };
        let body = &src[body_start..body_start + close_rel];

        if let Some(members) = parse_members(body) {
            out.push(ParsedEnum { type_name, members });
        }
        search = body_start + close_rel + 1;
    }

    out
}

/// Parse a flat enum body (`A = 0, B = 1, C,` …) into resolved members.
fn parse_members(body: &str) -> Option<Vec<(String, i64)>> {
    let mut members = Vec::new();
    let mut next_implicit = 0i64;

    for raw in body.split(',') {
        // Drop any trailing line comment, then trim.
        let entry = raw.split("//").next().unwrap_or("").trim();
        if entry.is_empty() {
            continue;
        }

        let eb = entry.as_bytes();
        let mut k = 0;
        while k < eb.len() && is_ident_byte(eb[k]) {
            k += 1;
        }
        if k == 0 {
            // Not a member (attribute, stray token) — bail on the whole enum
            // rather than guess, so we never emit half-parsed garbage.
            return None;
        }
        let name = entry[..k].to_string();

        let rest = entry[k..].trim_start();
        let value = if let Some(num) = rest.strip_prefix('=') {
            parse_int(num.trim())?
        } else if rest.is_empty() {
            next_implicit
        } else {
            return None;
        };

        next_implicit = value + 1;
        members.push((name, value));
    }

    if members.is_empty() {
        None
    } else {
        Some(members)
    }
}

/// Parse a C# integer literal: decimal or `0x` hex, optional sign, optional
/// `u`/`l` suffixes.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(hex) = s.strip_prefix("-0x").or_else(|| s.strip_prefix("-0X")) {
        i64::from_str_radix(hex, 16).ok().map(|v| -v)
    } else {
        s.parse::<i64>().ok()
    }
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_values() {
        let src = "public enum Foo {\n Mouse = 0,\n Rabbit = 1,\n Cat = 2,\n}";
        let e = extract_enums(src);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].type_name, "Foo");
        assert_eq!(
            e[0].members,
            vec![
                ("Mouse".into(), 0),
                ("Rabbit".into(), 1),
                ("Cat".into(), 2),
            ]
        );
    }

    #[test]
    fn resolves_implicit_and_sparse_values() {
        // Implicit increments, then a jump, then implicit again.
        let src = "enum E { A, B, C = 10, D, E = 0x20 }";
        let e = extract_enums(src);
        assert_eq!(
            e[0].members,
            vec![
                ("A".into(), 0),
                ("B".into(), 1),
                ("C".into(), 10),
                ("D".into(), 11),
                ("E".into(), 32),
            ]
        );
    }

    #[test]
    fn handles_base_type_and_modifiers() {
        let src = "internal enum Flags : byte { None = 0, Set = 1 }";
        let e = extract_enums(src);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].type_name, "Flags");
        assert_eq!(e[0].members.len(), 2);
    }

    #[test]
    fn ignores_enum_in_prose() {
        // "enum" as an English word in a doc comment must not parse.
        let src = "/// The enum below is transcribed. Some other text.\nclass C {}";
        assert!(extract_enums(src).is_empty());
    }

    #[test]
    fn extracts_multiple_enums_from_one_file() {
        let src = "public enum A { X = 1 }\n\npublic enum B { Y = 2, Z = 3 }";
        let e = extract_enums(src);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].type_name, "A");
        assert_eq!(e[1].type_name, "B");
    }

    #[test]
    fn by_value_lookup() {
        let src = "enum E { A = 5, B = 7 }";
        let e = &extract_enums(src)[0];
        let map = e.by_value();
        assert_eq!(map.get(&5), Some(&"A"));
        assert_eq!(map.get(&7), Some(&"B"));
    }
}
