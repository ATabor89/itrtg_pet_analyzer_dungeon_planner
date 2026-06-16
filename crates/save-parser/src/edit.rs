//! Save editing: apply scalar field overrides to a decoded save and faithfully
//! re-encode it, so the result loads in-game.
//!
//! This is a deliberate single-player "cheat"/debug aid — the game's author is
//! fine with edited saves as long as high-score submission is disabled. Its
//! main use here is reverse-engineering: granting currency to buy upgrades, and
//! nudging a *maxed* upgrade **down** a level (which the game can't do) to
//! disambiguate which save field backs it (e.g. Camp Exp Boost vs the TBS
//! double-points field, both stored as 100 — see FINDINGS.md).
//!
//! It builds on the lossless [`raw`](crate::raw) layer: every byte except the
//! edited scalars is preserved, and [`crate::container`] round-trips the gzip
//! container. The output carries **real, unredacted** save data — never commit
//! it (`.gitignore` covers `**/edited_*.txt`).

use anyhow::{Context, Result};

use crate::{container, raw};

/// One field override: the dotted raw-tree path and the new scalar text.
#[derive(Debug, Clone)]
pub struct ScalarEdit {
    pub path: Vec<String>,
    pub value: String,
}

impl ScalarEdit {
    /// Build from a dotted path string (e.g. `"p.025"`) and a value.
    pub fn parse(path: &str, value: &str) -> Self {
        ScalarEdit {
            path: path.split('.').map(str::to_string).collect(),
            value: value.to_string(),
        }
    }
}

/// What an applied edit changed, for reporting back to the user.
#[derive(Debug, Clone)]
pub struct AppliedEdit {
    pub path: String,
    pub old: String,
    pub new: String,
}

/// Resolve a friendly currency name to its raw-tree path. Only fields we have
/// actually *located* are named here; everything else uses an explicit path via
/// [`ScalarEdit::parse`]. (ChP / Overflow Points are not yet located — they are
/// not stored as recoverable scalars in any captured save, so they await a
/// purpose-built before/after save to pin down.)
pub fn named_target(name: &str) -> Option<&'static [&'static str]> {
    match name {
        // Available god power (root `p.j`), verified across the reference saves.
        "gp" => Some(&["p", "j"]),
        _ => None,
    }
}

/// Decode `raw_save`, apply every edit to the lossless tree, and re-encode the
/// container. Returns the new save text plus a record of each change.
///
/// After encoding, it decodes the result again and confirms each edited path
/// now reads the requested value — so a serializer/encoder bug surfaces here
/// rather than as a corrupt save the game silently rejects.
pub fn edit_save(raw_save: &str, edits: &[ScalarEdit]) -> Result<(String, Vec<AppliedEdit>)> {
    let decoded = container::decode_container(raw_save).context("decode save container")?;
    let mut root = raw::parse(&decoded.plaintext);

    let mut applied = Vec::with_capacity(edits.len());
    for edit in edits {
        let segs: Vec<&str> = edit.path.iter().map(String::as_str).collect();
        let old = root
            .set_scalar_path(&segs, &edit.value)
            .with_context(|| format!("set {}", edit.path.join(".")))?;
        applied.push(AppliedEdit {
            path: edit.path.join("."),
            old,
            new: edit.value.clone(),
        });
    }

    let reserialized = root.serialize();
    let encoded = container::encode_container(&reserialized, &decoded.prefix);

    // Self-check: the re-encoded save must decode back and read the new values.
    let check_plaintext = container::decode_to_plaintext(&encoded)
        .context("re-decode the edited save for verification")?;
    let check_root = raw::parse(&check_plaintext);
    for edit in edits {
        let segs: Vec<&str> = edit.path.iter().map(String::as_str).collect();
        match check_root.get_path(&segs) {
            Some(raw::Raw::Scalar(s)) if *s == edit.value => {}
            other => anyhow::bail!(
                "verification failed for {}: expected {:?}, found {:?}",
                edit.path.join("."),
                edit.value,
                other
            ),
        }
    }

    Ok((encoded, applied))
}
