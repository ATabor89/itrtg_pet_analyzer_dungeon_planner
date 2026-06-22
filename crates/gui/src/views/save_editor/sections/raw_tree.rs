//! The raw save tree navigator: a searchable, name-labeled view of the lossless
//! tree. Editing a scalar leaf stages a change through the same [`EditSession`]
//! as the structured sections, so the two views share one pending log and never
//! drift.
//!
//! Search has two modes. **Filter** (default) hides everything that doesn't
//! match. **Reveal in place** keeps the whole tree visible but expands the path
//! to each match and scrolls to the first — useful for poking at values near a
//! known field. Matching considers raw keys, scalar values, *and* the registry
//! display name at each node, so searching "Shadow Clones" finds the labeled
//! field, not just the literal string elsewhere.
//!
//! When a search is active we compute the matching/ancestor-of-match node set in
//! a single O(N) pre-pass (`build_matches`) and the render walk just consults it
//! — so typing in the search box doesn't re-walk the subtree at every container.
//!
//! Edits are collected during the (immutable) walk and applied afterwards, so we
//! never hold a `&Raw` borrow of the tree while calling `&mut` `set_scalar`.

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use itrtg_models::{Class, Element};
use save_parser::labels::Resolve;
use save_parser::raw::{Field, Raw};
use save_parser::{items, model};

use crate::style;
use crate::views::save_editor::registry::{FieldKind, FieldRegistry};
use crate::views::save_editor::session::EditSession;

/// Turn an id field's raw value into a human name, per its [`Resolve`] hint.
/// `root` is the whole tree, needed to chase an equipment-instance id into `X.R`.
fn resolve_name(resolve: Resolve, value: &str, root: &Raw) -> Option<String> {
    let v = value.trim();
    match resolve {
        Resolve::Literal => (!v.is_empty()).then(|| v.to_string()),
        Resolve::Material => items::material_name(v.parse().ok()?).map(str::to_string),
        Resolve::Dungeon => items::dungeon_name(v.parse().ok()?).map(str::to_string),
        Resolve::Pond => items::pond_name(v.parse().ok()?).map(str::to_string),
        Resolve::Statue => items::statue_name(v.parse().ok()?).map(str::to_string),
        Resolve::VillageBuilding => items::village_building_name(v.parse().ok()?).map(str::to_string),
        Resolve::UltimateBeing => items::ultimate_being_name(v.parse().ok()?).map(str::to_string),
        Resolve::Equipment => items::equipment_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::Monument => items::monument_name(v.parse().ok()?).map(str::to_string),
        Resolve::Might => items::might_name(v.parse().ok()?).map(str::to_string),
        Resolve::Creation => items::creation_name(v.parse().ok()?).map(str::to_string),
        Resolve::SpaceDim => items::spacedim_name(v.parse().ok()?).map(str::to_string),
        Resolve::PhysicalTraining => {
            items::physical_training_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::Skill => items::skill_name(v.parse().ok()?).map(str::to_string),
        Resolve::Monster => items::monster_name(v.parse().ok()?).map(str::to_string),
        Resolve::DivinityUpgrade => {
            items::divinity_upgrade_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::AdventureItem => items::adventure_item_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureEnemy => items::adventure_enemy_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureClass => items::adventure_class_name(v.parse().ok()?).map(str::to_string),
        Resolve::Research => model::research_name(v.parse().ok()?).map(str::to_string),
        Resolve::Element => model::element_from_id(v.parse().ok()?).map(element_name),
        Resolve::Class => model::class_from_id(v.parse().ok()?).map(class_name),
        Resolve::PetType => items::pet_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::ElementalForm => {
            items::elemental_form_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::CampaignType => items::campaign_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::CampaignPref => {
            // Stored offset by 1: 0 = unset, else campaign_type_name(id - 1).
            let id: u32 = v.parse().ok()?;
            match id.checked_sub(1) {
                None => Some("Unset".to_string()),
                Some(c) => items::campaign_type_name(c).map(str::to_string),
            }
        }
        Resolve::FeedingSetting => {
            items::feeding_setting_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::GemElement => items::gem_element_name(v.parse().ok()?).map(str::to_string),
        Resolve::Challenge => items::challenge_name(v.parse().ok()?).map(str::to_string),
        Resolve::ChallengeDifficulty => {
            items::challenge_difficulty_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::UltimateOverflowUpgrade => {
            items::ultimate_overflow_upgrade_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::RtiBonus => items::rti_bonus_name(v.parse().ok()?).map(str::to_string),
        Resolve::EquipmentInstance => resolve_equipment_instance(v, root),
        // Node-based: handled directly in `element_label`, never as a scalar.
        Resolve::EquipmentNode | Resolve::CoreNode => None,
    }
}

/// Format a core element struct as "Enemy Quality" (e.g. "Slime SSS"); falls
/// back to "Core" / omits the quality when an id is unknown.
fn core_label(node: &Raw) -> Option<String> {
    let enemy_id = scalar_u32(node, "a")?;
    let name = items::adventure_enemy_name(enemy_id).unwrap_or("Core");
    let mut s = name.to_string();
    if let Some(q) = scalar_u32(node, "d").and_then(items::quality_name) {
        s.push(' ');
        s.push_str(q);
    }
    Some(s)
}

/// Format an equipment element struct as "Name Quality+Plus" (e.g.
/// "Magic Stick SSS+20"); the +plus is omitted at +0.
fn equip_label(node: &Raw) -> Option<String> {
    let type_id = scalar_u32(node, "a")?;
    let name = items::equipment_type_name(type_id).unwrap_or("Equipment");
    let mut s = name.to_string();
    if let Some(q) = scalar_u32(node, "c").and_then(items::quality_name) {
        s.push(' ');
        s.push_str(q);
    }
    match scalar_u32(node, "b") {
        Some(plus) if plus > 0 => s.push_str(&format!("+{plus}")),
        _ => {}
    }
    Some(s)
}

fn element_name(e: Element) -> String {
    match e {
        Element::Fire => "Fire",
        Element::Water => "Water",
        Element::Wind => "Wind",
        Element::Earth => "Earth",
        Element::Neutral => "Neutral",
        Element::All => "All",
    }
    .to_string()
}

fn class_name(c: Class) -> String {
    match c {
        Class::Adventurer => "Adventurer",
        Class::Blacksmith => "Blacksmith",
        Class::Alchemist => "Alchemist",
        Class::Defender => "Defender",
        Class::Supporter => "Supporter",
        Class::Rogue => "Rogue",
        Class::Assassin => "Assassin",
        Class::Mage => "Mage",
        Class::Wildcard => "Wildcard",
    }
    .to_string()
}

/// Resolve a pet's equipment-slot id (`w.e`/`w.f`/`w.g`) to the item, e.g.
/// "Magic Stick SSS+20". A slot value of 0 means **empty** (no annotation).
///
/// Matches on the **equip id** (`d`) first — that's what slots reference — then
/// the catalog id `h` as a fallback. See the inline note for the proof and the
/// collision the old "`h` first" rule got wrong.
fn resolve_equipment_instance(value: &str, root: &Raw) -> Option<String> {
    let instance: u32 = value.parse().ok()?;
    if instance == 0 {
        return None; // 0 = empty slot
    }
    let Raw::List(list) = root.get_path(&["X", "R"])? else {
        return None;
    };
    // Pet slots (`w.e`/`w.f`/`w.g`) reference the **equip id `d`** (0 = empty /
    // in inventory) — proven 2026-06-19 on real Steam saves: event gear with
    // `d≠h` resolves by `d` (e.g. Vampire's armor slot 23 = `d`23 = Merry
    // Mantle, while that instance's `h` is 136). Match `d` first; fall back to
    // the always-unique catalog id `h` only when no `d` matches (defensive —
    // and what prevents the `d=20`/`h=20` cross-field collision seen on edited
    // saves from picking the wrong item).
    let item = list
        .iter()
        .find(|it| scalar_u32(it, "d") == Some(instance))
        .or_else(|| list.iter().find(|it| scalar_u32(it, "h") == Some(instance)))?;
    equip_label(item)
}

/// The `X.R` list **index** of the equipment instance a slot value references,
/// using the same `d`-first / `h`-fallback matching as
/// [`resolve_equipment_instance`]. Drives the cross-reference jump (a pet's
/// weapon/armor/accessory slot → the item's node). `None` for 0/empty or no match.
fn equipment_instance_index(value: &str, root: &Raw) -> Option<usize> {
    let instance: u32 = value.parse().ok()?;
    if instance == 0 {
        return None;
    }
    let Raw::List(list) = root.get_path(&["X", "R"])? else {
        return None;
    };
    list.iter()
        .position(|it| scalar_u32(it, "d") == Some(instance))
        .or_else(|| list.iter().position(|it| scalar_u32(it, "h") == Some(instance)))
}

fn scalar_u32(node: &Raw, key: &str) -> Option<u32> {
    match node.get(key) {
        Some(Raw::Scalar(s)) => s.parse().ok(),
        _ => None,
    }
}

/// A staged edit gathered during the walk: (path, label, new value).
type StagedEdit = (Vec<String>, String, String);

/// How an active search affects the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// No active query.
    None,
    /// Hide everything that doesn't match.
    Filter,
    /// Keep the whole tree; expand the path to matches and scroll to the first.
    Reveal,
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    registry: &FieldRegistry,
    buffers: &mut HashMap<String, String>,
    search: &mut String,
    reveal: &mut bool,
    scrolled_query: &mut Option<String>,
    generation: &mut u64,
    jump: &mut Option<String>,
) {
    ui.heading("Raw Save Tree");
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(240.0)
                .hint_text("key, field name, or value"),
        );
        if ui.button("Clear").clicked() {
            search.clear();
            *jump = None;
        }
        ui.separator();
        ui.checkbox(reveal, "Reveal in place").on_hover_text(
            "Jump to matches and expand the path to them, without hiding the rest of the tree.",
        );
        ui.separator();
        // Collapse everything by bumping the browse id namespace: every container
        // gets a fresh (default-collapsed) id, including hidden descendants.
        if ui
            .button("⊟ Collapse all")
            .on_hover_text("Collapse the entire tree")
            .clicked()
        {
            *generation = generation.wrapping_add(1);
        }
    });
    ui.label(
        RichText::new(
            "Known fields show their name. Editing a scalar stages a change in the \
             pending list; values are written verbatim.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );

    // A search supersedes (and consumes) an active navigation jump.
    if !search.trim().is_empty() {
        *jump = None;
    }
    // Banner for an active jump (set by "navigate to tree" from elsewhere).
    if let Some(target) = jump.clone() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("▶ Navigated to").color(style::ACCENT).size(11.0));
            ui.label(RichText::new(&target).color(style::TEXT_BRIGHT).monospace().size(11.0));
            if ui.small_button("Clear").clicked() {
                *jump = None;
            }
        });
    }
    ui.separator();

    let query = search.trim().to_lowercase();
    // Mode + the "reveal key" we dedupe scroll-once against: a search query wins;
    // otherwise an active jump reveals the exact path it names.
    let (mode, reveal_key, matches) = if !query.is_empty() {
        let m = if *reveal { Mode::Reveal } else { Mode::Filter };
        (m, query.clone(), build_matches(session.root(), &query, registry))
    } else if let Some(target) = jump.clone() {
        (Mode::Reveal, target.clone(), prefix_set(&target))
    } else {
        (Mode::None, String::new(), HashSet::new())
    };

    // Scroll to the first match once per reveal key — not every frame, or the
    // viewport would snap back and the user could never scroll away.
    if mode != Mode::Reveal {
        *scrolled_query = None;
    }
    let want_scroll = mode == Mode::Reveal && scrolled_query.as_deref() != Some(reveal_key.as_str());

    let mut edits: Vec<StagedEdit> = Vec::new();
    let mut scrolled = false;
    let mut jump_request: Option<String> = None;
    {
        let root = session.root();
        let mut walk = Walk {
            registry,
            buffers,
            edits: &mut edits,
            query: &query,
            mode,
            matches: &matches,
            want_scroll,
            scrolled: &mut scrolled,
            generation: *generation,
            root,
            jump_request: &mut jump_request,
        };
        let mut path: Vec<String> = Vec::new();
        if let Raw::Struct(fields) = root.peel() {
            for (key, field) in fields {
                walk.render_field(ui, &mut path, key, field);
            }
        }
    }
    if want_scroll {
        *scrolled_query = Some(reveal_key);
    }

    // A clicked cross-reference link: reveal the referenced node. Clear the
    // search so the jump (not a stale query) drives the view, and reset the
    // scroll dedupe so the new target scrolls into view.
    if let Some(target) = jump_request {
        *jump = Some(target);
        search.clear();
        *scrolled_query = None;
    }

    // Apply staged edits now that the read-only borrow of the tree is released.
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let _ = session.set_scalar(&p, label, &value);
    }
}

/// The registry display-name at `path`, lowercased-contains the query?
fn name_matches(registry: &FieldRegistry, path: &[String], query: &str) -> bool {
    let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    registry
        .lookup(&p)
        .is_some_and(|d| d.name.to_lowercase().contains(query))
}

/// The set of a dotted path and all its ancestor prefixes, e.g. `X.Q.1.b` →
/// {`X`, `X.Q`, `X.Q.1`, `X.Q.1.b`}. Used to drive a navigation jump: the walk
/// force-opens every container on the path and scrolls to the leaf, exactly as
/// Reveal mode does for a search match.
fn prefix_set(dotted: &str) -> HashSet<String> {
    let parts: Vec<&str> = dotted.split('.').collect();
    (1..=parts.len()).map(|i| parts[..i].join(".")).collect()
}

/// Build the set of node paths (dotted) that match the query or are an ancestor
/// of a match — a single pass over the whole tree.
fn build_matches(root: &Raw, query: &str, registry: &FieldRegistry) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    if let Raw::Struct(fields) = root.peel() {
        for (k, f) in fields {
            if let Field::Value(v) = f {
                path.push(k.clone());
                collect_matches(v, &mut path, query, registry, &mut out);
                path.pop();
            }
        }
    }
    out
}

/// Returns whether `node` (at `path`) matches or contains a match, inserting
/// every such node's dotted path into `out`.
fn collect_matches(
    node: &Raw,
    path: &mut Vec<String>,
    query: &str,
    registry: &FieldRegistry,
    out: &mut HashSet<String>,
) -> bool {
    // This node matches if its key (last path segment) or registry name matches.
    let mut found = name_matches(registry, path, query)
        || path
            .last()
            .is_some_and(|seg| seg.to_lowercase().contains(query));

    match node.peel() {
        Raw::Scalar(s) => {
            if s.to_lowercase().contains(query) {
                found = true;
            }
        }
        Raw::Struct(fields) => {
            for (k, f) in fields {
                if let Field::Value(v) = f {
                    path.push(k.clone());
                    found |= collect_matches(v, path, query, registry, out);
                    path.pop();
                }
            }
        }
        Raw::List(items) => {
            for (i, it) in items.iter().enumerate() {
                path.push(i.to_string());
                found |= collect_matches(it, path, query, registry, out);
                path.pop();
            }
        }
        Raw::Base64(_) => {}
    }

    if found {
        out.insert(path.join("."));
    }
    found
}

/// How a node is named under its parent: a struct key or a list index.
enum NodeName<'a> {
    Key(&'a str),
    Index(usize),
}

impl NodeName<'_> {
    fn display(&self) -> String {
        match self {
            NodeName::Key(k) => (*k).to_string(),
            NodeName::Index(i) => format!("[{i}]"),
        }
    }
}

/// Mutable walk context (kept separate from the borrowed `&Raw` tree).
struct Walk<'a> {
    registry: &'a FieldRegistry,
    buffers: &'a mut HashMap<String, String>,
    edits: &'a mut Vec<StagedEdit>,
    /// Lowercased search query (empty when `mode == None`).
    query: &'a str,
    mode: Mode,
    /// Node paths that match or are an ancestor of a match (empty in `None`).
    matches: &'a HashSet<String>,
    /// Whether we should scroll to the first match this frame (Reveal only).
    want_scroll: bool,
    /// Whether we've already scrolled this frame.
    scrolled: &'a mut bool,
    /// Browse-mode id namespace generation; bumped by "Collapse all" so every
    /// container gets a fresh, default-collapsed id.
    generation: u64,
    /// The whole tree, for cross-references (equipment-instance id → item).
    root: &'a Raw,
    /// Set when a cross-reference link is clicked: the dotted path to jump to
    /// (applied after the walk — switches the tree to reveal that node).
    jump_request: &'a mut Option<String>,
}

impl Walk<'_> {
    /// The registry display-name for the current path, if known.
    fn known_name(&self, path: &[String]) -> Option<&'static str> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        self.registry.lookup(&p).map(|d| d.name)
    }

    /// Resolve a human title for an element container from one of its children
    /// (a pet by its name, a monument by its id), if the schema says how.
    fn element_label(&self, path: &[String], value: &Raw) -> Option<String> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let (key, resolve) = self.registry.lookup(&p)?.element_name?;
        // Node-based titles read the whole element, not a single child scalar.
        if resolve == Resolve::EquipmentNode {
            return equip_label(value);
        }
        if resolve == Resolve::CoreNode {
            return core_label(value);
        }
        let child = match value.get(key)? {
            Raw::Scalar(s) => s.as_str(),
            _ => return None,
        };
        resolve_name(resolve, child, self.root)
    }

    /// If `path` names an id scalar, resolve its value to a name annotation.
    fn scalar_annotation(&self, path: &[String], value: &str) -> Option<String> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let resolve = self.registry.lookup(&p)?.resolve?;
        resolve_name(resolve, value, self.root)
    }

    /// If this scalar is an equipment-instance reference (a pet's `w.e`/`w.f`/
    /// `w.g` slot), the dotted `X.R.<i>` path of the item it points at — the
    /// target of a cross-reference jump. Only `EquipmentInstance` ids reference
    /// another tree node; every other id resolves to a static name table.
    fn cross_ref_target(&self, path: &[String], value: &str) -> Option<String> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        if self.registry.lookup(&p)?.resolve? != Resolve::EquipmentInstance {
            return None;
        }
        let idx = equipment_instance_index(value, self.root)?;
        Some(format!("X.R.{idx}"))
    }

    /// Is this node a match or ancestor-of-match (from the pre-pass)?
    fn on_match(&self, path: &[String]) -> bool {
        self.mode != Mode::None && self.matches.contains(&path.join("."))
    }

    /// Render one struct field (a key and its value).
    fn render_field(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, key: &str, field: &Field) {
        path.push(key.to_string());
        match field {
            Field::EmptyColon | Field::EmptyBare => {
                let show = self.mode != Mode::Filter || key.to_lowercase().contains(self.query);
                if show {
                    self.empty_row(ui, &NodeName::Key(key));
                }
            }
            Field::Value(v) => self.render_value(ui, path, NodeName::Key(key), v),
        }
        path.pop();
    }

    /// Render a value (scalar leaf, or a struct/list container).
    fn render_value(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, name: NodeName, value: &Raw) {
        match value.peel() {
            Raw::Scalar(s) => {
                let is_match = self.on_match(path);
                let visible = self.mode != Mode::Filter || is_match;
                if visible {
                    self.scalar_row(ui, path, &name, s, is_match);
                }
            }
            Raw::Struct(fields) => {
                if self.mode == Mode::Filter && !self.on_match(path) {
                    return;
                }
                let force_open = self.force_state(path);
                let extra = self.element_label(path, value);
                let summary = format!("{{{} fields}}", fields.len());
                let raw = value.peel().serialize();
                self.container(ui, path, &name, extra, summary, raw, force_open, |w, path, ui| {
                    for (k, f) in fields {
                        w.render_field(ui, path, k, f);
                    }
                });
            }
            Raw::List(items) => {
                if self.mode == Mode::Filter && !self.on_match(path) {
                    return;
                }
                let force_open = self.force_state(path);
                let extra = self.element_label(path, value);
                let summary = format!("[{} items]", items.len());
                let raw = value.peel().serialize();
                self.container(ui, path, &name, extra, summary, raw, force_open, |w, path, ui| {
                    for (i, item) in items.iter().enumerate() {
                        path.push(i.to_string());
                        w.render_value(ui, path, NodeName::Index(i), item);
                        path.pop();
                    }
                });
            }
            // peel() already removed any Base64 wrapper.
            Raw::Base64(_) => {}
        }
    }

    /// Force a container open only while searching and it's on the path to a
    /// match. In browse mode we never force state — collapse-all is handled by
    /// the id-namespace generation instead, so the user's manual expansions are
    /// preserved and not fought every frame.
    fn force_state(&self, path: &[String]) -> Option<bool> {
        if self.mode != Mode::None && self.on_match(path) {
            Some(true)
        } else {
            None
        }
    }

    /// The CollapsingHeader id for a node: a transient `search:` namespace while
    /// searching (so forced-open matches don't leak into the browse view when the
    /// search clears), and a generation-stamped browse namespace otherwise (so
    /// "Collapse all" resets everything by bumping the generation).
    ///
    /// While a search is active *all* containers use the transient namespace, so
    /// any manual expand/collapse done mid-search (e.g. poking at a neighbor in
    /// Reveal mode) is also discarded when the search clears — by design.
    fn header_salt(&self, path: &[String]) -> String {
        if self.mode == Mode::None {
            format!("{}\u{1f}{}", self.generation, path.join("."))
        } else {
            format!("search\u{1f}{}", path.join("."))
        }
    }

    /// A collapsing container header with a name label and a summary count.
    #[allow(clippy::too_many_arguments)]
    fn container(
        &mut self,
        ui: &mut egui::Ui,
        path: &mut Vec<String>,
        name: &NodeName,
        extra: Option<String>,
        summary: String,
        raw: String,
        force_open: Option<bool>,
        build: impl FnOnce(&mut Walk, &mut Vec<String>, &mut egui::Ui),
    ) {
        let known = self.known_name(path);
        let title = match (&extra, known) {
            // resolved name (e.g. "Robot") · type label · node · summary
            (Some(e), Some(n)) => format!("{}  ·  {} {}   {}", e, n, name.display(), summary),
            (Some(e), None) => format!("{}  ·  {}   {}", e, name.display(), summary),
            (None, Some(n)) => format!("{}  ·  {}   {}", n, name.display(), summary),
            (None, None) => format!("{}   {}", name.display(), summary),
        };
        let color = if extra.is_some() {
            style::TEXT_BRIGHT
        } else if known.is_some() {
            style::ACCENT
        } else {
            style::TEXT_NORMAL
        };
        let salt = self.header_salt(path);
        let resp = egui::CollapsingHeader::new(RichText::new(title).color(color))
            .id_salt(salt)
            .open(force_open)
            .show(ui, |ui| build(self, path, ui));
        let path_str = path.join(".");
        resp.header_response.context_menu(|ui| {
            if ui.button("Copy path").clicked() {
                ui.ctx().copy_text(path_str.clone());
                ui.close_menu();
            }
            if ui
                .button("Copy node (raw)")
                .on_hover_text("Copy this node and its whole subtree as raw save text")
                .clicked()
            {
                ui.ctx().copy_text(raw.clone());
                ui.close_menu();
            }
        });
    }

    /// An editable scalar leaf row. `is_match` highlights it and, in Reveal mode,
    /// scrolls the first match into view (once per query).
    fn scalar_row(
        &mut self,
        ui: &mut egui::Ui,
        path: &[String],
        name: &NodeName,
        current: &str,
        is_match: bool,
    ) {
        let known = self.known_name(path).map(|s| s.to_string());
        let annotation = self.scalar_annotation(path, current);
        // Descriptor-driven edit affordances (additive — the verbatim text edit
        // stays, so the raw tree remains fully unrestricted).
        let (is_bool, range) = {
            let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
            self.registry
                .lookup(&p)
                .map_or((false, None), |d| (d.kind == FieldKind::Bool, d.range))
        };
        let key = path.join(".");
        let path_str = path.join(".");
        let value_str = current.to_string();
        let row = ui.horizontal(|ui| {
            // The name/key label carries the copy menu — right-clicking the value
            // box hits egui's own text context menu instead.
            let label_resp = match &known {
                Some(n) => {
                    let r1 = ui.label(RichText::new(n).color(style::ACCENT).strong());
                    let r2 = ui.label(
                        RichText::new(format!("· {}", name.display()))
                            .color(style::TEXT_MUTED)
                            .monospace(),
                    );
                    r1 | r2
                }
                None => {
                    let color = if is_match {
                        style::WARNING
                    } else {
                        style::TEXT_NORMAL
                    };
                    ui.label(RichText::new(name.display()).color(color).monospace())
                }
            };
            label_resp
                .on_hover_text("Right-click to copy path / value")
                .context_menu(|ui| {
                    if ui.button("Copy path").clicked() {
                        ui.ctx().copy_text(path_str.clone());
                        ui.close_menu();
                    }
                    if ui.button("Copy value").clicked() {
                        ui.ctx().copy_text(value_str.clone());
                        ui.close_menu();
                    }
                });

            let mut newval: Option<String> = None;
            {
                let buf = self.buffers.entry(key).or_insert_with(|| current.to_string());
                let resp = ui.add(
                    egui::TextEdit::singleline(buf)
                        .desired_width(260.0)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.lost_focus() {
                    let t = buf.trim().to_string();
                    if t != current {
                        buf.clone_from(&t);
                        newval = Some(t);
                    }
                } else if !resp.has_focus() && buf.as_str() != current {
                    // Mirror changes made elsewhere (structured sections, undo).
                    *buf = current.to_string();
                }
            }
            // Bool fields: quick True/False set buttons (text edit still works).
            if is_bool {
                if ui.small_button("True").clicked() && current != "True" {
                    newval = Some("True".to_string());
                }
                if ui.small_button("False").clicked() && current != "False" {
                    newval = Some("False".to_string());
                }
            }
            // Ranged numeric fields: show the bound, warn (don't block) if the
            // current value is outside it.
            if let Some((lo, hi)) = range {
                let oor = current.trim().parse::<u32>().is_ok_and(|v| v < lo || v > hi);
                let (color, hover) = if oor {
                    (style::WARNING, "value is outside the expected range")
                } else {
                    (style::TEXT_MUTED, "expected range")
                };
                ui.label(RichText::new(format!("({lo}–{hi})")).color(color).size(11.0))
                    .on_hover_text(hover);
            }
            if let Some(a) = &annotation {
                // An equipment-instance reference is a clickable cross-ref that
                // jumps to the item's node; other ids are plain name labels.
                match self.cross_ref_target(path, current) {
                    Some(target) => {
                        if ui
                            .link(RichText::new(format!("→ {a}")).color(style::ACCENT))
                            .on_hover_text("Go to this item in the tree")
                            .clicked()
                        {
                            *self.jump_request = Some(target);
                        }
                    }
                    None => {
                        ui.label(RichText::new(format!("→ {a}")).color(style::SUCCESS));
                    }
                }
            }
            if is_match {
                ui.label(RichText::new("◀").color(style::WARNING).small());
            }
            if let Some(v) = newval {
                let label = known.unwrap_or_else(|| path.join("."));
                self.edits.push((path.to_vec(), label, v));
            }
        });

        // Bring the first match into view in Reveal mode, once per query.
        if is_match && self.mode == Mode::Reveal && self.want_scroll && !*self.scrolled {
            row.response.scroll_to_me(Some(egui::Align::Center));
            *self.scrolled = true;
        }
    }

    fn empty_row(&self, ui: &mut egui::Ui, name: &NodeName) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(name.display())
                    .color(style::TEXT_MUTED)
                    .monospace(),
            );
            ui.label(RichText::new("(empty)").color(style::TEXT_MUTED).size(11.0));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::raw::{Field, Raw};

    fn scalar(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }

    /// Searching a registry display name finds the labeled node (and its
    /// ancestors), not just literal occurrences of the text elsewhere.
    #[test]
    fn search_matches_registry_labels() {
        // `e` is labeled "Shadow Clones"; `e.a`/`e.b` "Shadow Clones (current/max)".
        let root = Raw::Struct(vec![(
            "e".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![
                ("a".into(), scalar("5")),
                ("b".into(), scalar("9")),
            ])))),
        )]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "shadow", &reg);
        assert!(m.contains("e"), "ancestor block marked");
        assert!(m.contains("e.a"), "leaf matched by its label");
        assert!(m.contains("e.b"));
    }

    #[test]
    fn resolves_ids_to_names() {
        let empty = Raw::Struct(vec![]);
        // Class / element ids via the model tables.
        assert_eq!(resolve_name(Resolve::Class, "8", &empty).as_deref(), Some("Mage"));
        assert_eq!(resolve_name(Resolve::Element, "1", &empty).as_deref(), Some("Fire"));
        // Dungeon ids (dungeon-team / active-run blocks).
        assert_eq!(resolve_name(Resolve::Dungeon, "2", &empty).as_deref(), Some("Scrapyard"));
        assert_eq!(resolve_name(Resolve::Dungeon, "5", &empty).as_deref(), Some("Forest"));
        // Fishing pond.
        assert_eq!(resolve_name(Resolve::Pond, "4", &empty).as_deref(), Some("Sad Pond"));
        // Monument id matches the items table (don't hardcode the name).
        assert_eq!(
            resolve_name(Resolve::Monument, "0", &empty),
            save_parser::items::monument_name(0).map(str::to_string)
        );
        // Physical-training / skill / monster ids via the items tables.
        assert_eq!(
            resolve_name(Resolve::PhysicalTraining, "0", &empty),
            save_parser::items::physical_training_name(0).map(str::to_string)
        );
        assert_eq!(
            resolve_name(Resolve::Skill, "0", &empty),
            save_parser::items::skill_name(0).map(str::to_string)
        );
        // Adventure item / core-enemy ids (separate namespaces).
        assert_eq!(
            resolve_name(Resolve::AdventureItem, "3", &empty),
            save_parser::items::adventure_item_name(3).map(str::to_string)
        );
        assert_eq!(
            resolve_name(Resolve::AdventureEnemy, "50", &empty),
            save_parser::items::adventure_enemy_name(50).map(str::to_string)
        );
        // A core element struct titles as "Enemy Quality".
        let core = Raw::Struct(vec![
            ("a".into(), scalar("50")), // Slime
            ("d".into(), scalar("8")),  // SSS
        ]);
        assert_eq!(core_label(&core).as_deref(), Some("Slime SSS"));
        assert_eq!(
            resolve_name(Resolve::Monster, "33", &empty),
            save_parser::items::monster_name(33).map(str::to_string)
        );
        // A literal is returned as-is; an unknown id resolves to nothing.
        assert_eq!(resolve_name(Resolve::Literal, "Robot", &empty).as_deref(), Some("Robot"));
        assert!(resolve_name(Resolve::Material, "not-a-number", &empty).is_none());
    }

    #[test]
    fn resolves_equipment_instance_across_the_tree() {
        // Pet slots reference the equip id `d`; `h` is a separate always-unique
        // catalog id. An unequipped inventory item carries `d=0` and is never
        // referenced by a slot — but resolution falls back to `h` for it.
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![Raw::Struct(vec![
                    ("a".into(), scalar("51")),  // type id
                    ("b".into(), scalar("20")),  // plus
                    ("c".into(), scalar("8")),   // quality (SSS)
                    ("d".into(), scalar("0")),   // not equipped → equip id 0
                    ("h".into(), scalar("858")), // unique catalog id
                ])])),
            )])))),
        )]);
        let got = resolve_name(Resolve::EquipmentInstance, "858", &root).expect("via h fallback");
        assert!(got.contains("SSS"), "quality letter: {got}");
        assert!(got.contains("+20"), "plus level: {got}");
        assert!(!got.contains("q8"), "no raw quality id: {got}");
        // 0 = empty slot → no annotation (even though an item has d==0).
        assert!(resolve_name(Resolve::EquipmentInstance, "0", &root).is_none());
        assert!(resolve_name(Resolve::EquipmentInstance, "999", &root).is_none());
    }

    /// `d`/`h` cross-field collision: a slot value matches one item's equip id
    /// `d` and a *different* item's catalog id `h`. The slot must resolve by `d`
    /// (proven on real Steam saves; this is the case the old "h first" rule got
    /// wrong on edited saves — Legendary `d=20` vs Magic Stick `h=20`).
    #[test]
    fn equipment_slot_resolves_by_d_on_collision() {
        let item = |a: &str, d: &str, h: &str| {
            Raw::Struct(vec![
                ("a".into(), scalar(a)),
                ("b".into(), scalar("10")),
                ("c".into(), scalar("6")),
                ("d".into(), scalar(d)),
                ("h".into(), scalar(h)),
            ])
        };
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![
                    item("51", "0", "20"), // Magic Stick, unequipped, catalog id 20
                    item("80", "20", "0"), // Legendary Stick, equipped, equip id 20
                ])),
            )])))),
        )]);
        // Slot 20 → the equipped Legendary Stick (type 80), not the Magic Stick.
        let got = resolve_name(Resolve::EquipmentInstance, "20", &root).expect("resolves");
        assert!(got.contains("Legendary Stick"), "got {got}");
    }

    /// The cross-ref jump finds the item's `X.R` index by the same `d`-first /
    /// `h`-fallback rule as resolution (here index 1 holds `d=20`).
    #[test]
    fn equipment_instance_index_finds_the_item() {
        let item = |a: &str, d: &str, h: &str| {
            Raw::Struct(vec![
                ("a".into(), scalar(a)),
                ("d".into(), scalar(d)),
                ("h".into(), scalar(h)),
            ])
        };
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![
                    item("51", "0", "858"), // [0] unequipped
                    item("80", "20", "0"),  // [1] equipped, equip id 20
                ])),
            )])))),
        )]);
        assert_eq!(equipment_instance_index("20", &root), Some(1)); // by d
        assert_eq!(equipment_instance_index("858", &root), Some(0)); // by h fallback
        assert_eq!(equipment_instance_index("0", &root), None); // empty slot
        assert_eq!(equipment_instance_index("999", &root), None); // no match
    }

    /// "Copy node (raw)" copies `value.peel().serialize()`: a base64-wrapped
    /// struct copies as its readable `k:v;` text (not a base64 blob), and the
    /// result re-parses to the same tree (so it can be pasted back later, #17).
    #[test]
    fn copy_node_raw_is_readable_and_round_trips() {
        let wrapped = Raw::Base64(Box::new(Raw::Struct(vec![
            ("a".into(), scalar("5")),
            ("b".into(), scalar("9")),
        ])));
        let copied = wrapped.peel().serialize();
        assert_eq!(copied, "a:5;b:9;", "readable struct text, not a base64 blob");
        // The copied text re-parses back to the same (peeled) structure.
        assert_eq!(save_parser::raw::parse(&copied), *wrapped.peel());
    }

    /// A jump's prefix set force-opens every ancestor container and the leaf.
    #[test]
    fn prefix_set_includes_path_and_all_ancestors() {
        let s = prefix_set("X.Q.1.b");
        assert!(s.contains("X"));
        assert!(s.contains("X.Q"));
        assert!(s.contains("X.Q.1"));
        assert!(s.contains("X.Q.1.b"));
        assert_eq!(s.len(), 4);
        // A single segment is its own only prefix.
        assert_eq!(prefix_set("W"), HashSet::from(["W".to_string()]));
    }

    /// A value/key match is included; unrelated siblings are not.
    #[test]
    fn search_matches_value_and_excludes_unrelated() {
        let root = Raw::Struct(vec![("W".into(), scalar("Adam")), ("c".into(), scalar("123"))]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "adam", &reg);
        assert!(m.contains("W"), "matched by scalar value");
        assert!(!m.contains("c"), "unrelated field excluded");
    }
}
