//! Parser for full ITRTG save files.
//!
//! The save format was reverse-engineered against same-session in-game
//! exports; the working notes live in `reference/save_file_deserialization/`
//! (`FINDINGS.md`). Three layers:
//!
//! 1. [`container`] — the outer encoding: 2 junk characters, base64, a
//!    4-byte length prefix, gzip, and base64 again, yielding a plaintext
//!    serialized tree.
//! 2. [`tree`] — the generic `key:value;` tree grammar (nested structs are
//!    base64-encoded values, lists are `&`-joined base64 elements).
//! 3. [`model`] — typed extraction of the parts we have identified so far
//!    (pets, equipment, materials, dungeon teams, campaigns).
//!
//! Anything not yet identified stays reachable through the raw [`tree::Node`]
//! kept on [`model::SaveFile`], so gaps can be explored without re-decoding.

pub mod container;
pub mod model;
pub mod tree;

pub use model::SaveFile;
pub use tree::Node;

/// Decode and parse a raw save file string into the typed model.
pub fn parse_save(raw: &str) -> anyhow::Result<SaveFile> {
    let plaintext = container::decode_to_plaintext(raw)?;
    let root = tree::parse(&plaintext);
    SaveFile::from_tree(root)
}
