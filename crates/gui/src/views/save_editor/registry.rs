//! The field registry: a map from raw-tree paths to human names, value kinds,
//! and the section each belongs to.
//!
//! Most labels are *derived from the model's schema* ([`save_parser::labels`]):
//! each block there (pets, equipment, creations, …) expands into wildcard path
//! patterns here, so one schema entry labels every element of a list. A `"*"`
//! segment in a pattern matches any single path segment (a list index). Adding a
//! label is a one-line change next to the model, not a parallel map.
//!
//! A handful of flat scalars (currencies, identity/meta, scattered `p.*` fields)
//! are still listed explicitly below — they don't belong to a repeated block.
//! Path semantics match [`save_parser::raw::Raw::get_path`].

use std::collections::HashMap;

use save_parser::labels::{BLOCKS, BlockSchema};

/// Which editor section a field is surfaced in (also the left-nav identity).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SectionId {
    #[default]
    Resources,
    RawTree,
}

impl SectionId {
    /// Sections shown in the left nav, in order.
    pub const ALL: &'static [SectionId] = &[SectionId::Resources, SectionId::RawTree];

    pub fn title(self) -> &'static str {
        match self {
            SectionId::Resources => "Resources & Currencies",
            SectionId::RawTree => "Raw Save Tree",
        }
    }
}

/// How a scalar field should be edited and validated.
///
/// `Bool`/`Text` aren't used by a seeded *Resources* field yet — they exist for
/// sections added next (the resources editor already handles them).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum FieldKind {
    /// An arbitrary-magnitude number (integer or scientific-notation double).
    /// Edited as validated text and written verbatim, so 17-digit counts and
    /// values like `7.3E+185` survive without floating-point precision loss.
    Number,
    /// A `True`/`False` boolean.
    Bool,
    /// Free text.
    Text,
}

/// One known field. `path` is a pattern: a `"*"` segment matches any one segment.
#[derive(Clone)]
pub struct FieldDef {
    pub path: Vec<&'static str>,
    pub name: &'static str,
    pub kind: FieldKind,
    pub section: SectionId,
    pub help: Option<&'static str>,
}

/// The registry of known fields, indexed by pattern length for fast lookup.
pub struct FieldRegistry {
    fields: Vec<FieldDef>,
    by_len: HashMap<usize, Vec<usize>>,
}

impl Default for FieldRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldRegistry {
    pub fn new() -> Self {
        let fields = seed();
        let mut by_len: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, f) in fields.iter().enumerate() {
            by_len.entry(f.path.len()).or_default().push(i);
        }
        Self { fields, by_len }
    }

    /// Fields belonging to a section, in declaration order.
    pub fn for_section(&self, section: SectionId) -> impl Iterator<Item = &FieldDef> {
        self.fields.iter().filter(move |f| f.section == section)
    }

    /// The known field whose pattern matches `path` (for labeling tree nodes).
    /// Exact entries are seeded before wildcard ones, so they win on overlap.
    pub fn lookup(&self, path: &[&str]) -> Option<&FieldDef> {
        let bucket = self.by_len.get(&path.len())?;
        bucket
            .iter()
            .map(|&i| &self.fields[i])
            .find(|f| pattern_matches(&f.path, path))
    }
}

/// Does a registry pattern match a concrete path? (Lengths are equal via the
/// `by_len` bucket; `"*"` matches any one segment.)
fn pattern_matches(pattern: &[&str], path: &[&str]) -> bool {
    pattern.len() == path.len()
        && pattern
            .iter()
            .zip(path)
            .all(|(p, s)| *p == "*" || *p == *s)
}

/// An explicitly-listed field (non-block).
fn def(
    path: &'static [&'static str],
    name: &'static str,
    kind: FieldKind,
    section: SectionId,
    help: &'static str,
) -> FieldDef {
    FieldDef {
        path: path.to_vec(),
        name,
        kind,
        section,
        help: if help.is_empty() { None } else { Some(help) },
    }
}

/// A label-only entry (tree navigation), used for schema-derived block fields.
fn label(path: Vec<&'static str>, name: &'static str) -> FieldDef {
    FieldDef {
        path,
        name,
        kind: FieldKind::Text,
        section: SectionId::RawTree,
        help: None,
    }
}

/// Expand one model block into wildcard patterns: a label for the block
/// container, one for each element, and one per labeled field.
fn push_block(out: &mut Vec<FieldDef>, block: &BlockSchema) {
    let mut element: Vec<&'static str> = block.base.to_vec();
    out.push(label(element.clone(), block.plural)); // the block/list container
    if block.is_list {
        element.push("*"); // each element
        out.push(label(element.clone(), block.name));
    }
    for fl in block.fields {
        let mut p = element.clone();
        p.extend(fl.key.split('.'));
        out.push(label(p, fl.label));
    }
}

/// Build the registry: explicit flat scalars first (so they win on overlap),
/// then the model-schema blocks expanded into wildcard patterns.
fn seed() -> Vec<FieldDef> {
    use FieldKind::{Number, Text};
    use SectionId::{RawTree, Resources};

    let mut v = vec![
        // -- Resources & currencies (the structured section) --
        def(&["p", "j"], "Available God Power", Number, Resources, "Spendable GP (p.j)"),
        def(&["p", "v"], "God Power Spent", Number, Resources, "Lifetime GP spent (p.v)"),
        def(&["p", "002"], "Crystal Power", Number, Resources, "p.002"),
        def(&["p", "F"], "Total Might (all rebirths)", Number, Resources, "p.F"),
        def(&["X", "y"], "Pet Stones", Number, Resources, "X.y"),
        def(&["X", "z"], "Pet Stones Spent", Number, Resources, "X.z"),
        def(&["X", "Y"], "Free Experience", Number, Resources, "X.Y"),
        def(&["K", "g"], "Total Divinity", Number, Resources, "Large double, scientific (K.g)"),
        def(&["T", "h"], "Unspent Baal Power", Number, Resources, "T.h"),
        def(&["025", "a"], "Fish Power", Number, Resources, "025.a"),
        def(&["e", "a"], "Shadow Clones (current)", Number, Resources, "e.a"),
        def(&["e", "b"], "Shadow Clones (max)", Number, Resources, "e.b"),
        def(&["p", "I"], "Pet Tokens", Number, Resources, "p.I"),
        def(&["p", "023"], "Class Change Tokens", Number, Resources, "p.023"),
        def(&["p", "K"], "Lucky Draws", Number, Resources, "p.K"),
        def(&["X", "c"], "Puny Food", Number, Resources, "X.c"),
        def(&["X", "d"], "Strong Food", Number, Resources, "X.d"),
        def(&["X", "e"], "Mighty Food", Number, Resources, "X.e"),
        def(&["X", "v"], "Chocolate", Number, Resources, "X.v"),

        // -- Tree labels only: root block containers not in the model schema --
        def(&["X"], "Pets / Pet System", Text, RawTree, "Pets, equipment, materials, teams, campaigns"),
        def(&["p"], "God Power", Text, RawTree, ""),
        def(&["e"], "Shadow Clones", Text, RawTree, ""),
        def(&["T"], "Baal Slayer (Baal Power)", Text, RawTree, ""),
        def(&["P"], "Current God Fight", Text, RawTree, ""),
        def(&["025"], "Fishing", Text, RawTree, ""),
        def(&["009"], "SpaceDim (Light Dimension)", Text, RawTree, ""),
        def(&["032"], "Adventure / Research", Text, RawTree, ""),
        def(&["K"], "Divinity Generator", Text, RawTree, ""),

        // -- Tree labels only: scattered god-power / consumable / upgrade scalars --
        def(&["p", "b"], "Godly Liquid", Text, RawTree, "p.b"),
        def(&["p", "m"], "Godly Liquid v2", Text, RawTree, "p.m"),
        def(&["p", "d"], "Chakra Pill", Text, RawTree, "p.d"),
        def(&["p", "n"], "Chakra Pill v2", Text, RawTree, "p.n"),
        def(&["p", "e"], "Ultimate Shadow Summon", Text, RawTree, "p.e"),
        def(&["p", "h"], "GP Creating Speed %", Text, RawTree, "p.h"),
        def(&["p", "i"], "GP Building Speed %", Text, RawTree, "p.i"),
        def(&["p", "C"], "Statistics Multi", Text, RawTree, "p.C"),
        def(&["p", "q"], "Creation Count (GP)", Text, RawTree, "p.q"),
        def(&["p", "r"], "Unused-GP Alloc: Physical %", Text, RawTree, "p.r"),
        def(&["p", "s"], "Unused-GP Alloc: Mystic %", Text, RawTree, "p.s"),
        def(&["p", "t"], "Unused-GP Alloc: Battle %", Text, RawTree, "p.t"),
        def(&["p", "u"], "Unused-GP Alloc: Creating %", Text, RawTree, "p.u"),
        def(&["p", "001"], "Max Crystal (upgrade)", Text, RawTree, "p.001"),
        def(&["p", "018"], "Inventory Limit (upgrade)", Text, RawTree, "p.018"),
        def(&["p", "021"], "Item Slots (upgrade)", Text, RawTree, "p.021"),
        def(&["p", "025"], "Camp Exp Boost % (upgrade)", Text, RawTree, "p.025"),
        def(&["p", "017"], "Dungeon Loot % (upgrade)", Text, RawTree, "p.017"),
        def(&["p", "019"], "Dungeon Exp % (upgrade)", Text, RawTree, "p.019"),
        def(&["p", "020"], "Crafting Boost % (upgrade)", Text, RawTree, "p.020"),
        def(&["X", "032"], "Crafting Queue Slots", Text, RawTree, "X.032"),
        def(&["x", "k"], "Rebirths", Text, RawTree, "x.k"),
        def(&["O", "030"], "Light Clones", Text, RawTree, "O.030"),
        def(&["018"], "Earth Eater Planets (rebirth)", Text, RawTree, "root 018"),
        def(&["033"], "Anni Cake Bonus %", Text, RawTree, "root 033"),

        // -- Identity & meta (root scalars) --
        def(&["W"], "God Name", Text, RawTree, "In-game deity name (identity)"),
        def(&["s"], "Account Login", Text, RawTree, "Linked account (identity)"),
        def(&["g"], "God Title", Text, RawTree, ""),
        def(&["c"], "Saved (unix seconds)", Number, RawTree, ""),
        def(&["005"], "Saved (unix ms)", Number, RawTree, ""),
        def(&["001"], "Steam id64", Text, RawTree, "identity"),
        def(&["002"], "Steam persona name", Text, RawTree, "identity"),
        def(&["003"], "Account / guest id", Text, RawTree, "identity"),
        def(&["004"], "Steam display name", Text, RawTree, "identity"),
    ];

    // Model-schema blocks → wildcard patterns (pets, equipment, creations, …).
    for block in BLOCKS {
        push_block(&mut v, block);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::save_editor::session::EditSession;
    use save_parser::raw::Raw;

    /// Load the committed (redacted) reference save for coverage checks.
    fn fixture_session() -> EditSession {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt"
        );
        let raw = std::fs::read_to_string(path).expect("reference save fixture is present");
        EditSession::load(&raw, None).expect("reference save decodes")
    }

    /// Resolve a (possibly wildcard) pattern against the fixture: an exact path
    /// must exist; a wildcard pattern must resolve on at least one element of its
    /// list (so optional per-element fields don't cause false negatives, while a
    /// typo'd key — present on no element — is still caught).
    fn pattern_resolves(session: &EditSession, pat: &[&'static str]) -> bool {
        match pat.iter().position(|s| *s == "*") {
            None => session.path_exists(pat),
            Some(star) => {
                let base = &pat[..star];
                let suffix = &pat[star + 1..];
                match session.root().get_path(base) {
                    Some(Raw::List(items)) => (0..items.len()).any(|i| {
                        let idx = i.to_string();
                        let mut full: Vec<&str> = base.to_vec();
                        full.push(idx.as_str());
                        full.extend(suffix.iter().copied());
                        session.path_exists(&full)
                    }),
                    // A 1-element list re-parses as a lone struct (list_or_single):
                    // the element *is* the struct at `base`, so the field sits at
                    // `base ++ suffix` with no index.
                    Some(Raw::Struct(_)) => {
                        let mut full: Vec<&str> = base.to_vec();
                        full.extend(suffix.iter().copied());
                        session.path_exists(&full)
                    }
                    _ => false,
                }
            }
        }
    }

    /// Every seeded pattern must resolve in a real save — guards against typo'd
    /// or stale keys, in the spirit of the planner's `test_campaign_bonus_coverage`.
    #[test]
    fn every_registry_path_resolves() {
        let session = fixture_session();
        let registry = FieldRegistry::new();
        let mut missing = Vec::new();
        for field in &registry.fields {
            if !pattern_resolves(&session, &field.path) {
                missing.push(format!("{} ({})", field.path.join("."), field.name));
            }
        }
        assert!(
            missing.is_empty(),
            "registry paths absent from the reference save: {missing:?}"
        );
    }

    #[test]
    fn lookup_matches_exact_and_wildcard() {
        let registry = FieldRegistry::new();
        // Exact.
        assert_eq!(
            registry.lookup(&["p", "j"]).map(|f| f.name),
            Some("Available God Power")
        );
        // Wildcard: a pet's growth field, any index.
        assert_eq!(
            registry.lookup(&["X", "b", "17", "E"]).map(|f| f.name),
            Some("Growth")
        );
        // Nested wildcard: class level.
        assert_eq!(
            registry.lookup(&["X", "b", "3", "w", "d", "b"]).map(|f| f.name),
            Some("Class Level")
        );
        assert!(registry.lookup(&["X", "b", "0", "zzz"]).is_none());
    }
}
