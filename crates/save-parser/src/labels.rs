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
//! Fields and elements can carry a [`Resolve`] hint: an id that the editor turns
//! into a human name (monument id → "Mighty Statue", class id → "Mage", an
//! equipment instance id → the item it points at). A block's `element_name` says
//! how to title each element (a pet by its name, a monument by its id).
//!
//! **Keep this in step with `model.rs`:** when you identify a new field there,
//! add a line here. The save-editor coverage test checks every key resolves on a
//! real save (so a key that exists *nowhere* is caught), and a single entry
//! labels every element of the block. It cannot catch a typo that happens to
//! land on another real key in the same struct — for that, cross-check the key
//! letters against `from_tree` in `model.rs`.

/// How an id field is turned into a human name by the editor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resolve {
    /// The value is already the name (e.g. a pet's name string).
    Literal,
    /// Material / item id → `items::material_name`.
    Material,
    /// Equipment *type* id → `items::equipment_type_name`.
    Equipment,
    /// A whole equipment element struct → "Name Quality+Plus" (the editor reads
    /// the element's `a`/`c`/`b` directly; the key field is ignored).
    EquipmentNode,
    /// Equipment *instance* id → look it up in `X.R` and name its type.
    EquipmentInstance,
    /// Monument id → `items::monument_name`.
    Monument,
    /// Might id → `items::might_name`.
    Might,
    /// Creation id → `items::creation_name`.
    Creation,
    /// SpaceDim element id → `items::spacedim_name`.
    SpaceDim,
    /// Physical-training id → `items::physical_training_name`.
    PhysicalTraining,
    /// Skill id → `items::skill_name`.
    Skill,
    /// Monster id → `items::monster_name`.
    Monster,
    /// Divinity Generator upgrade id → `items::divinity_upgrade_name`.
    DivinityUpgrade,
    /// Adventure-mode item id → `items::adventure_item_name`.
    AdventureItem,
    /// Adventure-mode enemy/entity id → `items::adventure_enemy_name`.
    AdventureEnemy,
    /// Adventure-mode class id → `items::adventure_class_name`.
    AdventureClass,
    /// A whole core element struct → "Enemy Quality" (e.g. "Slime SSS"); the
    /// editor reads the element's `a`/`d` directly. Like [`Resolve::EquipmentNode`].
    CoreNode,
    /// Research id → `model::research_name`.
    Research,
    /// Class id → `model::class_from_id`.
    Class,
    /// Element id → `model::element_from_id`.
    Element,
    /// Pet type id → `items::pet_type_name`.
    PetType,
    /// Elemental-form id → `items::elemental_form_name`.
    ElementalForm,
    /// Favorite/Hate campaign id, stored **offset by 1** (`0` = unset) →
    /// `items::campaign_type_name(id - 1)`.
    CampaignPref,
    /// Pet feeding-setting id → `items::feeding_setting_name`.
    FeedingSetting,
    /// Gem element id → `items::gem_element_name` (the full set incl.
    /// Dark/Light/Elemental/All — unlike [`Resolve::Element`], which is 0–4).
    GemElement,
}

/// One labeled field within a block element. `key` is the path *relative to the
/// element*, dot-joined for nested structs (`"w.d.b"`).
pub struct FieldLabel {
    pub key: &'static str,
    pub label: &'static str,
    /// If set, the field is an id the editor annotates with a resolved name.
    pub resolve: Option<Resolve>,
}

/// How to title each element of a block from one of its fields.
pub struct ElementName {
    /// Which field (relative key) holds the id/name.
    pub key: &'static str,
    pub resolve: Resolve,
}

/// A block of same-shaped data in the tree.
pub struct BlockSchema {
    /// Path prefix to the block (dotted segments from the root).
    pub base: &'static [&'static str],
    /// Singular display name for each element (e.g. "Pet").
    pub name: &'static str,
    /// Plural display name for the block/list container (e.g. "Pets").
    pub plural: &'static str,
    /// `true` when `base` is a list addressed by index (fields live at
    /// `base.<index>.key`); `false` for a single struct (`base.key`).
    pub is_list: bool,
    /// How to title each element (e.g. a pet by its name). `None` = just `[i]`.
    pub element_name: Option<ElementName>,
    /// The labeled fields of each element / of the struct.
    pub fields: &'static [FieldLabel],
}

/// A plain labeled field.
macro_rules! lbl {
    ($k:literal, $l:literal) => {
        FieldLabel { key: $k, label: $l, resolve: None }
    };
}

/// A labeled id field that resolves to a name.
macro_rules! lblr {
    ($k:literal, $l:literal, $r:expr) => {
        FieldLabel { key: $k, label: $l, resolve: Some($r) }
    };
}

/// Pets — `X.b.<index>` (with nested `w` dungeon/class sub-structs).
pub const PET_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Name"),
    lblr!("k", "Type Id", Resolve::PetType),
    lbl!("l", "Unlocked"),
    lbl!("E", "Growth (base)"),
    lbl!("d", "Growth Component (d)"),
    lbl!("e", "Growth Component (e)"),
    lbl!("f", "Growth Component (f)"),
    lbl!("n", "Growth Pool"),
    lbl!("g", "Normal Level"),
    lbl!("h", "Normal Exp (current)"),
    lbl!("j", "Normal Health"),
    lbl!("o", "Clone Physical"),
    lbl!("p", "Clone Mystic"),
    lbl!("q", "Clone Battle"),
    lbl!("r", "Clone HP"),
    lbl!("s", "Recovery Timer (ms)"),
    lbl!("v", "Team Slot"),
    lblr!("F", "Partner Type Id", Resolve::PetType),
    lbl!("G", "Partner Days"),
    lbl!("H", "Working Exp (ms)"),
    lblr!("y", "Elemental Form", Resolve::ElementalForm),
    lbl!("B", "Token Improved"),
    lblr!("t", "Favorite Camp", Resolve::CampaignPref),
    lblr!("u", "Hate Camp", Resolve::CampaignPref),
    lblr!("x", "Feeding Setting", Resolve::FeedingSetting),
    lbl!("A", "Vaccinated"),
    lbl!("C", "Skin Index"),
    lbl!("w", "Dungeon & Class"),
    lblr!("w.a", "Element Id", Resolve::Element),
    lbl!("w.b", "Dungeon Level"),
    lbl!("w.c", "Dungeon Exp (current)"),
    lbl!("w.d", "Class"),
    lblr!("w.d.a", "Class Id", Resolve::Class),
    lbl!("w.d.b", "Class Level"),
    lbl!("w.d.c", "Class Exp (current)"),
    lblr!("w.e", "Weapon (instance id)", Resolve::EquipmentInstance),
    lblr!("w.f", "Armor (instance id)", Resolve::EquipmentInstance),
    lblr!("w.g", "Accessory (instance id)", Resolve::EquipmentInstance),
];

/// Owned equipment instances — `X.R.<index>`.
pub const EQUIPMENT_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Type Id", Resolve::Equipment),
    lbl!("b", "Plus Level"),
    lbl!("c", "Quality"),
    lbl!("d", "Equip Ref (0 = unequipped)"),
    lbl!("e", "Plus Cap"),
    lbl!("f", "Gem Level"),
    lblr!("g", "Gem Element Id", Resolve::GemElement),
    lbl!("h", "Unique Instance Id"),
];

/// Material / item stacks — `X.Q.<index>`.
pub const MATERIAL_FIELDS: &[FieldLabel] =
    &[lblr!("a", "Item Id", Resolve::Material), lbl!("b", "Count")];

/// Gem inventory — `X.002.<index>`.
pub const GEM_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Element Id", Resolve::Element),
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

/// Adventure-mode inventory — `032.d.<index>` (`c`/`d` are 0, unlabeled).
pub const ADVENTURE_ITEM_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Item Id", Resolve::AdventureItem),
    lbl!("b", "Count"),
];

/// Adventure-mode cores — `032.G.<index>`. `b` (always 1) is unlabeled.
pub const CORE_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Enemy Id", Resolve::AdventureEnemy),
    lbl!("c", "Count"),
    lbl!("d", "Quality"),
];

/// Adventure-mode researches — `032.H.a.<index>`.
pub const RESEARCH_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Research Id", Resolve::Research),
    lbl!("b", "Level"),
    lbl!("f", "Max Level"),
    lbl!("c", "In Progress"),
    lbl!("d", "Progress"),
];

/// Creations — `i.<index>`.
pub const CREATION_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Creation Id", Resolve::Creation),
    lbl!("d", "Current Amount"),
    lbl!("e", "Clone Cost"),
    lbl!("g", "Total Created"),
    lbl!("i", "Next At"),
];

/// Monuments — `D.<index>`. The `e` sub-struct holds the monument's *upgrade*
/// (the level/next-at/spread that FINDINGS previously had as "unlocated").
pub const MONUMENT_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Monument Id", Resolve::Monument),
    lbl!("b", "Level"),
    lbl!("g", "Next At"),
    lbl!("h", "Spread"),
    lbl!("f", "Building"),
    lbl!("c", "Clones Allocated"),
    lbl!("d", "Progress"),
    lbl!("e", "Upgrade"),
    lbl!("e.b", "Upgrade Level"),
    lbl!("e.f", "Upgrade Next At"),
    lbl!("e.g", "Upgrade Spread"),
];

/// Mights — `V.<index>`.
pub const MIGHT_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Might Id", Resolve::Might),
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
    lblr!("a", "Element Id", Resolve::SpaceDim),
    lbl!("b", "Clones Allocated"),
    lbl!("c", "Level"),
    lbl!("d", "Next At"),
    lbl!("e", "Progress"),
    lbl!("f", "Spread"),
];

/// Physical conditioning exercises — `h.<index>`. The `d` field (always 0 so
/// far) is left unlabeled pending identification.
pub const PHYSICAL_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Training Id", Resolve::PhysicalTraining),
    lbl!("b", "Level"),
    lbl!("c", "Clones Allocated"),
];

/// Skills — `j.<index>`. The `e` sub-struct holds the "Special"-menu usage data:
/// `e.b` is the usage count that drives the training cap for this Skill and the
/// index-matched Physical. `e.c` (a small stable int) and the outer `d` stay
/// unlabeled pending identification.
pub const SKILL_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Skill Id", Resolve::Skill),
    lbl!("b", "Level"),
    lbl!("c", "Clones Allocated"),
    lbl!("e", "Usage"),
    lblr!("e.a", "Skill Id", Resolve::Skill),
    lbl!("e.b", "Usage Count"),
];

/// Monsters (fought for Battle/Divinity) — `k.<index>`.
pub const MONSTER_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Monster Id", Resolve::Monster),
    lbl!("b", "Defeated"),
    lbl!("c", "Clones Allocated"),
];

/// Divinity Generator upgrade tracks — `K.l.<index>` (0 = Capacity, 1 = Divinity
/// Gain, 2 = Converting Speed). `c`/`d`/`e`/`h` stay unlabeled pending ID.
pub const DIVINITY_UPGRADE_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Upgrade Id", Resolve::DivinityUpgrade),
    lbl!("b", "Level"),
    lbl!("f", "Next At"),
    lbl!("g", "Spread"),
];

/// Baal-Slayer (TBS) component levels — single struct at `S`.
pub const TBS_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Eyes Level"),
    lbl!("c", "Mouth Level"),
    lbl!("d", "Wings Level"),
    lbl!("e", "Tail Level"),
    lbl!("f", "Feet Level"),
];

/// Adventure-mode adventurer ("MVBattleStats") — single struct at `032.b`
/// (`KPJFCPPKHDL`). The same struct shape backs enemies too, hence `a` = entity.
pub const ADVENTURER_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Entity", Resolve::AdventureEnemy),
    lbl!("b", "Level"),
    lbl!("c", "Exp"),
    // `d` (BigDouble) feeds the Attack calc (`0.8 * d/5`); exact role unconfirmed.
    lbl!("d", "Unknown (d)"),
    lblr!("e", "Class", Resolve::AdventureClass),
    // `f` = per-class progression (`HGKLOMCJAIM`): one record per class the player
    // has leveled (class levels track independently). See CLASS_PROGRESSION_FIELDS.
    lbl!("f", "Class Progression"),
    lbl!("g", "Battle Skills"), // PGEICDFPINA = AdvBattleSkill instances
    // `h` (a second skill-id list) is omitted when empty — present only when the
    // adventurer has skills in that slot, so it is intentionally NOT labeled
    // (the registry test requires every labeled path to exist in the ref save).
    // `i` is the populated skill-id list (e.g. `19&6&48&5` = Dodge / Speed Boost
    // / Dual Wield / Drops Boost).
    lbl!("i", "Skill Ids (&-list)"),
    // `j`/`k` are stored BigDoubles with no in-class reads (live: 136 / 1,064,697)
    // — meaning unconfirmed. `l` tracks a running maximum of something (live 1923).
    lbl!("j", "Unknown (j)"),
    lbl!("k", "Unknown (k)"),
    lbl!("l", "Unknown (l)"),
    lbl!("m", "Equipment"), // DDKDNIFCAJO = adventure gear (same class as 032.c)
    lbl!("n", "Current HP"),       // clamped to max-HP method INJMAMDMHFJ()
    lbl!("o", "Current MP"),       // clamped to max-MP method AKAIHHFEFMM()
    lbl!("p", "Recovery timer"),   // >0 shows "Recovering"; 0 = active
    lbl!("q", "Screen X"),         // entity UI x-position (FLCAOMHAGOB, default 110)
    lbl!("r", "Screen Y"),         // entity UI y-position (NJHJAPPCPAA, default 150)
    lbl!("s", "Active Pill"),      // BEFDMHPNDHH = AdvPill buff (feeds Attack)
    lbl!("t", "Skill Loadout"),    // OKOCFJJNMAK = SetSkill assignments
];

/// Adventure-mode per-class progression — `032.b.f.<index>` (`HGKLOMCJAIM`).
/// One entry per class the player has leveled; class levels advance independently.
pub const CLASS_PROGRESSION_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Class", Resolve::AdventureClass),
    lbl!("b", "Level"),
    lbl!("c", "Exp"),
    lbl!("d", "Unknown (d)"), // small flag/counter (live 0/1)
];

/// Title each element from one of its fields (id → name).
const fn elem(key: &'static str, resolve: Resolve) -> Option<ElementName> {
    Some(ElementName { key, resolve })
}

/// Every block, consumed by the save editor to build tree labels.
pub const BLOCKS: &[BlockSchema] = &[
    BlockSchema { base: &["X", "b"], name: "Pet", plural: "Pets", is_list: true, element_name: elem("a", Resolve::Literal), fields: PET_FIELDS },
    BlockSchema { base: &["X", "R"], name: "Equipment", plural: "Equipment", is_list: true, element_name: elem("a", Resolve::EquipmentNode), fields: EQUIPMENT_FIELDS },
    BlockSchema { base: &["X", "Q"], name: "Material", plural: "Materials", is_list: true, element_name: elem("a", Resolve::Material), fields: MATERIAL_FIELDS },
    BlockSchema { base: &["X", "002"], name: "Gem", plural: "Gems", is_list: true, element_name: elem("a", Resolve::Element), fields: GEM_FIELDS },
    BlockSchema { base: &["X", "S"], name: "Dungeon Team", plural: "Dungeon Teams", is_list: true, element_name: elem("i", Resolve::Literal), fields: DUNGEON_TEAM_FIELDS },
    BlockSchema { base: &["X", "x"], name: "Campaign", plural: "Campaigns", is_list: true, element_name: None, fields: CAMPAIGN_FIELDS },
    BlockSchema { base: &["032", "H", "a"], name: "Research", plural: "Researches", is_list: true, element_name: elem("a", Resolve::Research), fields: RESEARCH_FIELDS },
    BlockSchema { base: &["032", "d"], name: "Adventure Item", plural: "Adventure Inventory", is_list: true, element_name: elem("a", Resolve::AdventureItem), fields: ADVENTURE_ITEM_FIELDS },
    BlockSchema { base: &["032", "G"], name: "Core", plural: "Cores", is_list: true, element_name: elem("a", Resolve::CoreNode), fields: CORE_FIELDS },
    BlockSchema { base: &["032", "b"], name: "Adventurer", plural: "Adventurer", is_list: false, element_name: None, fields: ADVENTURER_FIELDS },
    BlockSchema { base: &["032", "b", "f"], name: "Class Progression", plural: "Class Progression", is_list: true, element_name: elem("a", Resolve::AdventureClass), fields: CLASS_PROGRESSION_FIELDS },
    BlockSchema { base: &["i"], name: "Creation", plural: "Creations", is_list: true, element_name: elem("a", Resolve::Creation), fields: CREATION_FIELDS },
    BlockSchema { base: &["D"], name: "Monument", plural: "Monuments", is_list: true, element_name: elem("a", Resolve::Monument), fields: MONUMENT_FIELDS },
    BlockSchema { base: &["V"], name: "Might", plural: "Mights", is_list: true, element_name: elem("a", Resolve::Might), fields: MIGHT_FIELDS },
    BlockSchema { base: &["009", "b"], name: "SpaceDim Element", plural: "SpaceDim Elements", is_list: true, element_name: elem("a", Resolve::SpaceDim), fields: SPACEDIM_FIELDS },
    BlockSchema { base: &["h"], name: "Physical", plural: "Physical", is_list: true, element_name: elem("a", Resolve::PhysicalTraining), fields: PHYSICAL_FIELDS },
    BlockSchema { base: &["j"], name: "Skill", plural: "Skills", is_list: true, element_name: elem("a", Resolve::Skill), fields: SKILL_FIELDS },
    BlockSchema { base: &["k"], name: "Monster", plural: "Monsters", is_list: true, element_name: elem("a", Resolve::Monster), fields: MONSTER_FIELDS },
    BlockSchema { base: &["K", "l"], name: "Divinity Upgrade", plural: "Divinity Upgrades", is_list: true, element_name: elem("a", Resolve::DivinityUpgrade), fields: DIVINITY_UPGRADE_FIELDS },
    BlockSchema { base: &["S"], name: "Baal Slayer Parts", plural: "Baal Slayer Parts", is_list: false, element_name: None, fields: TBS_FIELDS },
];
