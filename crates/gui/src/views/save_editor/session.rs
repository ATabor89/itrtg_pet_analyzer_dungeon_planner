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
use save_parser::edit;
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

/// A newly-created equipment instance (not representable as a scalar edit, since
/// it appends a list element to `X.R`). Tracked alongside [`PendingEdit`] so it
/// shows in the pending panel and can be undone.
#[derive(Clone)]
pub struct AddedEquip {
    /// The assigned instance id (its `d`/`h`).
    pub instance_id: u32,
    /// Human label, e.g. "Magic Stick SSS+20".
    pub label: String,
    /// If equipped on creation: the pet slot path it set and that slot's original
    /// value (to restore on undo).
    pub slot: Option<(Vec<String>, String)>,
}

/// A newly-created material stack (appended to `X.Q`). Tracked like
/// [`AddedEquip`] so it shows in the pending panel and can be undone.
#[derive(Clone)]
pub struct AddedMaterial {
    pub item_id: u32,
    pub label: String,
}

/// A newly-created gem stack (appended to `X.002`), keyed by element + level.
#[derive(Clone)]
pub struct AddedGem {
    pub element_id: u32,
    pub level: u32,
    pub label: String,
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
    /// Equipment instances created this session (see [`AddedEquip`]).
    added: Vec<AddedEquip>,
    /// Material stacks created this session (see [`AddedMaterial`]).
    added_materials: Vec<AddedMaterial>,
    /// Gem stacks created this session (see [`AddedGem`]).
    added_gems: Vec<AddedGem>,
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
            added: Vec::new(),
            added_materials: Vec::new(),
            added_gems: Vec::new(),
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

    /// Equipment instances created this session.
    pub fn added(&self) -> &[AddedEquip] {
        &self.added
    }

    /// Material stacks created this session.
    pub fn added_materials(&self) -> &[AddedMaterial] {
        &self.added_materials
    }

    /// Gem stacks created this session.
    pub fn added_gems(&self) -> &[AddedGem] {
        &self.added_gems
    }

    pub fn is_dirty(&self) -> bool {
        !self.pending.is_empty()
            || !self.added.is_empty()
            || !self.added_materials.is_empty()
            || !self.added_gems.is_empty()
    }

    /// Total staged changes (scalar edits + created equipment + added items/gems).
    pub fn change_count(&self) -> usize {
        self.pending.len() + self.added.len() + self.added_materials.len() + self.added_gems.len()
    }

    /// Set the count of the gem stack (element, level) — upsert. Edits the
    /// existing `X.002` stack if present, else creates it. Returns whether a new
    /// stack was added.
    pub fn set_gem(
        &mut self,
        element_id: u32,
        level: u32,
        count: &str,
        label: impl Into<String>,
    ) -> Result<bool> {
        // Normalize a lone-struct X.002 into a real list (byte-identical) so the
        // index scan/path below are valid.
        edit::ensure_list(&mut self.root, "002")?;
        let idx = match self.root.get_path(&["X", "002"]) {
            Some(Raw::List(items)) => items.iter().position(|it| {
                scalar_u32(it, "a") == Some(element_id) && scalar_u32(it, "b") == Some(level)
            }),
            _ => None,
        };
        if let Some(idx) = idx {
            self.set_scalar(&["X", "002", &idx.to_string(), "c"], label, count)?;
            Ok(false)
        } else {
            edit::add_gem(&mut self.root, element_id, level, count)?;
            self.added_gems.push(AddedGem {
                element_id,
                level,
                label: label.into(),
            });
            self.dirty_derived = true;
            Ok(true)
        }
    }

    /// Undo a created gem stack (by index into [`added_gems`]).
    pub fn undo_added_gem(&mut self, index: usize) {
        let Some(entry) = self.added_gems.get(index).cloned() else {
            return;
        };
        let mut removed_pos = None;
        // `rposition` targets the appended (last) stack defensively, in case a
        // save ever shipped a duplicate (element, level) (shouldn't happen).
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "002"])
            && let Some(pos) = items.iter().rposition(|it| {
                scalar_u32(it, "a") == Some(entry.element_id)
                    && scalar_u32(it, "b") == Some(entry.level)
            })
        {
            items.remove(pos);
            removed_pos = Some(pos);
        }
        if let Some(pos) = removed_pos {
            self.reindex_pending_after_removal(&["X", "002"], pos);
        }
        self.added_gems.remove(index);
        self.dirty_derived = true;
    }

    /// Set the quantity of material `item_id` (upsert): edit the existing `X.Q`
    /// stack if present, else create one. Returns whether a new stack was added.
    pub fn set_material(
        &mut self,
        item_id: u32,
        count: &str,
        label: impl Into<String>,
    ) -> Result<bool> {
        let selector = format!("a={item_id}");
        let exists = self.root.get_path(&["X", "Q", &selector, "a"]).is_some();
        if exists {
            self.set_scalar(&["X", "Q", &selector, "b"], label, count)?;
            Ok(false)
        } else {
            edit::add_material(&mut self.root, item_id, count)?;
            self.added_materials.push(AddedMaterial {
                item_id,
                label: label.into(),
            });
            self.dirty_derived = true;
            Ok(true)
        }
    }

    /// Undo a created material stack (by index into [`added_materials`]): remove
    /// the `X.Q` element whose `a` matches, and drop any scalar edits that were
    /// staged against that stack (they'd otherwise dangle and fail validation).
    pub fn undo_added_material(&mut self, index: usize) {
        let Some(entry) = self.added_materials.get(index).cloned() else {
            return;
        };
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "Q"])
            && let Some(pos) = items.iter().position(|it| {
                matches!(it.get("a"), Some(Raw::Scalar(s)) if s.parse::<u32>().ok() == Some(entry.item_id))
            })
        {
            items.remove(pos);
        }
        // Quantity edits on this stack use the `a=<id>` selector path.
        let sel = format!("a={}", entry.item_id);
        self.drop_pending_with_prefix(&["X", "Q", &sel]);
        self.added_materials.remove(index);
        self.dirty_derived = true;
    }

    /// Drop pending scalar edits whose path begins with `prefix` (used when an
    /// undone addition removes a *selector-addressed* element, e.g. a material
    /// `X.Q.a=<id>`; other selector edits are unaffected by the removal).
    fn drop_pending_with_prefix(&mut self, prefix: &[&str]) {
        self.pending.retain(|e| {
            !(e.path.len() >= prefix.len() && e.path.iter().zip(prefix).all(|(a, b)| a == b))
        });
    }

    /// Fix index-addressed pending edits after removing element `removed_pos`
    /// from the list at `list_prefix` (e.g. `["X","R"]`): drop edits on the
    /// removed element and decrement the index of edits on later elements, which
    /// have shifted down by one. Keeps validation/round-trip consistent.
    fn reindex_pending_after_removal(&mut self, list_prefix: &[&str], removed_pos: usize) {
        let plen = list_prefix.len();
        self.pending.retain_mut(|e| {
            if e.path.len() > plen
                && e.path[..plen].iter().zip(list_prefix).all(|(a, b)| a == b)
                && let Ok(idx) = e.path[plen].parse::<usize>()
            {
                if idx == removed_pos {
                    return false;
                }
                if idx > removed_pos {
                    e.path[plen] = (idx - 1).to_string();
                }
            }
            true
        });
    }

    /// Create a new equipment instance (appended to `X.R`), optionally equipping
    /// it on a pet slot. Returns the assigned instance id. Tracked in [`added`]
    /// for the pending panel / undo.
    #[allow(clippy::too_many_arguments)]
    pub fn add_equipment(
        &mut self,
        type_id: u32,
        plus: u32,
        quality: u32,
        gem_level: u32,
        gem_element: u32,
        label: impl Into<String>,
        equip: Option<(usize, &str)>,
    ) -> Result<u32> {
        let id =
            edit::add_equip_instance(&mut self.root, type_id, plus, quality, gem_level, gem_element)?;
        let slot = match equip {
            Some((pet, slot_key)) => {
                let pet = pet.to_string();
                let path = vec!["X".to_string(), "b".to_string(), pet, "w".to_string(), slot_key.to_string()];
                let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
                let original = self.root.set_scalar_path(&p, &id.to_string())?;
                Some((path, original))
            }
            None => None,
        };
        self.added.push(AddedEquip {
            instance_id: id,
            label: label.into(),
            slot,
        });
        self.dirty_derived = true;
        Ok(id)
    }

    /// Undo a created equipment instance (by index into [`added`]): remove its
    /// `X.R` element and restore any slot it was equipped into.
    pub fn undo_added(&mut self, index: usize) {
        let Some(entry) = self.added.get(index).cloned() else {
            return;
        };
        // Restore the pet slot first (if equipped).
        if let Some((path, original)) = &entry.slot {
            let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
            let _ = self.root.set_scalar_path(&p, original);
        }
        // Remove the X.R element whose mirror id `h` matches.
        let mut removed_pos = None;
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "R"])
            && let Some(pos) = items.iter().position(|it| {
                matches!(it.get("h"), Some(Raw::Scalar(s)) if s.parse::<u32>().ok() == Some(entry.instance_id))
            })
        {
            items.remove(pos);
            removed_pos = Some(pos);
        }
        // Re-index pending edits: drop edits on the removed instance and shift
        // edits on later instances down one (X.R is index-addressed).
        if let Some(pos) = removed_pos {
            self.reindex_pending_after_removal(&["X", "R"], pos);
        }
        self.added.remove(index);
        self.dirty_derived = true;
    }

    /// The canonical tree, for read-only traversal (the raw tree navigator).
    /// Edits must still go through [`set_scalar`](Self::set_scalar).
    pub fn root(&self) -> &Raw {
        &self.root
    }

    /// Whether `path` resolves to any node (scalar or container). Used to check
    /// registry coverage, including block-level labels that aren't scalars.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn path_exists(&self, path: &[&str]) -> bool {
        self.root.get_path(path).is_some()
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
        self.validate_filtered(encoded, false)
    }

    /// Like [`validate_encoded`](Self::validate_encoded), but for a redacted
    /// copy: pending edits to root identity fields are skipped, since redaction
    /// intentionally overwrites those with placeholders (a user could have
    /// edited e.g. `W` in the raw tree, and the redacted copy must still win).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn validate_encoded_redacted(&self, encoded: &str) -> Result<()> {
        self.validate_filtered(encoded, true)
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn validate_filtered(&self, encoded: &str, skip_identity: bool) -> Result<()> {
        let decoded = container::decode_container(encoded)
            .context("re-decoding the written save for validation")?;
        let root = raw::parse(&decoded.plaintext);
        for edit in &self.pending {
            if skip_identity && is_identity_path(&edit.path) {
                continue;
            }
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
        // Each created instance must be present in the re-decoded X.R.
        for a in &self.added {
            let present = matches!(root.get_path(&["X", "R"]), Some(Raw::List(items))
                if items.iter().any(|it| matches!(it.get("h"),
                    Some(Raw::Scalar(s)) if s.parse::<u32>().ok() == Some(a.instance_id))));
            if !present {
                bail!("validation failed: created item #{} missing after round-trip", a.instance_id);
            }
        }
        // Each created material stack must be present in X.Q.
        for m in &self.added_materials {
            if root.get_path(&["X", "Q", &format!("a={}", m.item_id), "a"]).is_none() {
                bail!("validation failed: added item id {} missing after round-trip", m.item_id);
            }
        }
        // Each created gem stack must be present in X.002 (matching element+level).
        for g in &self.added_gems {
            let present = matches!(root.get_path(&["X", "002"]), Some(Raw::List(items))
                if items.iter().any(|it| scalar_u32(it, "a") == Some(g.element_id)
                    && scalar_u32(it, "b") == Some(g.level)));
            if !present {
                bail!(
                    "validation failed: added gem (element {}, level {}) missing after round-trip",
                    g.element_id,
                    g.level
                );
            }
        }
        Ok(())
    }
}

/// A struct field read as a `u32` (peeling base64), if it's a numeric scalar.
fn scalar_u32(node: &Raw, key: &str) -> Option<u32> {
    match node.get(key) {
        Some(Raw::Scalar(s)) => s.parse().ok(),
        _ => None,
    }
}

/// Is this a single-segment path naming a root identity field (`W`, `s`,
/// `001`–`004`)? Those are intentionally overwritten by redaction.
fn is_identity_path(path: &[String]) -> bool {
    path.len() == 1
        && redact::IDENTITY_FIELDS
            .iter()
            .any(|(key, _)| *key == path[0])
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

    fn sc(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }
    fn b64(r: Raw) -> Field {
        Field::Value(Raw::Base64(Box::new(r)))
    }
    /// An equipment instance with id `d` (mirrored in `h`).
    fn instance(d: &str) -> Raw {
        Raw::Struct(vec![("a".into(), sc("51")), ("d".into(), sc(d)), ("h".into(), sc(d))])
    }
    /// A pet whose weapon slot `w.e` holds `e`.
    fn pet(e: &str) -> Raw {
        Raw::Struct(vec![(
            "w".into(),
            b64(Raw::Struct(vec![
                ("e".into(), sc(e)),
                ("f".into(), sc("0")),
                ("g".into(), sc("0")),
            ])),
        )])
    }
    /// A root with X.R (two instances, d=5,8) and X.b (two pets) — two elements
    /// each so the lists don't collapse to a lone struct.
    fn equip_root() -> Raw {
        let x = Raw::Struct(vec![
            ("R".into(), Field::Value(Raw::List(vec![instance("5"), instance("8")]))),
            ("b".into(), Field::Value(Raw::List(vec![pet("0"), pet("0")]))),
        ]);
        Raw::Struct(vec![("X".into(), b64(x))])
    }

    /// A material stack `{a:id, b:count}`.
    fn mat(id: &str, count: &str) -> Raw {
        Raw::Struct(vec![("a".into(), sc(id)), ("b".into(), sc(count))])
    }
    /// A root with X.Q holding two material stacks.
    fn mat_root() -> Raw {
        let x = Raw::Struct(vec![(
            "Q".into(),
            Field::Value(Raw::List(vec![mat("117", "10"), mat("159", "20")])),
        )]);
        Raw::Struct(vec![("X".into(), b64(x))])
    }

    #[test]
    fn set_material_updates_existing_stack() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        let created = s.set_material(117, "500", "Ant").unwrap();
        assert!(!created);
        assert_eq!(s.value(&["X", "Q", "a=117", "b"]).as_deref(), Some("500"));
        assert_eq!(s.pending().len(), 1); // a scalar edit, not an addition
        assert!(s.added_materials().is_empty());
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn set_material_creates_new_stack_and_undoes() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        let created = s.set_material(999, "5", "Item 999").unwrap();
        assert!(created);
        assert_eq!(s.added_materials().len(), 1);
        assert_eq!(s.value(&["X", "Q", "a=999", "b"]).as_deref(), Some("5"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        s.undo_added_material(0);
        assert!(s.added_materials().is_empty());
        assert!(s.value(&["X", "Q", "a=999", "b"]).is_none());
    }

    /// A gem stack `{a:element, b:level, c:count}`.
    fn gem(element: &str, level: &str, count: &str) -> Raw {
        Raw::Struct(vec![("a".into(), sc(element)), ("b".into(), sc(level)), ("c".into(), sc(count))])
    }
    /// A root with X.002 holding two gem stacks (Fire L1, Fire L10).
    fn gem_root() -> Raw {
        let x = Raw::Struct(vec![(
            "002".into(),
            Field::Value(Raw::List(vec![gem("1", "1", "5"), gem("1", "10", "2")])),
        )]);
        Raw::Struct(vec![("X".into(), b64(x))])
    }

    #[test]
    fn set_gem_updates_existing_and_creates_new() {
        let mut s = EditSession::load(&encoded(&gem_root()), None).unwrap();
        // Update Fire L1 (index 0).
        assert!(!s.set_gem(1, 1, "50", "Fire L1").unwrap());
        assert_eq!(s.value(&["X", "002", "0", "c"]).as_deref(), Some("50"));
        assert!(s.added_gems().is_empty());
        // Create Water L5 (new stack, appended at index 2).
        assert!(s.set_gem(2, 5, "3", "Water L5").unwrap());
        assert_eq!(s.added_gems().len(), 1);
        assert_eq!(s.value(&["X", "002", "2", "a"]).as_deref(), Some("2"));
        assert_eq!(s.value(&["X", "002", "2", "b"]).as_deref(), Some("5"));
        assert_eq!(s.value(&["X", "002", "2", "c"]).as_deref(), Some("3"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        s.undo_added_gem(0);
        assert!(s.added_gems().is_empty());
        assert!(s.value(&["X", "002", "2", "a"]).is_none());
    }

    #[test]
    fn undo_added_gem_reindexes_dependent_pending() {
        let mut s = EditSession::load(&encoded(&gem_root()), None).unwrap();
        s.set_gem(2, 5, "3", "Water L5").unwrap(); // A → idx 2
        s.set_gem(3, 3, "4", "Earth L3").unwrap(); // B → idx 3
        // Edit B's count (stages a pending edit at X.002.3.c).
        s.set_gem(3, 3, "99", "Earth L3 count").unwrap();
        assert_eq!(s.pending().len(), 1);

        // Undo A. B shifts from idx 3 → idx 2; its pending edit must follow.
        s.undo_added_gem(0);
        assert_eq!(s.added_gems().len(), 1);
        assert_eq!(s.value(&["X", "002", "2", "a"]).as_deref(), Some("3")); // B (Earth) now idx 2
        assert_eq!(s.value(&["X", "002", "2", "c"]).as_deref(), Some("99"));
        // Validates — the re-indexed pending edit resolves, not dangling.
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn undo_added_material_drops_dependent_pending_edit() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        s.set_material(999, "5", "Item 999").unwrap(); // creates the stack
        // Then edit its quantity (as the inventory page's Apply would).
        s.set_scalar(&["X", "Q", "a=999", "b"], "Item 999", "50").unwrap();
        assert_eq!(s.pending().len(), 1);

        s.undo_added_material(0);
        assert!(s.added_materials().is_empty());
        assert_eq!(s.pending().len(), 0, "dependent edit dropped, not dangling");
        // The save still validates (no edit pointing at the removed stack).
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn add_equipment_appends_with_fresh_id_and_undoes() {
        let mut s = EditSession::load(&encoded(&equip_root()), None).unwrap();
        let id = s.add_equipment(51, 20, 8, 3, 1, "Magic Stick SSS+20", None).unwrap();
        assert_eq!(id, 9); // max d (8) + 1
        assert_eq!(s.added().len(), 1);
        assert!(s.is_dirty());
        // New element at X.R[2] with the gem fields.
        assert_eq!(s.value(&["X", "R", "2", "a"]).as_deref(), Some("51"));
        assert_eq!(s.value(&["X", "R", "2", "h"]).as_deref(), Some("9"));
        assert_eq!(s.value(&["X", "R", "2", "f"]).as_deref(), Some("3"));
        assert_eq!(s.value(&["X", "R", "2", "g"]).as_deref(), Some("1"));

        // Round-trips and validates.
        let out = s.encode();
        s.validate_encoded(&out).unwrap();

        s.undo_added(0);
        assert!(s.added().is_empty());
        assert!(s.value(&["X", "R", "2", "a"]).is_none()); // removed
    }

    #[test]
    fn add_equipment_equips_pet_and_restores_on_undo() {
        let mut s = EditSession::load(&encoded(&equip_root()), None).unwrap();
        let id = s
            .add_equipment(51, 0, 8, 0, 0, "Magic Stick", Some((0, "e")))
            .unwrap();
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]), Some(id.to_string()));
        s.undo_added(0);
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("0")); // restored
        assert!(s.value(&["X", "R", "2", "a"]).is_none()); // instance removed
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
    fn redacted_validation_tolerates_an_edited_identity_field() {
        // A root with an identity field `W` (god name) plus `p.j`.
        let root = Raw::Struct(vec![
            ("W".to_string(), Field::Value(Raw::Scalar("RealGodName".into()))),
            (
                "p".to_string(),
                Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                    "j".to_string(),
                    Field::Value(Raw::Scalar("100".into())),
                )])))),
            ),
        ]);
        let mut session = EditSession::load(&encoded(&root), None).unwrap();

        // User edits the identity field in the raw tree, then exports a redacted
        // copy. Redaction overwrites `W` with a placeholder, so the plain
        // validator would (correctly) refuse — the redacted validator must not.
        session.set_scalar(&["W"], "God Name", "EditedName").unwrap();
        let (enc, _changes) = session.encode_redacted().unwrap();
        assert!(session.validate_encoded(&enc).is_err());
        session.validate_encoded_redacted(&enc).unwrap();

        // The redacted output really carries the placeholder, not either name.
        let reloaded = EditSession::load(&enc, None).unwrap();
        assert_eq!(reloaded.value(&["W"]).as_deref(), Some("RedactedGod"));
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
