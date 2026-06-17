//! Declarative field-label schema for the save tree.
//!
//! This is the *naming* counterpart to the typed extraction in [`crate::model`]:
//! the same key→meaning knowledge that `from_tree` uses imperatively, exposed as
//! data so tools (the save editor's tree navigator and, later, its structured
//! sections) can label raw fields without maintaining a parallel map.
//!
//! Each [`BlockSchema`] describes one block of same-shaped data — a list whose
//! elements share a struct shape (pets, equipment, creations, …) or a single
//! keyed struct (Baal-Slayer parts). Keys are relative to the element and may be
//! dotted for nested structs (e.g. a pet's `w.d.b` is its class level).
//!
//! **Keep this in step with `model.rs`:** when you identify a new field there,
//! add a line here. The save-editor coverage test checks every key resolves on a
//! real save (so a key that exists *nowhere* is caught), and a single entry
//! labels every element of the block. It cannot catch a typo that happens to
//! land on another real key in the same struct — for that, cross-check the key
//! letters against `from_tree` in `model.rs`.

/// One labeled field within a block element. `key` is the path *relative to the
/// element*, dot-joined for nested structs (`"w.d.b"`).
pub struct FieldLabel {
    pub key: &'static str,
    pub label: &'static str,
}

/// A block of same-shaped data in the tree.
pub struct BlockSchema {
    /// Path prefix to the block (dotted segments from the root).
    pub base: &'static [&'static str],
    /// Singular display name for the block and each of its elements.
    pub name: &'static str,
    /// `true` when `base` is a list addressed by index (fields live at
    /// `base.<index>.key`); `false` for a single struct (`base.key`).
    pub is_list: bool,
    /// The labeled fields of each element / of the struct.
    pub fields: &'static [FieldLabel],
}

macro_rules! lbl {
    ($k:literal, $l:literal) => {
        FieldLabel { key: $k, label: $l }
    };
}

/// Pets — `X.b.<index>` (with nested `w` dungeon/class sub-structs).
pub const PET_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Name"),
    lbl!("k", "Type Id"),
    lbl!("l", "Unlocked"),
    lbl!("E", "Growth"),
    lbl!("g", "Normal Level"),
    lbl!("h", "Normal Exp (current)"),
    lbl!("j", "Normal Health"),
    lbl!("o", "Clone Physical"),
    lbl!("p", "Clone Mystic"),
    lbl!("q", "Clone Battle"),
    lbl!("r", "Clone HP"),
    lbl!("v", "Team Slot"),
    lbl!("F", "Partner Type Id"),
    lbl!("G", "Partner Days"),
    lbl!("H", "Working Exp (ms)"),
    lbl!("w", "Dungeon & Class"),
    lbl!("w.a", "Element Id"),
    lbl!("w.b", "Dungeon Level"),
    lbl!("w.c", "Dungeon Exp (current)"),
    lbl!("w.d", "Class"),
    lbl!("w.d.a", "Class Id"),
    lbl!("w.d.b", "Class Level"),
    lbl!("w.d.c", "Class Exp (current)"),
    lbl!("w.e", "Weapon (instance id)"),
    lbl!("w.f", "Armor (instance id)"),
    lbl!("w.g", "Accessory (instance id)"),
];

/// Owned equipment instances — `X.R.<index>`.
pub const EQUIPMENT_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Type Id"),
    lbl!("b", "Plus Level"),
    lbl!("c", "Quality"),
    lbl!("d", "Instance Id"),
    lbl!("e", "Plus Cap"),
    lbl!("f", "Gem Level"),
    lbl!("g", "Gem Element Id"),
    lbl!("h", "Instance Id (mirror)"),
];

/// Material / item stacks — `X.Q.<index>`.
pub const MATERIAL_FIELDS: &[FieldLabel] = &[lbl!("a", "Item Id"), lbl!("b", "Count")];

/// Gem inventory — `X.002.<index>`.
pub const GEM_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Element Id"),
    lbl!("b", "Level"),
    lbl!("c", "Count"),
];

/// Persistent dungeon teams — `X.S.<index>`.
pub const DUNGEON_TEAM_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Dungeon Id"),
    lbl!("d", "Depth"),
    lbl!("i", "Dungeon Name"),
    lbl!("a", "Member Pet Type Ids"),
    lbl!("c", "Pending Loot"),
];

/// Campaign slots — `X.x.<index>`.
pub const CAMPAIGN_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Slot Index"),
    lbl!("d", "Pet Type Ids"),
    lbl!("e", "Duration (ms)"),
    lbl!("f", "Bonus"),
];

/// Adventure-mode researches — `032.H.a.<index>`.
pub const RESEARCH_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Research Id"),
    lbl!("b", "Level"),
    lbl!("f", "Max Level"),
    lbl!("c", "In Progress"),
    lbl!("d", "Progress"),
];

/// Creations — `i.<index>`.
pub const CREATION_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Creation Id"),
    lbl!("d", "Current Amount"),
    lbl!("e", "Clone Cost"),
    lbl!("g", "Total Created"),
    lbl!("i", "Next At"),
];

/// Monuments — `D.<index>`.
pub const MONUMENT_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Monument Id"),
    lbl!("b", "Level"),
    lbl!("g", "Next At"),
    lbl!("h", "Spread"),
    lbl!("f", "Building"),
    lbl!("c", "Clones Allocated"),
    lbl!("d", "Progress"),
];

/// Mights — `V.<index>`.
pub const MIGHT_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Might Id"),
    lbl!("b", "Level"),
    lbl!("m", "Next At"),
    lbl!("n", "Spread"),
    lbl!("e", "Special (Unleash)"),
    lbl!("g", "Base Duration (s)"),
    lbl!("i", "Unleash HP Recovery %"),
    lbl!("j", "Unleash Attack %"),
    lbl!("k", "Unleash Mystic %"),
];

/// SpaceDim / Light-Dimension elements — `009.b.<index>`.
pub const SPACEDIM_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Element Id"),
    lbl!("b", "Clones Allocated"),
    lbl!("c", "Level"),
    lbl!("d", "Next At"),
    lbl!("e", "Progress"),
    lbl!("f", "Spread"),
];

/// Divinity Generator upgrade tracks — `K.l.<index>`.
pub const DIVINITY_UPGRADE_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Upgrade Id"),
    lbl!("b", "Level"),
    lbl!("g", "Multiplier"),
];

/// Baal-Slayer (TBS) component levels — single struct at `S`.
pub const TBS_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Eyes Level"),
    lbl!("c", "Mouth Level"),
    lbl!("d", "Wings Level"),
    lbl!("e", "Tail Level"),
    lbl!("f", "Feet Level"),
];

/// Every block, consumed by the save editor to build tree labels.
pub const BLOCKS: &[BlockSchema] = &[
    BlockSchema { base: &["X", "b"], name: "Pet", is_list: true, fields: PET_FIELDS },
    BlockSchema { base: &["X", "R"], name: "Equipment", is_list: true, fields: EQUIPMENT_FIELDS },
    BlockSchema { base: &["X", "Q"], name: "Material", is_list: true, fields: MATERIAL_FIELDS },
    BlockSchema { base: &["X", "002"], name: "Gem", is_list: true, fields: GEM_FIELDS },
    BlockSchema { base: &["X", "S"], name: "Dungeon Team", is_list: true, fields: DUNGEON_TEAM_FIELDS },
    BlockSchema { base: &["X", "x"], name: "Campaign", is_list: true, fields: CAMPAIGN_FIELDS },
    BlockSchema { base: &["032", "H", "a"], name: "Research", is_list: true, fields: RESEARCH_FIELDS },
    BlockSchema { base: &["i"], name: "Creation", is_list: true, fields: CREATION_FIELDS },
    BlockSchema { base: &["D"], name: "Monument", is_list: true, fields: MONUMENT_FIELDS },
    BlockSchema { base: &["V"], name: "Might", is_list: true, fields: MIGHT_FIELDS },
    BlockSchema { base: &["009", "b"], name: "SpaceDim Element", is_list: true, fields: SPACEDIM_FIELDS },
    BlockSchema { base: &["K", "l"], name: "Divinity Upgrade", is_list: true, fields: DIVINITY_UPGRADE_FIELDS },
    BlockSchema { base: &["S"], name: "Baal Slayer Parts", is_list: false, fields: TBS_FIELDS },
];
