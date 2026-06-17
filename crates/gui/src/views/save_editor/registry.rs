//! The field registry: a declarative map from raw-tree paths to human names,
//! value kinds, and the section each belongs to.
//!
//! This is the extensibility seam. The raw tree navigator uses it to label
//! otherwise-nebulous keys; structured sections use it to enumerate their fields
//! without hardcoding paths in two places. Adding a new area (RTI, the planet,
//! the crystal factory, …) later is a matter of adding entries here plus a
//! section module — no change to the editor core.
//!
//! Path semantics match [`save_parser::raw::Raw::get_path`]: struct keys, plus
//! list selectors (a numeric index or `field=value`) when a node is a list.
//! Field meanings are sourced from `reference/save_file_deserialization/FINDINGS.md`.

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
/// `Bool`/`Text` are not yet used by any seeded field — they exist for sections
/// added next (the resources editor already handles them).
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

/// One known field.
#[derive(Clone)]
pub struct FieldDef {
    pub path: &'static [&'static str],
    pub name: &'static str,
    pub kind: FieldKind,
    pub section: SectionId,
    pub help: Option<&'static str>,
}

/// The registry of known fields.
pub struct FieldRegistry {
    fields: Vec<FieldDef>,
}

impl Default for FieldRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldRegistry {
    pub fn new() -> Self {
        Self { fields: seed() }
    }

    /// Fields belonging to a section, in declaration order.
    pub fn for_section(&self, section: SectionId) -> impl Iterator<Item = &FieldDef> {
        self.fields.iter().filter(move |f| f.section == section)
    }

    /// The known field at an exact path, if any (for labeling tree nodes).
    pub fn lookup(&self, path: &[&str]) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.path == path)
    }
}

/// Build a field definition (terse helper for the seed table).
const fn def(
    path: &'static [&'static str],
    name: &'static str,
    kind: FieldKind,
    section: SectionId,
    help: &'static str,
) -> FieldDef {
    FieldDef {
        path,
        name,
        kind,
        section,
        help: if help.is_empty() { None } else { Some(help) },
    }
}

/// The seed registry. Start with the high-confidence flat scalars from
/// FINDINGS.md; grow this as sections are added.
fn seed() -> Vec<FieldDef> {
    use FieldKind::{Number, Text};
    use SectionId::{RawTree, Resources};
    vec![
        // -- Resources & currencies (the structured section) --
        // God-power block (root `p`).
        def(&["p", "j"], "Available God Power", Number, Resources, "Spendable GP (root p.j)"),
        def(&["p", "v"], "God Power Spent", Number, Resources, "Lifetime GP spent (p.v)"),
        def(&["p", "002"], "Crystal Power", Number, Resources, "p.002"),
        def(&["p", "F"], "Total Might (all rebirths)", Number, Resources, "Global Total Might (p.F)"),
        // Pet system block (root `X`).
        def(&["X", "y"], "Pet Stones", Number, Resources, "Current pet stones (X.y)"),
        def(&["X", "z"], "Pet Stones Spent", Number, Resources, "Cumulative spent (X.z)"),
        def(&["X", "Y"], "Free Experience", Number, Resources, "Pet free experience (X.Y)"),
        // Other currency blocks.
        def(&["K", "g"], "Total Divinity", Number, Resources, "Large double, scientific (K.g)"),
        def(&["T", "h"], "Unspent Baal Power", Number, Resources, "T.h"),
        def(&["025", "a"], "Fish Power", Number, Resources, "025.a"),
        def(&["e", "a"], "Shadow Clones (current)", Number, Resources, "e.a"),
        def(&["e", "b"], "Shadow Clones (max)", Number, Resources, "e.b"),

        // -- Tree labels only (not yet a structured section) --
        // Top-level blocks: label the container so the tree is scannable.
        def(&["X"], "Pets / Pet System", Text, RawTree, "All pets, equipment, materials, teams, campaigns"),
        def(&["p"], "God Power", Text, RawTree, "God-power block"),
        def(&["D"], "Monuments", Text, RawTree, "Mighty Statue … White Hole"),
        def(&["V"], "Mights", Text, RawTree, "14 mights"),
        def(&["i"], "Creations", Text, RawTree, "Shadow Clone … Universe"),
        def(&["K"], "Divinity Generator", Text, RawTree, ""),
        def(&["T"], "Baal Slayer", Text, RawTree, ""),
        def(&["P"], "Current God Fight", Text, RawTree, ""),
        def(&["e"], "Shadow Clones", Text, RawTree, ""),
        def(&["025"], "Fishing", Text, RawTree, ""),
        // Identity & meta (root scalars).
        def(&["W"], "God Name", Text, RawTree, "In-game deity name (identity)"),
        def(&["s"], "Account Login", Text, RawTree, "Linked Steam/Kongregate account (identity)"),
        def(&["g"], "God Title", Text, RawTree, ""),
        def(&["c"], "Saved (unix seconds)", Number, RawTree, ""),
        def(&["005"], "Saved (unix ms)", Number, RawTree, ""),
        def(&["001"], "Steam id64", Text, RawTree, "identity"),
        def(&["002"], "Steam persona name", Text, RawTree, "identity"),
        def(&["003"], "Account / guest id", Text, RawTree, "identity"),
        def(&["004"], "Steam display name", Text, RawTree, "identity"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::save_editor::session::EditSession;

    /// Load the committed (redacted) reference save for coverage checks.
    fn fixture_session() -> EditSession {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt"
        );
        let raw = std::fs::read_to_string(path).expect("reference save fixture is present");
        EditSession::load(&raw, None).expect("reference save decodes")
    }

    /// Every seeded path must resolve in a real save — guards against typo'd or
    /// stale paths, in the spirit of the planner's `test_campaign_bonus_coverage`.
    /// Uses `path_exists` (not `value`) so block-level container labels, which
    /// aren't scalars, are covered too.
    #[test]
    fn every_registry_path_resolves() {
        let session = fixture_session();
        let registry = FieldRegistry::new();
        let mut missing = Vec::new();
        for field in &registry.fields {
            if !session.path_exists(field.path) {
                missing.push(format!("{} ({})", field.path.join("."), field.name));
            }
        }
        assert!(
            missing.is_empty(),
            "registry paths absent from the reference save: {missing:?}"
        );
    }

    /// End-to-end on a real save: edit a field, re-encode the whole container,
    /// and confirm it round-trips (decodes, validates, reloads with the edit).
    #[test]
    fn real_save_edit_round_trips() {
        let mut session = fixture_session();
        let before = session.value(&["p", "j"]).expect("save has available GP");
        assert_ne!(before, "123456789");

        session
            .set_scalar(&["p", "j"], "Available God Power", "123456789")
            .unwrap();

        let encoded = session.encode();
        session.validate_encoded(&encoded).expect("round-trips");

        let reloaded = EditSession::load(&encoded, None).unwrap();
        assert_eq!(reloaded.value(&["p", "j"]).as_deref(), Some("123456789"));
        // An unrelated field is untouched.
        assert_eq!(
            reloaded.value(&["X", "y"]),
            session.value(&["X", "y"]),
            "pet stones unchanged by the GP edit"
        );
    }

    #[test]
    fn lookup_matches_exact_path() {
        let registry = FieldRegistry::new();
        assert_eq!(
            registry.lookup(&["p", "j"]).map(|f| f.name),
            Some("Available God Power")
        );
        assert!(registry.lookup(&["p", "nonexistent"]).is_none());
    }
}
