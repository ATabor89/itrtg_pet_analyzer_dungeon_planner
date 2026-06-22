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

/// A newly-created challenge-completion entry (appended to `x.242`), keyed by
/// challenge id.
#[derive(Clone)]
pub struct AddedChallenge {
    pub challenge_id: u32,
    pub label: String,
}

/// A newly-created adventure-inventory item (appended to `032.d`), keyed by id.
#[derive(Clone)]
pub struct AddedAdventureItem {
    pub item_id: u32,
    pub label: String,
}

/// A newly-created adventure core (appended to `032.G`), keyed by enemy id.
#[derive(Clone)]
pub struct AddedCore {
    pub enemy_id: u32,
    pub label: String,
}

/// A loaded list element removed this session, kept so the deletion shows in the
/// pending panel and can be undone (re-inserted).
#[derive(Clone)]
pub struct RemovedElement {
    /// The list it came from (`["X","R"]` / `["X","Q"]` / `["X","002"]`).
    pub list: Vec<&'static str>,
    /// The removed node, for re-insertion on undo.
    pub element: Raw,
    pub label: String,
    /// Pet slots that referenced a removed equipment instance (cleared to `0` on
    /// delete, restored on undo) — so nothing dangles.
    cleared_slots: Vec<(Vec<String>, String)>,
}

/// A whole-subtree replacement staged this session (the inverse of the tree's
/// "Copy node (raw)"): the node at `path` was swapped for pasted raw text. The
/// original node is kept so the paste can be undone. Validated by a full encode
/// round-trip before it is ever recorded, so a recorded `TreeEdit` always
/// re-encodes cleanly.
#[derive(Clone)]
pub struct TreeEdit {
    /// Dotted (all-index) path of the replaced node.
    pub path: Vec<String>,
    /// The node as it was before the paste, for undo.
    original: Raw,
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
    /// Challenge entries created this session (see [`AddedChallenge`]).
    added_challenges: Vec<AddedChallenge>,
    /// Adventure-inventory items created this session.
    added_adventure_items: Vec<AddedAdventureItem>,
    /// Adventure cores created this session.
    added_cores: Vec<AddedCore>,
    /// Loaded list elements deleted this session (see [`RemovedElement`]).
    removed: Vec<RemovedElement>,
    /// Whole-subtree pastes staged this session (see [`TreeEdit`]).
    tree_edits: Vec<TreeEdit>,
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
            added_challenges: Vec::new(),
            added_adventure_items: Vec::new(),
            added_cores: Vec::new(),
            removed: Vec::new(),
            tree_edits: Vec::new(),
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

    /// Challenge entries created this session.
    pub fn added_challenges(&self) -> &[AddedChallenge] {
        &self.added_challenges
    }

    /// Adventure-inventory items created this session.
    pub fn added_adventure_items(&self) -> &[AddedAdventureItem] {
        &self.added_adventure_items
    }

    /// Adventure cores created this session.
    pub fn added_cores(&self) -> &[AddedCore] {
        &self.added_cores
    }

    /// Loaded list elements deleted this session.
    pub fn removed(&self) -> &[RemovedElement] {
        &self.removed
    }

    pub fn tree_edits(&self) -> &[TreeEdit] {
        &self.tree_edits
    }

    pub fn is_dirty(&self) -> bool {
        !self.pending.is_empty()
            || !self.added.is_empty()
            || !self.added_materials.is_empty()
            || !self.added_gems.is_empty()
            || !self.added_challenges.is_empty()
            || !self.added_adventure_items.is_empty()
            || !self.added_cores.is_empty()
            || !self.removed.is_empty()
            || !self.tree_edits.is_empty()
    }

    /// Total staged changes (scalar edits + created/added + deleted + pastes).
    pub fn change_count(&self) -> usize {
        self.pending.len()
            + self.added.len()
            + self.added_materials.len()
            + self.added_gems.len()
            + self.added_challenges.len()
            + self.added_adventure_items.len()
            + self.added_cores.len()
            + self.removed.len()
            + self.tree_edits.len()
    }

    /// Replace the whole subtree at `path` with `new_text` (raw save text — the
    /// inverse of the tree's "Copy node (raw)"). The candidate is applied, then
    /// the *entire* save is re-encoded and round-trip validated; if that fails
    /// the original node is restored and an error is returned, so a malformed
    /// paste can never corrupt the staged save. `path` must name an existing,
    /// non-root node.
    pub fn replace_node(&mut self, path: &[&str], new_text: &str) -> Result<()> {
        if path.is_empty() {
            anyhow::bail!("cannot replace the root node");
        }
        let candidate = raw::parse(new_text.trim());
        let node = self
            .root
            .get_path_mut(path)
            .ok_or_else(|| anyhow::anyhow!("path not found: {}", path.join(".")))?;
        let original = node.clone();
        *node = candidate;
        // Validate the whole save still round-trips; revert on failure.
        let encoded = self.encode();
        if let Err(e) = self.validate_encoded(&encoded) {
            if let Some(n) = self.root.get_path_mut(path) {
                *n = original;
            }
            return Err(anyhow::anyhow!("pasted subtree does not round-trip: {e}"));
        }
        self.tree_edits.push(TreeEdit {
            path: path.iter().map(|s| s.to_string()).collect(),
            original,
        });
        self.dirty_derived = true;
        Ok(())
    }

    /// Undo a staged subtree paste, restoring the original node.
    pub fn undo_tree_edit(&mut self, index: usize) {
        if index >= self.tree_edits.len() {
            return;
        }
        let entry = self.tree_edits.remove(index);
        let p: Vec<&str> = entry.path.iter().map(String::as_str).collect();
        if let Some(node) = self.root.get_path_mut(&p) {
            *node = entry.original;
        }
        self.dirty_derived = true;
    }

    /// Delete the equipment instance at `X.R.<index>`. If it's a session-created
    /// instance, this just undoes its creation; otherwise it clears the slot of
    /// any pet equipping it (so nothing dangles) and tracks the removal for undo.
    pub fn delete_equipment(&mut self, index: usize, label: impl Into<String>) -> Result<()> {
        edit::ensure_list(&mut self.root, "R")?;
        let element = self
            .root
            .get_path(&["X", "R", &index.to_string()])
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no equipment at index {index}"))?;
        let h = scalar_u32(&element, "h");
        let d = scalar_u32(&element, "d");

        // A session-created instance → undo its creation (restores any slot it set).
        if let Some(h) = h
            && let Some(ai) = self.added.iter().position(|a| a.instance_id == h)
        {
            self.undo_added(ai);
            return Ok(());
        }

        // Clear pet slots referencing this loaded instance. Slots hold the equip
        // id `d` (not the catalog id `h`), so match on `d` — matching `h` too
        // could wrongly clear a slot that points at a *different* item whose `d`
        // equals this item's `h` (the cross-field collision; see
        // `resolve_equipment_instance`).
        let mut cleared = Vec::new();
        let pet_count = match self.root.get_path(&["X", "b"]) {
            Some(Raw::List(p)) => p.len(),
            Some(Raw::Struct(_)) => 1, // a 1-element list re-parses as a lone struct
            _ => 0,
        };
        for pi in 0..pet_count {
            for slot in ["e", "f", "g"] {
                let path = vec![
                    "X".to_string(),
                    "b".to_string(),
                    pi.to_string(),
                    "w".to_string(),
                    slot.to_string(),
                ];
                let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
                let cur = match self.root.get_path(&p) {
                    Some(Raw::Scalar(s)) => s.parse::<u32>().ok(),
                    _ => None,
                };
                if let Some(c) = cur
                    && c != 0
                    && Some(c) == d
                {
                    let orig = self.root.set_scalar_path(&p, "0")?;
                    // Drop any pending edit on this slot (now cleared) so it
                    // doesn't conflict; undo restores `orig` directly.
                    self.drop_pending_with_prefix(&p);
                    cleared.push((path.clone(), orig));
                }
            }
        }

        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "R"])
            && index < items.len()
        {
            items.remove(index);
        }
        self.reindex_pending_after_removal(&["X", "R"], index);
        self.removed.push(RemovedElement {
            list: vec!["X", "R"],
            element,
            label: label.into(),
            cleared_slots: cleared,
        });
        self.dirty_derived = true;
        Ok(())
    }

    /// Delete the material stack at `X.Q.<index>` (or undo its creation if it was
    /// added this session).
    pub fn delete_material(&mut self, index: usize, label: impl Into<String>) -> Result<()> {
        edit::ensure_list(&mut self.root, "Q")?;
        let element = self
            .root
            .get_path(&["X", "Q", &index.to_string()])
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no material at index {index}"))?;
        let item_id = scalar_u32(&element, "a");
        if let Some(id) = item_id
            && let Some(ai) = self.added_materials.iter().position(|m| m.item_id == id)
        {
            self.undo_added_material(ai);
            return Ok(());
        }
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "Q"])
            && index < items.len()
        {
            items.remove(index);
        }
        // Material edits are selector-addressed; drop just this stack's edits.
        if let Some(id) = item_id {
            self.drop_pending_with_prefix(&["X", "Q", &format!("a={id}")]);
        }
        self.removed.push(RemovedElement {
            list: vec!["X", "Q"],
            element,
            label: label.into(),
            cleared_slots: Vec::new(),
        });
        self.dirty_derived = true;
        Ok(())
    }

    /// Delete the gem stack at `X.002.<index>` (or undo its creation if added
    /// this session).
    pub fn delete_gem(&mut self, index: usize, label: impl Into<String>) -> Result<()> {
        edit::ensure_list(&mut self.root, "002")?;
        let element = self
            .root
            .get_path(&["X", "002", &index.to_string()])
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no gem at index {index}"))?;
        let el = scalar_u32(&element, "a");
        let lv = scalar_u32(&element, "b");
        if let (Some(e), Some(l)) = (el, lv)
            && let Some(ai) = self.added_gems.iter().position(|g| g.element_id == e && g.level == l)
        {
            self.undo_added_gem(ai);
            return Ok(());
        }
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["X", "002"])
            && index < items.len()
        {
            items.remove(index);
        }
        self.reindex_pending_after_removal(&["X", "002"], index);
        self.removed.push(RemovedElement {
            list: vec!["X", "002"],
            element,
            label: label.into(),
            cleared_slots: Vec::new(),
        });
        self.dirty_derived = true;
        Ok(())
    }

    /// Undo a deletion (by index into [`removed`]): re-insert the element (at the
    /// end — order is content-addressed, not positional) and restore any pet
    /// slots that were cleared.
    pub fn undo_removed(&mut self, index: usize) {
        let Some(entry) = self.removed.get(index).cloned() else {
            return;
        };
        if let Some(Raw::List(items)) = self.root.get_path_mut(&entry.list) {
            items.push(entry.element);
        }
        for (path, orig) in &entry.cleared_slots {
            let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
            let _ = self.root.set_scalar_path(&p, orig);
        }
        self.removed.remove(index);
        self.dirty_derived = true;
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

    /// Upsert a challenge completion (`x.242`): if the challenge id is already
    /// present, set its completion count `b`; otherwise append a new entry
    /// `{a:id, b:count, c:difficulty, d:0, e:0}`. Returns whether a new entry
    /// was added. (The game keys challenges by id and recomputes ChP from the
    /// list, so we upsert rather than allow duplicates.)
    pub fn set_challenge(
        &mut self,
        challenge_id: u32,
        completions: &str,
        difficulty: u32,
        label: impl Into<String>,
    ) -> Result<bool> {
        // Normalize / create the x.242 list so the index path below is valid.
        edit::ensure_list_at(&mut self.root, &["x", "242"])?;
        let idx = match self.root.get_path(&["x", "242"]) {
            Some(Raw::List(items)) => {
                items.iter().position(|it| scalar_u32(it, "a") == Some(challenge_id))
            }
            _ => None,
        };
        if let Some(idx) = idx {
            self.set_scalar(&["x", "242", &idx.to_string(), "b"], label, completions)?;
            Ok(false)
        } else {
            edit::add_challenge_entry(&mut self.root, challenge_id, completions, difficulty)?;
            self.added_challenges.push(AddedChallenge {
                challenge_id,
                label: label.into(),
            });
            self.dirty_derived = true;
            Ok(true)
        }
    }

    /// Undo a created challenge entry (by index into [`added_challenges`]).
    pub fn undo_added_challenge(&mut self, index: usize) {
        let Some(entry) = self.added_challenges.get(index).cloned() else {
            return;
        };
        let mut removed_pos = None;
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["x", "242"])
            && let Some(pos) =
                items.iter().rposition(|it| scalar_u32(it, "a") == Some(entry.challenge_id))
        {
            items.remove(pos);
            removed_pos = Some(pos);
        }
        if let Some(pos) = removed_pos {
            self.reindex_pending_after_removal(&["x", "242"], pos);
        }
        self.added_challenges.remove(index);
        self.dirty_derived = true;
    }

    /// Upsert an adventure-inventory item (`032.d`): set the existing item's
    /// count `b` if the id is present, else append a new entry. Returns whether a
    /// new entry was added.
    pub fn set_adventure_item(
        &mut self,
        item_id: u32,
        count: &str,
        label: impl Into<String>,
    ) -> Result<bool> {
        edit::ensure_list_at(&mut self.root, &["032", "d"])?;
        let idx = match self.root.get_path(&["032", "d"]) {
            Some(Raw::List(items)) => items.iter().position(|it| scalar_u32(it, "a") == Some(item_id)),
            _ => None,
        };
        if let Some(idx) = idx {
            self.set_scalar(&["032", "d", &idx.to_string(), "b"], label, count)?;
            Ok(false)
        } else {
            edit::add_adventure_item(&mut self.root, item_id, count)?;
            self.added_adventure_items.push(AddedAdventureItem { item_id, label: label.into() });
            self.dirty_derived = true;
            Ok(true)
        }
    }

    /// Undo a created adventure item (by index into [`added_adventure_items`]).
    pub fn undo_added_adventure_item(&mut self, index: usize) {
        let Some(entry) = self.added_adventure_items.get(index).cloned() else {
            return;
        };
        let mut removed_pos = None;
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["032", "d"])
            && let Some(pos) = items.iter().rposition(|it| scalar_u32(it, "a") == Some(entry.item_id))
        {
            items.remove(pos);
            removed_pos = Some(pos);
        }
        if let Some(pos) = removed_pos {
            self.reindex_pending_after_removal(&["032", "d"], pos);
        }
        self.added_adventure_items.remove(index);
        self.dirty_derived = true;
    }

    /// Upsert an adventure core (`032.G`): set the existing core's count `c` and
    /// quality `d` if the enemy id is present, else append a new entry. Returns
    /// whether a new entry was added.
    pub fn set_core(
        &mut self,
        enemy_id: u32,
        count: &str,
        quality: u32,
        label: impl Into<String>,
    ) -> Result<bool> {
        edit::ensure_list_at(&mut self.root, &["032", "G"])?;
        let idx = match self.root.get_path(&["032", "G"]) {
            Some(Raw::List(items)) => items.iter().position(|it| scalar_u32(it, "a") == Some(enemy_id)),
            _ => None,
        };
        if let Some(idx) = idx {
            let label = label.into();
            let i = idx.to_string();
            self.set_scalar(&["032", "G", &i, "c"], label.clone(), count)?;
            self.set_scalar(&["032", "G", &i, "d"], label, &quality.to_string())?;
            Ok(false)
        } else {
            edit::add_core(&mut self.root, enemy_id, count, quality)?;
            self.added_cores.push(AddedCore { enemy_id, label: label.into() });
            self.dirty_derived = true;
            Ok(true)
        }
    }

    /// Undo a created core (by index into [`added_cores`]).
    pub fn undo_added_core(&mut self, index: usize) {
        let Some(entry) = self.added_cores.get(index).cloned() else {
            return;
        };
        let mut removed_pos = None;
        if let Some(Raw::List(items)) = self.root.get_path_mut(&["032", "G"])
            && let Some(pos) = items.iter().rposition(|it| scalar_u32(it, "a") == Some(entry.enemy_id))
        {
            items.remove(pos);
            removed_pos = Some(pos);
        }
        if let Some(pos) = removed_pos {
            self.reindex_pending_after_removal(&["032", "G"], pos);
        }
        self.added_cores.remove(index);
        self.dirty_derived = true;
    }

    /// Equip an existing inventory equipment instance (`X.R.<equip_index>`,
    /// normally `d`=0) onto a pet's slot (`X.b.<pet_index>.w.<slot_key>`, slot_key
    /// = `e`/`f`/`g`). Mints a fresh equip-ref `d` = max(d)+1 (collision-free,
    /// like the builder), and sets both the item's `d` and the pet slot to it. If
    /// the slot already holds an item, that item is returned to inventory first
    /// (its `d` → 0). All staged as scalar edits (individually undoable).
    pub fn equip_existing(
        &mut self,
        equip_index: usize,
        pet_index: usize,
        slot_key: &str,
        label: impl Into<String>,
    ) -> Result<()> {
        let label = label.into();
        let new_d = edit::max_instance_id(&self.root) + 1;
        let pet = pet_index.to_string();
        let slot_path = ["X", "b", &pet, "w", slot_key];
        // Swap: find the item currently in the slot (if any), to unequip it.
        let displaced: Option<usize> = match self.root.get_path(&slot_path) {
            Some(Raw::Scalar(s)) => s
                .parse::<u32>()
                .ok()
                .filter(|&c| c != 0)
                .and_then(|cur| match self.root.get_path(&["X", "R"]) {
                    Some(Raw::List(items)) => {
                        items.iter().position(|it| scalar_u32(it, "d") == Some(cur))
                    }
                    _ => None,
                }),
            _ => None,
        };
        if let Some(idx) = displaced {
            self.set_scalar(
                &["X", "R", &idx.to_string(), "d"],
                format!("{label} (unequip displaced)"),
                "0",
            )?;
        }
        let ei = equip_index.to_string();
        self.set_scalar(&["X", "R", &ei, "d"], label.clone(), &new_d.to_string())?;
        self.set_scalar(&slot_path, label, &new_d.to_string())?;
        Ok(())
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
        // Each created challenge entry must be present in x.242 (by challenge id).
        // Use the `a=<id>` selector path so a single-entry list — which re-parses
        // as a lone struct after a round-trip — still resolves (cf. materials).
        for c in &self.added_challenges {
            if root
                .get_path(&["x", "242", &format!("a={}", c.challenge_id), "a"])
                .is_none()
            {
                bail!(
                    "validation failed: added challenge id {} missing after round-trip",
                    c.challenge_id
                );
            }
        }
        // Each created adventure item must be present in 032.d (by item id).
        for a in &self.added_adventure_items {
            if root.get_path(&["032", "d", &format!("a={}", a.item_id), "a"]).is_none() {
                bail!(
                    "validation failed: added adventure item id {} missing after round-trip",
                    a.item_id
                );
            }
        }
        // Each created core must be present in 032.G (by enemy id).
        for c in &self.added_cores {
            if root.get_path(&["032", "G", &format!("a={}", c.enemy_id), "a"]).is_none() {
                bail!(
                    "validation failed: added core enemy id {} missing after round-trip",
                    c.enemy_id
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
    fn replace_node_swaps_subtree_round_trips_and_undoes() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        // Replace the first material element's whole subtree via raw text.
        s.replace_node(&["X", "Q", "0"], "a:117;b:9999;").unwrap();
        assert_eq!(s.value(&["X", "Q", "0", "b"]).as_deref(), Some("9999"));
        assert_eq!(s.tree_edits().len(), 1);
        assert!(s.is_dirty());
        // The whole save still round-trips.
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        // Undo restores the original element.
        s.undo_tree_edit(0);
        assert!(s.tree_edits().is_empty());
        assert_eq!(s.value(&["X", "Q", "0", "b"]).as_deref(), Some("10"));
    }

    #[test]
    fn replace_node_rejects_root_and_missing_path() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        assert!(s.replace_node(&[], "x").is_err());
        assert!(s.replace_node(&["X", "Q", "99"], "a:1;b:1;").is_err());
        assert!(s.tree_edits().is_empty());
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

    /// A challenge entry `{a:id, b:completions, c:0, d:0, e:0}`.
    fn chal(a: &str, b: &str) -> Raw {
        Raw::Struct(vec![
            ("a".into(), sc(a)),
            ("b".into(), sc(b)),
            ("c".into(), sc("0")),
            ("d".into(), sc("0")),
            ("e".into(), sc("0")),
        ])
    }
    /// A root with the Statistics block `x.242` holding two challenge entries
    /// (AAC id 10, DRC id 3).
    fn chal_root() -> Raw {
        let x = Raw::Struct(vec![(
            "242".into(),
            Field::Value(Raw::List(vec![chal("10", "10"), chal("3", "8")])),
        )]);
        Raw::Struct(vec![("x".into(), b64(x))])
    }

    #[test]
    fn set_challenge_updates_existing_and_creates_new() {
        let mut s = EditSession::load(&encoded(&chal_root()), None).unwrap();
        // Update existing AAC (id 10, index 0) — a scalar edit, not an addition.
        assert!(!s.set_challenge(10, "25", 0, "AAC").unwrap());
        assert_eq!(s.value(&["x", "242", "0", "b"]).as_deref(), Some("25"));
        assert!(s.added_challenges().is_empty());
        // Create a new challenge (UUC id 1, appended at index 2).
        assert!(s.set_challenge(1, "2", 0, "UUC").unwrap());
        assert_eq!(s.added_challenges().len(), 1);
        assert_eq!(s.value(&["x", "242", "2", "a"]).as_deref(), Some("1"));
        assert_eq!(s.value(&["x", "242", "2", "b"]).as_deref(), Some("2"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        // Undo the created entry — it leaves the tree.
        s.undo_added_challenge(0);
        assert!(s.added_challenges().is_empty());
        assert!(s.value(&["x", "242", "2", "a"]).is_none());
    }

    #[test]
    fn set_challenge_creates_x242_list_when_absent() {
        // Statistics block with no 242 field at all.
        let root = Raw::Struct(vec![("x".into(), b64(Raw::Struct(vec![("013".into(), sc("0"))])))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        assert!(s.set_challenge(10, "5", 0, "AAC").unwrap());
        assert_eq!(s.value(&["x", "242", "0", "a"]).as_deref(), Some("10"));
        assert_eq!(s.value(&["x", "242", "0", "b"]).as_deref(), Some("5"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn delete_equipment_clears_equipped_pet_slot_and_undo_restores() {
        // Pet 0 has weapon = instance id 5 (loaded), instances are d=5,8.
        let x = Raw::Struct(vec![
            ("R".into(), Field::Value(Raw::List(vec![instance("5"), instance("8")]))),
            ("b".into(), Field::Value(Raw::List(vec![pet("5"), pet("0")]))),
        ]);
        let root = Raw::Struct(vec![("X".into(), b64(x))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("5"));

        s.delete_equipment(0, "Magic Stick").unwrap(); // X.R[0] has h=5
        assert_eq!(s.removed().len(), 1);
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("0")); // cleared
        assert_eq!(s.value(&["X", "R", "0", "d"]).as_deref(), Some("8")); // d=5 gone
        let out = s.encode();
        s.validate_encoded(&out).unwrap();

        s.undo_removed(0);
        assert!(s.removed().is_empty());
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("5")); // restored
    }

    #[test]
    fn set_adventure_item_updates_and_creates() {
        let adv = Raw::Struct(vec![(
            "d".into(),
            Field::Value(Raw::List(vec![Raw::Struct(vec![
                ("a".into(), sc("1")),
                ("b".into(), sc("100")),
                ("c".into(), sc("0")),
                ("d".into(), sc("0")),
            ])])),
        )]);
        let root = Raw::Struct(vec![("032".into(), b64(adv))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        // Update existing item 1 (index 0) — a scalar edit, not an addition.
        assert!(!s.set_adventure_item(1, "500", "Item 1").unwrap());
        assert_eq!(s.value(&["032", "d", "0", "b"]).as_deref(), Some("500"));
        assert!(s.added_adventure_items().is_empty());
        // Add a new item 50 (appended at index 1).
        assert!(s.set_adventure_item(50, "7", "Item 50").unwrap());
        assert_eq!(s.added_adventure_items().len(), 1);
        assert_eq!(s.value(&["032", "d", "1", "a"]).as_deref(), Some("50"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        s.undo_added_adventure_item(0);
        assert!(s.added_adventure_items().is_empty());
        assert!(s.value(&["032", "d", "1", "a"]).is_none());
    }

    #[test]
    fn set_core_updates_and_creates() {
        let adv = Raw::Struct(vec![(
            "G".into(),
            Field::Value(Raw::List(vec![Raw::Struct(vec![
                ("a".into(), sc("50")),
                ("b".into(), sc("1")),
                ("c".into(), sc("1024")),
                ("d".into(), sc("6")),
            ])])),
        )]);
        let root = Raw::Struct(vec![("032".into(), b64(adv))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        // Update existing core 50: sets both count (c) and quality (d).
        assert!(!s.set_core(50, "2000", 8, "Slime core").unwrap());
        assert_eq!(s.value(&["032", "G", "0", "c"]).as_deref(), Some("2000"));
        assert_eq!(s.value(&["032", "G", "0", "d"]).as_deref(), Some("8"));
        assert!(s.added_cores().is_empty());
        // Add a new core 69.
        assert!(s.set_core(69, "5", 7, "Core 69").unwrap());
        assert_eq!(s.added_cores().len(), 1);
        assert_eq!(s.value(&["032", "G", "1", "a"]).as_deref(), Some("69"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        s.undo_added_core(0);
        assert!(s.added_cores().is_empty());
        assert!(s.value(&["032", "G", "1", "a"]).is_none());
    }

    #[test]
    fn equip_existing_assigns_fresh_ref_and_sets_slot() {
        // X.R: [0] unequipped (d=0), [1] equipped (d=5). Pet 0 weapon empty.
        // Two pets so X.b stays an index-addressable list (1 pet → lone struct).
        let x = Raw::Struct(vec![
            ("R".into(), Field::Value(Raw::List(vec![instance("0"), instance("5")]))),
            ("b".into(), Field::Value(Raw::List(vec![pet("0"), pet("0")]))),
        ]);
        let root = Raw::Struct(vec![("X".into(), b64(x))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        s.equip_existing(0, 0, "e", "Magic Stick").unwrap();
        // Fresh d = max(0,5)+1 = 6, written to the item and the pet slot.
        assert_eq!(s.value(&["X", "R", "0", "d"]).as_deref(), Some("6"));
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("6"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn equip_existing_swaps_out_current_item() {
        // Pet 0 weapon already holds d=5 (item index 1). Equipping item index 0
        // returns the displaced item to inventory (d→0).
        let x = Raw::Struct(vec![
            ("R".into(), Field::Value(Raw::List(vec![instance("0"), instance("5")]))),
            ("b".into(), Field::Value(Raw::List(vec![pet("5"), pet("0")]))),
        ]);
        let root = Raw::Struct(vec![("X".into(), b64(x))]);
        let mut s = EditSession::load(&encoded(&root), None).unwrap();
        s.equip_existing(0, 0, "e", "New Weapon").unwrap();
        assert_eq!(s.value(&["X", "R", "1", "d"]).as_deref(), Some("0")); // displaced
        assert_eq!(s.value(&["X", "R", "0", "d"]).as_deref(), Some("6")); // new ref
        assert_eq!(s.value(&["X", "b", "0", "w", "e"]).as_deref(), Some("6"));
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
    }

    #[test]
    fn delete_material_and_undo_restores() {
        let mut s = EditSession::load(&encoded(&mat_root()), None).unwrap();
        s.delete_material(0, "Ant").unwrap(); // X.Q[0] = id 117
        assert_eq!(s.removed().len(), 1);
        assert!(s.value(&["X", "Q", "a=117", "a"]).is_none());
        let out = s.encode();
        s.validate_encoded(&out).unwrap();
        s.undo_removed(0);
        assert!(s.removed().is_empty());
        assert_eq!(s.value(&["X", "Q", "a=117", "a"]).as_deref(), Some("117"));
    }

    #[test]
    fn delete_just_created_equipment_undoes_creation() {
        let mut s = EditSession::load(&encoded(&equip_root()), None).unwrap();
        s.add_equipment(51, 0, 8, 0, 0, "Magic Stick", None).unwrap(); // appended at X.R[2]
        assert_eq!(s.added().len(), 1);
        s.delete_equipment(2, "Magic Stick").unwrap();
        assert!(s.added().is_empty()); // creation undone
        assert!(s.removed().is_empty()); // not tracked as a separate removal
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
