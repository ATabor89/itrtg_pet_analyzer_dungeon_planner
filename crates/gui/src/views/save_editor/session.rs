//! The editing spine for a loaded save.
//!
//! [`EditSession`] owns the single source of truth — the lossless
//! [`save_parser::raw::Raw`] tree — and is the only thing that mutates it.
//! Every edit, whether it comes from a structured section or the raw tree
//! navigator, routes through [`EditSession::set_scalar`], so the two views can
//! never drift: they both read and write the same tree.
//!
//! The typed [`SaveFile`] is a *derived, display-only* projection, re-built from
//! the raw tree after an edit. It is never mutated independently.

use anyhow::{Context, Result, bail};
use save_parser::container::{self, ContainerFormat};
use save_parser::model::SaveFile;
use save_parser::raw::{self, Raw};
use save_parser::redact;
use save_parser::tree;

/// One field change relative to the save as it was loaded. The `pending` list is
/// always the *net* set of changes from the loaded state (repeat edits to the
/// same path coalesce), so it doubles as the undo log and the change preview.
#[derive(Clone)]
pub struct PendingEdit {
    /// Dotted path segments (struct keys / list selectors) into the raw tree.
    pub path: Vec<String>,
    /// Human label for the change (registry name, or the dotted path).
    pub label: String,
    /// The value as the save was loaded — what `undo` restores.
    pub original: String,
    /// The current value.
    pub new: String,
}

/// A loaded save plus all in-progress edits.
pub struct EditSession {
    /// File name the save was loaded from, for display.
    pub source_name: Option<String>,
    /// The leading container junk prefix (`V2`), needed to re-encode.
    prefix: String,
    /// Which platform container the save was decoded from (display only).
    format: ContainerFormat,
    /// The canonical, mutable tree. All edits land here.
    root: Raw,
    /// Display-only typed projection, re-derived after edits. `None` if the
    /// typed extraction failed — raw editing still works regardless, so a save
    /// the typed model can't fully parse is still editable.
    derived: Option<SaveFile>,
    pending: Vec<PendingEdit>,
    /// Set when an edit invalidates `derived`; cleared by `rederive_if_needed`.
    dirty_derived: bool,
}

impl EditSession {
    /// Decode and parse a raw save string into an editable session.
    pub fn load(raw_text: &str, source_name: Option<String>) -> Result<Self> {
        let decoded =
            container::decode_container(raw_text).context("decoding the save container")?;
        let root = raw::parse(&decoded.plaintext);
        // The typed projection is best-effort: raw editing does not depend on it.
        let derived = derive(&root).ok();
        Ok(Self {
            source_name,
            prefix: decoded.prefix,
            format: decoded.format,
            root,
            derived,
            pending: Vec::new(),
            dirty_derived: false,
        })
    }

    /// A friendly label for which platform container the save came from.
    pub fn format_label(&self) -> &'static str {
        match self.format {
            ContainerFormat::SteamGzip => "Steam",
            ContainerFormat::KongregateLzf => "web/Kongregate",
        }
    }

    /// The derived typed projection, if it parsed (refresh with
    /// [`rederive_if_needed`](Self::rederive_if_needed) first).
    pub fn derived(&self) -> Option<&SaveFile> {
        self.derived.as_ref()
    }

    pub fn pending(&self) -> &[PendingEdit] {
        &self.pending
    }

    pub fn is_dirty(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The canonical tree, for read-only traversal (the raw tree navigator).
    /// Edits must still go through [`set_scalar`](Self::set_scalar).
    pub fn root(&self) -> &Raw {
        &self.root
    }

    /// The current scalar text at `path`, or `None` if the path is absent, names
    /// a non-scalar, or is an empty field.
    pub fn value(&self, path: &[&str]) -> Option<String> {
        match self.root.get_path(path)? {
            Raw::Scalar(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Apply a scalar edit and record it in the pending log.
    ///
    /// The log is kept as the *net* change from the loaded save: editing the same
    /// path twice coalesces into one entry, and editing a field back to its
    /// loaded value drops the entry entirely.
    pub fn set_scalar(
        &mut self,
        path: &[&str],
        label: impl Into<String>,
        value: &str,
    ) -> Result<()> {
        // `prev` is the value immediately before this write — the loaded value on
        // the first edit, or the previous edited value on a repeat.
        let prev = self.root.set_scalar_path(path, value)?;
        self.dirty_derived = true;

        let owned: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        if let Some(existing) = self.pending.iter().position(|e| e.path == owned) {
            if self.pending[existing].original == value {
                // Reverted to the loaded value — no net change remains.
                self.pending.remove(existing);
            } else {
                self.pending[existing].new = value.to_string();
            }
        } else if prev != value {
            self.pending.push(PendingEdit {
                path: owned,
                label: label.into(),
                original: prev,
                new: value.to_string(),
            });
        }
        Ok(())
    }

    /// Revert one pending edit (by index into [`pending`]).
    pub fn undo(&mut self, index: usize) -> Result<()> {
        let Some(edit) = self.pending.get(index) else {
            return Ok(());
        };
        let path: Vec<&str> = edit.path.iter().map(|s| s.as_str()).collect();
        let original = edit.original.clone();
        self.root.set_scalar_path(&path, &original)?;
        self.pending.remove(index);
        self.dirty_derived = true;
        Ok(())
    }

    /// Rebuild the derived typed projection if an edit invalidated it. Call once
    /// per frame before reading [`derived`].
    pub fn rederive_if_needed(&mut self) {
        if self.dirty_derived {
            self.derived = derive(&self.root).ok();
            self.dirty_derived = false;
        }
    }

    // Saving is desktop-only for now, so these are unused on wasm (where there
    // is no file write and no test harness) — silence dead-code there only.

    /// Serialize the canonical tree back to plaintext.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn plaintext(&self) -> String {
        self.root.serialize()
    }

    /// Encode the current tree into a game-loadable container.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn encode(&self) -> String {
        container::encode_container(&self.plaintext(), &self.prefix)
    }

    /// Encode a copy with identity fields redacted, for sharing. Errors if any
    /// redacted value still appears in the output (a mirrored field redaction
    /// failed to reach) — the same guard `save-dump --redact` uses.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn encode_redacted(&self) -> Result<(String, Vec<redact::Redaction>)> {
        let mut root = self.root.clone();
        let changes = redact::redact_identity(&mut root);
        let plaintext = root.serialize();
        let olds: Vec<&str> = changes.iter().map(|c| c.old.as_str()).collect();
        if !redact::residual_hits(&plaintext, &olds).is_empty() {
            bail!("refusing to write: a redacted identity value still appears in the output");
        }
        Ok((container::encode_container(&plaintext, &self.prefix), changes))
    }

    /// Round-trip safety check: a freshly-encoded save must decode and every
    /// pending edit must read back as its new value. Mirrors the post-write
    /// verification in the `save-edit` CLI.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn validate_encoded(&self, encoded: &str) -> Result<()> {
        let decoded = container::decode_container(encoded)
            .context("re-decoding the written save for validation")?;
        let root = raw::parse(&decoded.plaintext);
        for edit in &self.pending {
            let path: Vec<&str> = edit.path.iter().map(|s| s.as_str()).collect();
            let got = match root.get_path(&path) {
                Some(Raw::Scalar(s)) => Some(s.as_str()),
                _ => None,
            };
            if got != Some(edit.new.as_str()) {
                bail!(
                    "validation failed at {}: expected {:?}, got {:?}",
                    edit.path.join("."),
                    edit.new,
                    got
                );
            }
        }
        Ok(())
    }
}

/// Re-derive the typed model from a raw tree (serialize → tree-parse → extract).
fn derive(root: &Raw) -> Result<SaveFile> {
    let plaintext = root.serialize();
    let node = tree::parse(&plaintext);
    SaveFile::from_tree(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::container::encode_container;
    use save_parser::raw::{Field, Raw};

    /// A root with a base64-wrapped `p` block holding `j` (available god power),
    /// the way a real save nests structs. `serialize()` base64-encodes the block.
    fn sample_root() -> Raw {
        Raw::Struct(vec![
            (
                "p".to_string(),
                Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                    "j".to_string(),
                    Field::Value(Raw::Scalar("100".into())),
                )])))),
            ),
            ("c".to_string(), Field::Value(Raw::Scalar("1781053129".into()))),
        ])
    }

    fn encoded(root: &Raw) -> String {
        encode_container(&root.serialize(), "V2")
    }

    #[test]
    fn no_op_round_trips_byte_for_byte() {
        let root = sample_root();
        let session = EditSession::load(&encoded(&root), None).unwrap();
        assert_eq!(session.plaintext(), root.serialize());
        assert!(!session.is_dirty());
    }

    #[test]
    fn edit_records_and_coalesces() {
        let mut session = EditSession::load(&encoded(&sample_root()), None).unwrap();

        assert_eq!(session.value(&["p", "j"]).as_deref(), Some("100"));
        session.set_scalar(&["p", "j"], "God Power", "500").unwrap();
        assert_eq!(session.value(&["p", "j"]).as_deref(), Some("500"));
        assert_eq!(session.pending().len(), 1);
        assert_eq!(session.pending()[0].original, "100");
        assert_eq!(session.pending()[0].new, "500");

        // A second edit to the same path coalesces (still one entry, same origin).
        session.set_scalar(&["p", "j"], "God Power", "900").unwrap();
        assert_eq!(session.pending().len(), 1);
        assert_eq!(session.pending()[0].original, "100");
        assert_eq!(session.pending()[0].new, "900");

        // Editing back to the loaded value drops the entry.
        session.set_scalar(&["p", "j"], "God Power", "100").unwrap();
        assert!(session.pending().is_empty());
        assert!(!session.is_dirty());
    }

    #[test]
    fn undo_restores_loaded_value() {
        let mut session = EditSession::load(&encoded(&sample_root()), None).unwrap();
        session.set_scalar(&["p", "j"], "God Power", "777").unwrap();
        session.undo(0).unwrap();
        assert_eq!(session.value(&["p", "j"]).as_deref(), Some("100"));
        assert!(session.pending().is_empty());
    }

    #[test]
    fn encoded_output_validates_and_round_trips() {
        let mut session = EditSession::load(&encoded(&sample_root()), None).unwrap();
        session.set_scalar(&["p", "j"], "God Power", "777").unwrap();

        let out = session.encode();
        session.validate_encoded(&out).unwrap();
        // The decoded output carries the edit.
        let reloaded = EditSession::load(&out, None).unwrap();
        assert_eq!(reloaded.value(&["p", "j"]).as_deref(), Some("777"));
    }
}
