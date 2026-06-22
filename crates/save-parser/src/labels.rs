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
    /// Dungeon id → `items::dungeon_name` (C# enum `GFEKIABOPIH`).
    Dungeon,
    /// Fishing pond id → `items::pond_name` (C# enum `BAMKFONNEMP`).
    Pond,
    /// Museum statue id → `items::statue_name` (C# enum `JBGNCMHGOFI`).
    Statue,
    /// Village building/feature id → `items::village_building_name` (`IMBOLMEHKCG`).
    VillageBuilding,
    /// Ultimate Being id → `items::ultimate_being_name` (planet UBs, 1-5).
    UltimateBeing,
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
    /// Campaign-slot type id (`AGGDKICFOAI`, *no* offset) →
    /// `items::campaign_type_name(id)` (0 = Growth).
    CampaignType,
    /// Pet feeding-setting id → `items::feeding_setting_name`.
    FeedingSetting,
    /// Gem element id → `items::gem_element_name` (the full set incl.
    /// Dark/Light/Elemental/All — unlike [`Resolve::Element`], which is 0–4).
    GemElement,
    /// Challenge id (`OIDDHCOBPLG`) → `items::challenge_name`.
    Challenge,
    /// Challenge difficulty id (`HOLHIHDKBKA`) → `items::challenge_difficulty_name`.
    ChallengeDifficulty,
    /// Ultimate-Overflow upgrade type id (`IDFOIHJPCHP`) →
    /// `items::ultimate_overflow_upgrade_name`.
    UltimateOverflowUpgrade,
    /// RTI bonus stat-type id (`BDAFIPJBPFN`) → `items::rti_bonus_name`.
    RtiBonus,
}

/// The value type of a save field, declared once so the tree navigator and the
/// structured sections share one notion of how to edit and bound it.
///
/// This is the *value-type* axis (what the number means / how to constrain it).
/// It is distinct from the registry's edit-widget kind (Number/Bool/Text), which
/// is about how the raw-tree editor renders the input; an `Id` here is edited as
/// a number there but also carries a [`Resolve`] for naming.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldKind {
    /// An unsigned integer, optionally range-bounded (see [`FieldLabel::range`]).
    UInt,
    /// An id whose value resolves to a name (carries a [`Resolve`]).
    Id,
    /// A `True`/`False` boolean.
    Bool,
    /// Free text or an opaque / arbitrary-magnitude scalar.
    Text,
}

/// One labeled field within a block element. `key` is the path *relative to the
/// element*, dot-joined for nested structs (`"w.d.b"`).
pub struct FieldLabel {
    pub key: &'static str,
    pub label: &'static str,
    /// The value type — drives editing/validation in both the tree and sections.
    pub kind: FieldKind,
    /// Inclusive `(min, max)` bound for a `UInt`, enforced everywhere the field
    /// is edited. `None` = unbounded.
    pub range: Option<(u32, u32)>,
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

/// A plain labeled field. Kind/range default to `Text`/unbounded — blocks that
/// need typed bounds are declared with [`save_block!`] instead.
macro_rules! lbl {
    ($k:literal, $l:literal) => {
        FieldLabel { key: $k, label: $l, kind: FieldKind::Text, range: None, resolve: None }
    };
}

/// A labeled id field that resolves to a name.
macro_rules! lblr {
    ($k:literal, $l:literal, $r:expr) => {
        FieldLabel { key: $k, label: $l, kind: FieldKind::Id, range: None, resolve: Some($r) }
    };
}

/// Declare a block's fields **once** as a canonical per-block enum plus the
/// `&[FieldLabel]` slice the registry consumes — the realization of the
/// type-driven model refactor (Option B). Each field's key, label, kind, range,
/// and resolve are stated in a single row; the generated enum's `match` arms are
/// exhaustive, so adding a field without specifying all of them is a compile
/// error, and the model's `from_node` reads the same `key()` constants — so the
/// three former copies (model parse, label table, section) can no longer drift.
///
/// The enum is the type-safe handle sections use (`EquipField::Enchant.range()`)
/// instead of bare string keys.
macro_rules! save_block {
    (
        $(#[$meta:meta])*
        $enum:ident => $slice:ident;
        $( $variant:ident : $key:literal, $label:literal, $kind:expr, $range:expr, $resolve:expr ; )+
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum $enum { $( $variant ),+ }

        impl $enum {
            /// Every field of the block, in declaration order.
            pub const ALL: &'static [$enum] = &[ $( $enum::$variant ),+ ];

            /// Raw key relative to the block element.
            pub const fn key(self) -> &'static str {
                match self { $( $enum::$variant => $key ),+ }
            }

            /// Canonical short display label.
            pub const fn label(self) -> &'static str {
                match self { $( $enum::$variant => $label ),+ }
            }

            /// The value type.
            pub const fn kind(self) -> FieldKind {
                match self { $( $enum::$variant => $kind ),+ }
            }

            /// Inclusive `(min, max)` bound, if any.
            pub const fn range(self) -> Option<(u32, u32)> {
                match self { $( $enum::$variant => $range ),+ }
            }

            /// Id-name resolution hint, if any.
            pub const fn resolve(self) -> Option<Resolve> {
                match self { $( $enum::$variant => $resolve ),+ }
            }

            /// Clamp a value into this field's range (identity if unbounded).
            pub const fn clamp(self, v: u32) -> u32 {
                match self.range() {
                    Some((lo, _)) if v < lo => lo,
                    Some((_, hi)) if v > hi => hi,
                    _ => v,
                }
            }
        }

        /// Label slice for the registry — generated from the same declaration.
        pub const $slice: &[FieldLabel] = &[ $(
            FieldLabel { key: $key, label: $label, kind: $kind, range: $range, resolve: $resolve },
        )+ ];
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

save_block! {
    /// Owned equipment instances — `X.R.<index>`. The pilot block for the
    /// type-driven refactor: model parse, label table, and the equipment section
    /// all derive from this one declaration. Ranges per the in-game caps:
    /// enchant 0–20, quality 0–8 (SSS), plus 0–30 (only Candy Cane reaches +30;
    /// every other item caps at +20 — the field bound is the max across all gear).
    EquipField => EQUIPMENT_FIELDS;
    TypeId:     "a", "Type Id",                    FieldKind::Id,   None,         Some(Resolve::Equipment);
    Plus:       "b", "Plus Level",                 FieldKind::UInt, Some((0, 30)), None;
    Quality:    "c", "Quality",                    FieldKind::UInt, Some((0, 8)),  None;
    EquipRef:   "d", "Equip Ref (0 = unequipped)", FieldKind::UInt, None,         None;
    Enchant:    "e", "Enchant Level",              FieldKind::UInt, Some((0, 20)), None;
    GemLevel:   "f", "Gem Level",                  FieldKind::UInt, None,         None;
    GemElement: "g", "Gem Element Id",             FieldKind::Id,   None,         Some(Resolve::GemElement);
    InstanceId: "h", "Unique Instance Id",         FieldKind::UInt, None,         None;
}

save_block! {
    /// Material / item stacks — `X.Q.<index>`.
    MaterialField => MATERIAL_FIELDS;
    Item:  "a", "Item Id", FieldKind::Id,   None, Some(Resolve::Material);
    Count: "b", "Count",   FieldKind::Text, None, None;
}

save_block! {
    /// Gem inventory — `X.002.<index>`.
    GemField => GEM_FIELDS;
    Element: "a", "Element Id", FieldKind::Id,   None, Some(Resolve::Element);
    Level:   "b", "Level",      FieldKind::UInt, None, None;
    Count:   "c", "Count",      FieldKind::Text, None, None;
}

save_block! {
    /// Persistent dungeon teams — `X.S.<index>` (`PCDCANGLENI`). Static team
    /// settings. Per-depth difficulty: `e`=D1, `f`=D2, `g`=D3 (player-confirmed;
    /// D4 not cleanly placeable — `h` is a list, not a difficulty int). `c` = the
    /// team's loot/inventory (see PendingLootField); not rolled until the run ends.
    DungeonTeamField => DUNGEON_TEAM_FIELDS;
    Dungeon:      "b", "Dungeon Id",          FieldKind::Id,   None, Some(Resolve::Dungeon);
    Depth:        "d", "Depth",               FieldKind::UInt, None, None;
    D1Difficulty: "e", "D1 Difficulty",       FieldKind::UInt, None, None;
    D2Difficulty: "f", "D2 Difficulty",       FieldKind::UInt, None, None;
    D3Difficulty: "g", "D3 Difficulty",       FieldKind::UInt, None, None;
    DungeonName:  "i", "Dungeon Name",        FieldKind::Text, None, None;
    Members:      "a", "Member Pet Type Ids", FieldKind::Text, None, None;
    PendingLoot:  "c", "Pending Loot",        FieldKind::Text, None, None;
}

save_block! {
    /// A team's pending-loot / inventory entry — `X.S.<i>.c.<index>`
    /// (`GCJMGGFGKBN`, same shape as the material inventory). `a`=item id, `b`=count.
    PendingLootField => PENDING_LOOT_FIELDS;
    Item:  "a", "Item Id", FieldKind::Id,   None, Some(Resolve::Material);
    Count: "b", "Count",   FieldKind::Text, None, None;
}

save_block! {
    /// The Challenge team — `X.Z` (a single `PCDCANGLENI`, C# `NMGIGAGPLCL`). `a`
    /// = members (`&`-joined pet ids). Its inventory `c` is empty in the ref save
    /// so unlabeled. (Challenges have no difficulty/depth/timer.)
    ChallengeTeamField => CHALLENGE_TEAM_FIELDS;
    Members: "a", "Member Pet Type Ids", FieldKind::Text, None, None;
}

save_block! {
    /// Active dungeon runs — `X.P.<index>` (`MKDNAHGDLPI`). `a`=dungeon id,
    /// `b`=elapsed ms (a float counting up to `c`), `c`=target duration ms
    /// (43,200,000 = 12 h), `d`=depth, `f`=team index (ties the run to its `X.S`
    /// team). `e`/`j` are RNG seeds. Setting `b` ≥ `c` completes the run.
    /// (Verified field-by-field on the reference save's 3 active runs.)
    ActiveDungeonField => ACTIVE_DUNGEON_FIELDS;
    DungeonId:      "a", "Dungeon Id",           FieldKind::Id,   None, Some(Resolve::Dungeon);
    Elapsed:        "b", "Elapsed (ms)",         FieldKind::Text, None, None;
    TargetDuration: "c", "Target Duration (ms)", FieldKind::Text, None, None;
    Depth:          "d", "Depth",                FieldKind::UInt, None, None;
    RngE:           "e", "RNG seed (e)",         FieldKind::Text, None, None;
    TeamIndex:      "f", "Team Index",           FieldKind::UInt, None, None;
    RngJ:           "j", "RNG seed (j)",         FieldKind::Text, None, None;
}

save_block! {
    /// Museum statues — `024.f.a.<index>` (`MCEIHMMCDNH`). `a` = level (20 when
    /// maxed), `b` = statue id (event commemoratives; you can own two of each).
    MuseumStatueField => MUSEUM_STATUE_FIELDS;
    Level:  "a", "Level",  FieldKind::UInt, None, None;
    Statue: "b", "Statue", FieldKind::Id,   None, Some(Resolve::Statue);
}

// Planet system — `root.T` (`AIDFNOPNJGK`, marker "Planet"). `d` = planet level
// (drives the planet name tiers; level 1-5 from feeding planet/earthlike/sun/
// solar-system/universe, then +1 per Ultimate Universe Challenge; the effective
// level for power adds the UUC count on top). `h` = unspent Baal Power
// (player-confirmed; spent on Light Clones that fight the UBs). The Planet
// Multiplier is computed (base 100% + Powersurge `T.k` + UB-kill `T.f`), not stored.
save_block! {
    /// Planet — top-level scalars (`T`). `d` = planet level, `h` = unspent Baal
    /// Power. The Planet Multiplier is computed (base 100% + Powersurge + UB-kill
    /// contributions), not stored. (Verified on the reference save: T.d=7, T.h=0.)
    PlanetField => PLANET_FIELDS;
    Level:     "d", "Planet Level",       FieldKind::UInt, None, None;
    BaalPower: "h", "Unspent Baal Power", FieldKind::Text, None, None;
}

save_block! {
    /// Planet — per-UB state — `T.k.<index>` (`FPBMNCNKPHN`), one per UB. `c` = UB
    /// id, `b` = kill/defeat count that DRIVES the "Multi from Ultimate Beings"
    /// (a fixed % per defeat: Planet Eater 1% / Godly Tribunal 12% / Living Sun
    /// 21% / God Above All 32% / ITRTG 45%); `a` = a per-UB state value (~100,
    /// exact role unconfirmed). Distinct from `T.f.b`, the per-spawn kill count.
    UbMultField => UB_MULTIPLIER_FIELDS;
    Ub:        "c", "UB",           FieldKind::Id,   None, Some(Resolve::UltimateBeing);
    KillCount: "b", "Kill Count",   FieldKind::UInt, None, None;
    State:     "a", "State (~100)", FieldKind::Text, None, None;
}

save_block! {
    /// Planet — Ultimate Beings — `T.f.<index>` (`CEFAAPALBMD`). The 5 UBs that
    /// attack on staggered spawn timers. `c` = UB id (1 Planet Eater … 5 ITRTG),
    /// `b` = kill count, `d` = spawn countdown ms (counts DOWN; spawns at ≤0 —
    /// set 0 to force a spawn), `e` = alive flag, `f` = god power gained.
    UbField => ULTIMATE_BEING_FIELDS;
    Ub:             "c", "UB",                   FieldKind::Id,   None, Some(Resolve::UltimateBeing);
    KillCount:      "b", "Kill Count",           FieldKind::UInt, None, None;
    SpawnCountdown: "d", "Spawn Countdown (ms)", FieldKind::Text, None, None;
    Alive:          "e", "Alive",                FieldKind::Bool, None, None;
    GodPowerGained: "f", "God Power Gained",     FieldKind::Text, None, None;
}

save_block! {
    /// Village building-state list — `024.a.<index>` (`AFELNLGMCAB`, marker
    /// "VillageBuilding"). One entry per building feature, keyed by `g` = building
    /// type (`IMBOLMEHKCG`). `c` = level, `f` = assigned pet; other fields are
    /// unlock/flag state (mostly default in the ref save).
    VillageBuildingField => VILLAGE_BUILDING_FIELDS;
    BuildingType: "g", "Building Type", FieldKind::Id, None, Some(Resolve::VillageBuilding);
}

save_block! {
    /// Worker buildings — Material Factory `024.g` (`CHDGDEINMHO`) and Alchemy Hut
    /// `024.h` (`GABIFCBBMPH`), both extending `ANECMNGBLNI`. `a` = level, `e` =
    /// manager slot (pet type id; 999 = empty), `d` = worker pet-slot list.
    WorkerBuildingField => WORKER_BUILDING_FIELDS;
    Level:   "a", "Level",                 FieldKind::UInt, None, None;
    Manager: "e", "Manager (pet type id)", FieldKind::Id,   None, Some(Resolve::PetType);
}

save_block! {
    /// A worker building's pet slot — `024.{g,h}.d.<index>` (`FGKIILDKMEA`). `a` =
    /// pet type id (999 = empty), `d` = work progress/exp. `b`/`c` are the
    /// in-progress craft (nested sub-structs, unconfirmed).
    WorkerSlotField => WORKER_SLOT_FIELDS;
    PetType:      "a", "Pet Type Id",   FieldKind::Id,   None, Some(Resolve::PetType);
    WorkProgress: "d", "Work Progress", FieldKind::Text, None, None;
}

save_block! {
    /// Pet Village Tavern — `024.b` (`IOBPPFGEBCD`). Runs pet quests. `b` = level,
    /// `d` = Quest Points, `i` = quests/day, `j` = max concurrent quests, `u` =
    /// Tavern Keeper slot (999 = empty), `x` = favorite quests (`&`-list). `a`/`t`
    /// are quest lists; `c` (upgrade-elapsed timer) is empty when not upgrading so
    /// unlabeled; other scalars unconfirmed.
    TavernField => TAVERN_FIELDS;
    Level:          "b", "Level",                    FieldKind::UInt, None, None;
    QuestPoints:    "d", "Quest Points",             FieldKind::Text, None, None;
    QuestsPerDay:   "i", "Quests Per Day",           FieldKind::UInt, None, None;
    MaxConcurrent:  "j", "Max Concurrent Quests",    FieldKind::UInt, None, None;
    TavernKeeper:   "u", "Tavern Keeper (slot)",     FieldKind::Text, None, None;
    FavoriteQuests: "x", "Favorite Quests (&-list)", FieldKind::Text, None, None;
}

save_block! {
    /// Pet Village Dojo — `024.d` (`JKDCFKCLCKH`). `b` = level, `c` = elapsed
    /// upgrade time (`LDMJEPGEOME`; accumulates to target then resets — set large
    /// to force-complete). The four 999 fields (`s`/`t`/`v`/`w`) are its 4 pet
    /// slots; other fields are per-stat training buffs (unconfirmed).
    DojoField => DOJO_FIELDS;
    Level:          "b", "Level",                FieldKind::UInt, None, None;
    UpgradeElapsed: "c", "Upgrade Elapsed (ms)", FieldKind::Text, None, None;
}

save_block! {
    /// Pet Village Strategy Room — `024.e` (`CJACGIIPNIG`). The three multipliers
    /// were player-confirmed by tweaking them in-game. `c` accumulates to target
    /// then resets; set large to finish.
    StrategyRoomField => STRATEGY_ROOM_FIELDS;
    Level:          "b", "Level",                 FieldKind::UInt, None, None;
    UpgradeElapsed: "c", "Upgrade Elapsed (ms)",  FieldKind::Text, None, None;
    PhysicalMulti:  "e", "Physical Multi %",      FieldKind::Text, None, None;
    MysticMulti:    "f", "Mystic Multi %",        FieldKind::Text, None, None;
    BattleMulti:    "g", "Battle Multi %",        FieldKind::Text, None, None;
    PetSlots:       "h", "Pet Slots (&-list, 8)", FieldKind::Text, None, None;
}

save_block! {
    /// Fishing block — `root.025` (`KACINBICCNH`). `a` = Fish Power (labeled in
    /// Resources), `b` = current exp (resets on level-up), `c` = level, `d`/`e` =
    /// selected bait/rod (material ids), `f` = current pond. Lists g/h/i =
    /// rods/bait/fish (see below).
    FishingField => FISHING_FIELDS;
    Exp:          "b", "Fishing Exp",   FieldKind::Text, None, None;
    Level:        "c", "Fishing Level", FieldKind::UInt, None, None;
    SelectedBait: "d", "Selected Bait", FieldKind::Id,   None, Some(Resolve::Material);
    SelectedRod:  "e", "Selected Rod",  FieldKind::Id,   None, Some(Resolve::Material);
    CurrentPond:  "f", "Current Pond",  FieldKind::Id,   None, Some(Resolve::Pond);
}

save_block! {
    /// Owned fishing rods — `025.g.<index>` (`ANCPDAFDBPP`). `a` = rod material id
    /// (500-504), `b` = owned (0/1).
    FishingRodField => FISHING_ROD_FIELDS;
    Rod:   "a", "Rod",   FieldKind::Id,   None,         Some(Resolve::Material);
    Owned: "b", "Owned", FieldKind::UInt, Some((0, 1)), None;
}

save_block! {
    /// Bait stacks — `025.h.<index>` (`ANCPDAFDBPP`). `a` = bait material id
    /// (520-524), `b` = count.
    FishingBaitField => FISHING_BAIT_FIELDS;
    Bait:  "a", "Bait",  FieldKind::Id,   None, Some(Resolve::Material);
    Count: "b", "Count", FieldKind::Text, None, None;
}

save_block! {
    /// Fish-caught records — `025.i.<index>` (`PNPLCJJOPIO`). `a` = fish material
    /// id (525+), `c` = lifetime caught count.
    FishingFishField => FISHING_FISH_FIELDS;
    Fish:   "a", "Fish",   FieldKind::Id,   None, Some(Resolve::Material);
    Caught: "c", "Caught", FieldKind::Text, None, None;
}

save_block! {
    /// Campaign slots — `X.x.<index>` (`FMOLELEHAFD`). One persistent slot per
    /// campaign type. `a` = campaign **type** (`AGGDKICFOAI`, 0 = Growth — NOT a
    /// slot index; verified on the reference save where the 8 slots carry types
    /// 0,1,2,3,4,5,6,8). `c` = elapsed ms (a float, counts up to `e`), `e` =
    /// target duration ms (43,200,000 = 12 h) — same elapsed/target shape as a
    /// dungeon run, so setting `c` = `e` completes the campaign. `d` = `&`-joined
    /// pet type ids; `f` = bonus.
    CampaignField => CAMPAIGN_FIELDS;
    CampaignType:   "a", "Campaign Type",        FieldKind::Id,   None, Some(Resolve::CampaignType);
    Elapsed:        "c", "Elapsed (ms)",         FieldKind::Text, None, None;
    PetTypeIds:     "d", "Pet Type Ids",         FieldKind::Text, None, None;
    TargetDuration: "e", "Target Duration (ms)", FieldKind::Text, None, None;
    Bonus:          "f", "Bonus",                FieldKind::Text, None, None;
}

save_block! {
    /// Adventure-mode inventory — `032.d.<index>` (`c`/`d` are 0, unlabeled).
    /// `a` = item id, `b` = count (verified varied counts on the reference save).
    AdventureItemField => ADVENTURE_ITEM_FIELDS;
    Item:  "a", "Item Id", FieldKind::Id,   None, Some(Resolve::AdventureItem);
    Count: "b", "Count",   FieldKind::Text, None, None;
}

save_block! {
    /// Adventure-mode cores — `032.G.<index>`. `a` = enemy id, `c` = count,
    /// `d` = quality (0–8 = F…SSS); `b` (always 1) is unlabeled.
    CoreField => CORE_FIELDS;
    Enemy:   "a", "Enemy Id", FieldKind::Id,   None,         Some(Resolve::AdventureEnemy);
    Count:   "c", "Count",    FieldKind::Text, None,         None;
    Quality: "d", "Quality",  FieldKind::UInt, Some((0, 8)), None;
}

save_block! {
    /// Adventure-mode researches — `032.H.a.<index>`.
    ResearchField => RESEARCH_FIELDS;
    Research:   "a", "Research Id", FieldKind::Id,   None, Some(Resolve::Research);
    Level:      "b", "Level",       FieldKind::UInt, None, None;
    MaxLevel:   "f", "Max Level",   FieldKind::UInt, None, None;
    InProgress: "c", "In Progress", FieldKind::Text, None, None;
    Progress:   "d", "Progress",    FieldKind::Text, None, None;
}

save_block! {
    /// Creations — `i.<index>`.
    CreationField => CREATION_FIELDS;
    Creation:      "a", "Creation Id",    FieldKind::Id,   None, Some(Resolve::Creation);
    CurrentAmount: "d", "Current Amount", FieldKind::Text, None, None;
    CloneCost:     "e", "Clone Cost",     FieldKind::Text, None, None;
    TotalCreated:  "g", "Total Created",  FieldKind::Text, None, None;
    NextAt:        "i", "Next At",        FieldKind::Text, None, None;
}

save_block! {
    /// Monuments — `D.<index>`. The `e` sub-struct holds the monument's *upgrade*
    /// (the level/next-at/spread that FINDINGS previously had as "unlocated").
    MonumentField => MONUMENT_FIELDS;
    Monument:        "a",   "Monument Id",      FieldKind::Id,   None, Some(Resolve::Monument);
    Level:           "b",   "Level",            FieldKind::UInt, None, None;
    NextAt:          "g",   "Next At",          FieldKind::Text, None, None;
    Spread:          "h",   "Spread",           FieldKind::Text, None, None;
    Building:        "f",   "Building",         FieldKind::Text, None, None;
    ClonesAllocated: "c",   "Clones Allocated", FieldKind::Text, None, None;
    Progress:        "d",   "Progress",         FieldKind::Text, None, None;
    Upgrade:         "e",   "Upgrade",          FieldKind::Text, None, None;
    UpgradeLevel:    "e.b", "Upgrade Level",    FieldKind::UInt, None, None;
    UpgradeNextAt:   "e.f", "Upgrade Next At",  FieldKind::Text, None, None;
    UpgradeSpread:   "e.g", "Upgrade Spread",   FieldKind::Text, None, None;
}

save_block! {
    /// Mights — `V.<index>`.
    MightField => MIGHT_FIELDS;
    Might:           "a", "Might Id",              FieldKind::Id,   None, Some(Resolve::Might);
    Level:           "b", "Level",                 FieldKind::UInt, None, None;
    NextAt:          "m", "Next At",               FieldKind::Text, None, None;
    Spread:          "n", "Spread",                FieldKind::Text, None, None;
    Special:         "e", "Special (Unleash)",     FieldKind::Text, None, None;
    BaseDuration:    "g", "Base Duration (s)",     FieldKind::Text, None, None;
    UnleashRecovery: "i", "Unleash HP Recovery %", FieldKind::Text, None, None;
    UnleashAttack:   "j", "Unleash Attack %",      FieldKind::Text, None, None;
    UnleashMystic:   "k", "Unleash Mystic %",      FieldKind::Text, None, None;
}

save_block! {
    /// SpaceDim / Light-Dimension elements — `009.b.<index>`.
    SpaceDimField => SPACEDIM_FIELDS;
    Element:         "a", "Element Id",       FieldKind::Id,   None, Some(Resolve::SpaceDim);
    ClonesAllocated: "b", "Clones Allocated", FieldKind::Text, None, None;
    Level:           "c", "Level",            FieldKind::UInt, None, None;
    NextAt:          "d", "Next At",          FieldKind::Text, None, None;
    Progress:        "e", "Progress",         FieldKind::Text, None, None;
    Spread:          "f", "Spread",           FieldKind::Text, None, None;
}

save_block! {
    /// Physical conditioning exercises — `h.<index>`. The `d` field (always 0 so
    /// far) is left unlabeled pending identification.
    PhysicalField => PHYSICAL_FIELDS;
    Training:        "a", "Training Id",      FieldKind::Id,   None, Some(Resolve::PhysicalTraining);
    Level:           "b", "Level",            FieldKind::UInt, None, None;
    ClonesAllocated: "c", "Clones Allocated", FieldKind::Text, None, None;
}

save_block! {
    /// Skills — `j.<index>`. The `e` sub-struct holds the "Special"-menu usage
    /// data: `e.b` is the usage count that drives the training cap for this Skill
    /// and the index-matched Physical. `e.c` (a small stable int) and the outer
    /// `d` stay unlabeled pending identification.
    SkillField => SKILL_FIELDS;
    Skill:           "a",   "Skill Id",         FieldKind::Id,   None, Some(Resolve::Skill);
    Level:           "b",   "Level",            FieldKind::UInt, None, None;
    ClonesAllocated: "c",   "Clones Allocated", FieldKind::Text, None, None;
    Usage:           "e",   "Usage",            FieldKind::Text, None, None;
    UsageSkill:      "e.a", "Skill Id",         FieldKind::Id,   None, Some(Resolve::Skill);
    UsageCount:      "e.b", "Usage Count",      FieldKind::Text, None, None;
}

save_block! {
    /// Monsters (fought for Battle/Divinity) — `k.<index>`.
    MonsterField => MONSTER_FIELDS;
    Monster:         "a", "Monster Id",       FieldKind::Id,   None, Some(Resolve::Monster);
    Defeated:        "b", "Defeated",         FieldKind::Text, None, None;
    ClonesAllocated: "c", "Clones Allocated", FieldKind::Text, None, None;
}

save_block! {
    /// Divinity Generator upgrade tracks — `K.l.<index>` (0 = Capacity, 1 =
    /// Divinity Gain, 2 = Converting Speed). `c`/`d`/`e`/`h` stay unlabeled pending ID.
    DivinityUpgradeField => DIVINITY_UPGRADE_FIELDS;
    Upgrade: "a", "Upgrade Id", FieldKind::Id,   None, Some(Resolve::DivinityUpgrade);
    Level:   "b", "Level",      FieldKind::UInt, None, None;
    NextAt:  "f", "Next At",    FieldKind::Text, None, None;
    Spread:  "g", "Spread",     FieldKind::Text, None, None;
}

save_block! {
    /// Baal-Slayer (TBS) component levels — single struct at `S`.
    TbsField => TBS_FIELDS;
    Eyes:  "b", "Eyes Level",  FieldKind::UInt, None, None;
    Mouth: "c", "Mouth Level", FieldKind::UInt, None, None;
    Wings: "d", "Wings Level", FieldKind::UInt, None, None;
    Tail:  "e", "Tail Level",  FieldKind::UInt, None, None;
    Feet:  "f", "Feet Level",  FieldKind::UInt, None, None;
}

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

/// Statistics block — `root.x` (`LLMCMCKAABP`, marker "Statistic"): a large bag
/// of ~360 numeric-key counters/totals. The confirmed gameplay trackers are
/// labeled here (mirroring `model::trackers`, diff-confirmed against tooltips);
/// the rest stay raw. Per-pet trackers feed the matching pet's campaign bonus.
pub const STATISTICS_FIELDS: &[FieldLabel] = &[
    lbl!("013", "AFK clones killed"),
    lbl!("071", "Lucky Draws opened"),
    lbl!("074", "Crystal power"),
    lbl!("078", "Dungeon bosses defeated"),
    lbl!("079", "Dungeon enemies defeated"),
    lbl!("080", "Dungeon rooms beaten"),
    lbl!("089", "Chocobear banked hours"),
    lbl!("129", "Total might"),
    lbl!("169", "Pandora feedings (this rebirth)"),
    lbl!("185", "Earth Eater planets (lifetime)"),
    lbl!("186", "Aether Ring lvl / Delirious Essence wins"),
    lbl!("216", "Pignata bashes"),
    lbl!("218", "God Power campaign hours"),
    lbl!("234", "Meteor campaign hours"),
    lbl!("259", "Caterpillar materials upgraded"),
    lbl!("270", "Pet stones via Baal Power (Vermillion Pheasant prog.)"),
    lbl!("310", "Mule quests"),
    lbl!("311", "Gold Dragon bonus growth"),
    lbl!("324", "Serow items saved"),
    lbl!("336", "Bag bonus growth"),
    // Day-challenge high scores — these (not the x.242 completion count) drive
    // each Day challenge's ChP reward (per the `OIHGOPGKAJO` score formulas in
    // KPLPGPEOFNB.cs ~6190). Editing these is how you change a Day challenge's ChP.
    lbl!("045", "Day Baal Challenge score (ChP basis)"),
    lbl!("047", "Day Universe Challenge score (ChP basis)"),
    lbl!("049", "Day Pet Challenge highest multiplier (ChP basis)"),
    lbl!("065", "Day Might Challenge score (ChP basis)"),
    lbl!("068", "Day No Divinity Challenge score (ChP basis)"),
    lbl!("134", "Road to Infinity — highest P.Baal (ChP basis)"),
    lbl!("304", "Day Extreme Building Challenge score (ChP basis)"),
];

save_block! {
    /// Per-challenge completion record (`KPLPGPEOFNB`), one per element of the
    /// `root.x.242` list. `a` is the challenge id, `b` the lifetime completion
    /// count (shown in the Challenges menu), `c` the difficulty, `d` an ms epoch
    /// (last completion time — inferred from per-challenge recency vs. count),
    /// `e` a UI sort flag. Validated against an in-game capture 2026-06-20.
    ChallengeCompletionField => CHALLENGE_COMPLETION_FIELDS;
    Challenge:     "a", "Challenge",          FieldKind::Id,   None, Some(Resolve::Challenge);
    Completions:   "b", "Completions",        FieldKind::Text, None, None;
    Difficulty:    "c", "Difficulty",         FieldKind::Id,   None, Some(Resolve::ChallengeDifficulty);
    LastCompleted: "d", "Last Completed (ms)", FieldKind::Text, None, None;
    Flag:          "e", "Flag (e)",           FieldKind::Text, None, None;
}

save_block! {
    /// Overflow-Point upgrade levels (`HNFHEBJIPEL`, `root.013`). Each stored
    /// field is the bought upgrade amount; the in-game effect getter adds a base.
    /// Labels are the literal "OfP …" names from the Challenge-Points debug tooltip
    /// (`LLMCMCKAABP.cs:4063`), mapped to keys via each getter's field
    /// (`HNFHEBJIPEL.cs:39–63`). Field `h` has no getter/label there (vestigial).
    OfpUpgradeField => OFP_UPGRADE_FIELDS;
    BlackHole:        "a", "OfP Black Hole",            FieldKind::Text, None, None;
    BlackHoleUpgrade: "b", "OfP Black Hole Upgrade",    FieldKind::Text, None, None;
    GemCap:           "c", "OfP Gem Cap",               FieldKind::Text, None, None;
    GemGain:          "d", "OfP Gem Gain",              FieldKind::Text, None, None;
    V2AutoKill:       "e", "OfP V2 Auto Kill",          FieldKind::Text, None, None;
    HpRegen:          "f", "OfP Hp Regen",              FieldKind::Text, None, None;
    CrystalPower:     "g", "OfP Crystal Power",         FieldKind::Text, None, None;
    Vestigial:        "h", "OfP Upgrade (h, unlabeled)", FieldKind::Text, None, None;
    CreatingStat:     "i", "OfP Creating Stat",         FieldKind::Text, None, None;
    Powersurge:       "j", "OfP Powersurge",            FieldKind::Text, None, None;
    CreationCount:    "k", "OfP Creation Count",        FieldKind::Text, None, None;
    MightSpeed:       "l", "OfP Might Speed",           FieldKind::Text, None, None;
    StatsMulti:       "m", "OfP Stats Multi",           FieldKind::Text, None, None;
    SpaceDim:         "n", "OfP Space Dim",             FieldKind::Text, None, None;
}

save_block! {
    /// RTI (Road to Infinity) bonus entry (`HEIPGLPOGEJ`, marker `RtiElement`; one
    /// per element of the `root.014.a` list — 10 entries, one per `BDAFIPJBPFN`
    /// stat type). `a` = stat type, `e` = elapsed timer. `b` feeds the "Increases
    /// your <stat> by …" tooltip (the stored bonus amount); `c`/`d`/`g`/`h` are
    /// per-type values not separately anchored — labeled neutrally.
    RtiBonusField => RTI_BONUS_FIELDS;
    BonusType:   "a", "Bonus Type",   FieldKind::Id,   None, Some(Resolve::RtiBonus);
    BonusAmount: "b", "Bonus Amount", FieldKind::Text, None, None;
    ValueC:      "c", "Value (c)",    FieldKind::Text, None, None;
    ValueD:      "d", "Value (d)",    FieldKind::Text, None, None;
    Elapsed:     "e", "Elapsed (ms)", FieldKind::Text, None, None;
    ValueG:      "g", "Value (g)",    FieldKind::Text, None, None;
    ValueH:      "h", "Value (h)",    FieldKind::Text, None, None;
}

save_block! {
    /// Ultimate-Overflow upgrade entry (`FDJCCPFCJAO`, one per element of the
    /// `root.029.a` list; parent `CDNMNLIAPKA` marker `UltimateOverflowBoosts`).
    /// `a` = upgrade type (`IDFOIHJPCHP`), `b` = bought level. The boosts bought
    /// with Ultimate Overflow Points (the fixture holds all 6 types at 0).
    UofpUpgradeField => UOFP_UPGRADE_FIELDS;
    UpgradeType: "a", "Upgrade Type", FieldKind::Id,   None, Some(Resolve::UltimateOverflowUpgrade);
    Level:       "b", "Level",        FieldKind::UInt, None, None;
}

save_block! {
    /// Challenge-Point upgrade levels — scalar fields directly on `root.X`
    /// (`MLILKGIALMB`, the `FIHAENJIDAO` accessor). Names are the literal "Chp …"
    /// labels from the Challenge-Points debug tooltip (`LLMCMCKAABP.cs:4063`);
    /// each maps to a save key in `MLILKGIALMB.EBOFJJHOOLP` (real deserializer
    /// line 10036 — the `n19`/`n41`/`-36` copies elsewhere are decoys). Total ChP
    /// spent = Σ(level × cost) per `MLILKGIALMB.cs:894`; stored value = bought
    /// level. `035`/`038` are bools.
    ChpUpgradeField => CHP_UPGRADE_FIELDS;
    PlanetLevel:        "E",   "ChP Planet Level",                         FieldKind::UInt, None, None;
    DivinityBoost:      "I",   "ChP Divinity boost",                       FieldKind::UInt, None, None;
    DamageReductionUbs: "D",   "ChP Damage Reduction UBs",                 FieldKind::UInt, None, None;
    FasterUbSpawn:      "041", "ChP Faster UB spawn",                      FieldKind::UInt, None, None;
    CrystalUpgrade:     "G",   "ChP Crystal Upgrade boost",                FieldKind::UInt, None, None;
    DamageBoostV2s:     "H",   "ChP Damage Boost V2s",                     FieldKind::UInt, None, None;
    CpBoost:            "J",   "ChP CP boost",                             FieldKind::UInt, None, None;
    CrystalSacrifice:   "039", "ChP Crystal Sacrifice boost",             FieldKind::UInt, None, None;
    BsBoost:            "029", "ChP BS boost",                             FieldKind::UInt, None, None;
    CsBoost:            "030", "ChP CS boost",                             FieldKind::UInt, None, None;
    TbsLevelLoss:       "K",   "ChP TBS Level Loss decrease",             FieldKind::UInt, None, None;
    PetStoneDrop:       "L",   "ChP Pet Stone Drop boost",                FieldKind::UInt, None, None;
    StonePetImprove:    "035", "ChP Stone Pet improvement",               FieldKind::Bool, None, None;
    AdvExpBoost:        "019", "ChP Adv EXP boost",                        FieldKind::UInt, None, None;
    DungeonDrop:        "V",   "ChP Dungeon Drop boost",                  FieldKind::UInt, None, None;
    DungeonExp:         "W",   "ChP Dungeon Exp boost",                   FieldKind::UInt, None, None;
    DungeonOvertime:    "037", "ChP Dungeon Overtime",                    FieldKind::UInt, None, None;
    QuestOvertime:      "038", "ChP Quest Overtime",                      FieldKind::Bool, None, None;
    D4BossRoom:         "034", "ChP D4 boss room (stored; shown as 60 − x)", FieldKind::UInt, None, None;
    CraftingBoost:      "X",   "ChP Crafting boost",                      FieldKind::UInt, None, None;
    SpaceDimBoost:      "014", "ChP SpaceDim boost",                      FieldKind::UInt, None, None;
    SelfReplicatingAi:  "040", "ChP Self Replicating AI boost",          FieldKind::UInt, None, None;
}

/// Title each element from one of its fields (id → name).
const fn elem(key: &'static str, resolve: Resolve) -> Option<ElementName> {
    Some(ElementName { key, resolve })
}

/// Every block, consumed by the save editor to build tree labels.
pub const BLOCKS: &[BlockSchema] = &[
    BlockSchema { base: &["X", "b"], name: "Pet", plural: "Pets", is_list: true, element_name: elem("a", Resolve::Literal), fields: PET_FIELDS },
    BlockSchema { base: &["x"], name: "Statistics", plural: "Statistics", is_list: false, element_name: None, fields: STATISTICS_FIELDS },
    BlockSchema { base: &["x", "242"], name: "Challenge", plural: "Challenge Completions", is_list: true, element_name: elem("a", Resolve::Challenge), fields: CHALLENGE_COMPLETION_FIELDS },
    BlockSchema { base: &["013"], name: "Overflow Point Upgrades", plural: "Overflow Point Upgrades", is_list: false, element_name: None, fields: OFP_UPGRADE_FIELDS },
    BlockSchema { base: &["029", "a"], name: "Ultimate Overflow Upgrade", plural: "Ultimate Overflow Upgrades", is_list: true, element_name: elem("a", Resolve::UltimateOverflowUpgrade), fields: UOFP_UPGRADE_FIELDS },
    BlockSchema { base: &["014", "a"], name: "RTI Bonus", plural: "RTI Bonuses", is_list: true, element_name: elem("a", Resolve::RtiBonus), fields: RTI_BONUS_FIELDS },
    // Base `["X"]` overlaps the explicit `def(&["X"], "Pets / Pet System")` in
    // the GUI registry; that explicit def is seeded first and wins on lookup for
    // the container label, while these fields land at the distinct `X.<key>`
    // paths. The keys are disjoint from every other `["X", …]` block/scalar.
    BlockSchema { base: &["X"], name: "Challenge Point Upgrades", plural: "Challenge Point Upgrades", is_list: false, element_name: None, fields: CHP_UPGRADE_FIELDS },
    BlockSchema { base: &["X", "R"], name: "Equipment", plural: "Equipment", is_list: true, element_name: elem("a", Resolve::EquipmentNode), fields: EQUIPMENT_FIELDS },
    BlockSchema { base: &["X", "Q"], name: "Material", plural: "Materials", is_list: true, element_name: elem("a", Resolve::Material), fields: MATERIAL_FIELDS },
    BlockSchema { base: &["X", "002"], name: "Gem", plural: "Gems", is_list: true, element_name: elem("a", Resolve::Element), fields: GEM_FIELDS },
    BlockSchema { base: &["X", "S"], name: "Dungeon Team", plural: "Dungeon Teams", is_list: true, element_name: elem("i", Resolve::Literal), fields: DUNGEON_TEAM_FIELDS },
    BlockSchema { base: &["X", "S", "*", "c"], name: "Pending Loot", plural: "Pending Loot", is_list: true, element_name: elem("a", Resolve::Material), fields: PENDING_LOOT_FIELDS },
    BlockSchema { base: &["X", "P"], name: "Active Dungeon Run", plural: "Active Dungeon Runs", is_list: true, element_name: elem("a", Resolve::Dungeon), fields: ACTIVE_DUNGEON_FIELDS },
    BlockSchema { base: &["X", "x"], name: "Campaign", plural: "Campaigns", is_list: true, element_name: elem("a", Resolve::CampaignType), fields: CAMPAIGN_FIELDS },
    BlockSchema { base: &["X", "Z"], name: "Challenge Team", plural: "Challenge Team", is_list: false, element_name: None, fields: CHALLENGE_TEAM_FIELDS },
    BlockSchema { base: &["T"], name: "Planet (Ultimate Beings)", plural: "Planet (Ultimate Beings)", is_list: false, element_name: None, fields: PLANET_FIELDS },
    BlockSchema { base: &["T", "f"], name: "Ultimate Being", plural: "Ultimate Beings", is_list: true, element_name: elem("c", Resolve::UltimateBeing), fields: ULTIMATE_BEING_FIELDS },
    BlockSchema { base: &["T", "k"], name: "UB Multiplier", plural: "UB Multipliers", is_list: true, element_name: elem("c", Resolve::UltimateBeing), fields: UB_MULTIPLIER_FIELDS },
    BlockSchema { base: &["024", "a"], name: "Village Building", plural: "Village Buildings", is_list: true, element_name: elem("g", Resolve::VillageBuilding), fields: VILLAGE_BUILDING_FIELDS },
    BlockSchema { base: &["024", "b"], name: "Tavern", plural: "Tavern", is_list: false, element_name: None, fields: TAVERN_FIELDS },
    BlockSchema { base: &["024", "d"], name: "Dojo", plural: "Dojo", is_list: false, element_name: None, fields: DOJO_FIELDS },
    BlockSchema { base: &["024", "e"], name: "Strategy Room", plural: "Strategy Room", is_list: false, element_name: None, fields: STRATEGY_ROOM_FIELDS },
    BlockSchema { base: &["024", "f", "a"], name: "Museum Statue", plural: "Museum Statues", is_list: true, element_name: elem("b", Resolve::Statue), fields: MUSEUM_STATUE_FIELDS },
    BlockSchema { base: &["024", "g"], name: "Material Factory", plural: "Material Factory", is_list: false, element_name: None, fields: WORKER_BUILDING_FIELDS },
    BlockSchema { base: &["024", "h"], name: "Alchemy Hut", plural: "Alchemy Hut", is_list: false, element_name: None, fields: WORKER_BUILDING_FIELDS },
    BlockSchema { base: &["024", "g", "d"], name: "Worker", plural: "Workers", is_list: true, element_name: elem("a", Resolve::PetType), fields: WORKER_SLOT_FIELDS },
    BlockSchema { base: &["024", "h", "d"], name: "Worker", plural: "Workers", is_list: true, element_name: elem("a", Resolve::PetType), fields: WORKER_SLOT_FIELDS },
    BlockSchema { base: &["025"], name: "Fishing", plural: "Fishing", is_list: false, element_name: None, fields: FISHING_FIELDS },
    BlockSchema { base: &["025", "g"], name: "Fishing Rod", plural: "Fishing Rods", is_list: true, element_name: elem("a", Resolve::Material), fields: FISHING_ROD_FIELDS },
    BlockSchema { base: &["025", "h"], name: "Bait", plural: "Bait", is_list: true, element_name: elem("a", Resolve::Material), fields: FISHING_BAIT_FIELDS },
    BlockSchema { base: &["025", "i"], name: "Fish Caught", plural: "Fish Caught", is_list: true, element_name: elem("a", Resolve::Material), fields: FISHING_FISH_FIELDS },
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The `save_block!`-generated `EquipField` and its derived `EQUIPMENT_FIELDS`
    /// stay one source: same keys, in order, with matching kind/range/resolve.
    #[test]
    fn equip_field_and_slice_agree() {
        assert_eq!(EquipField::ALL.len(), EQUIPMENT_FIELDS.len());
        for (f, fl) in EquipField::ALL.iter().zip(EQUIPMENT_FIELDS) {
            assert_eq!(f.key(), fl.key);
            assert_eq!(f.label(), fl.label);
            assert_eq!(f.kind(), fl.kind);
            assert_eq!(f.range(), fl.range);
            assert_eq!(f.resolve(), fl.resolve);
        }
    }

    /// Ranges match the in-game caps and `clamp` enforces them.
    #[test]
    fn equip_field_clamp_enforces_bounds() {
        assert_eq!(EquipField::Quality.range(), Some((0, 8)));
        // Plus caps at 30 (Candy Cane's max), not 20 — only it exceeds +20.
        assert_eq!(EquipField::Plus.range(), Some((0, 30)));
        assert_eq!(EquipField::Enchant.range(), Some((0, 20)));
        assert_eq!(EquipField::GemLevel.range(), None);
        // Clamp: over the cap pins to max, under to min, unbounded passes through.
        assert_eq!(EquipField::Quality.clamp(50), 8);
        assert_eq!(EquipField::Enchant.clamp(50), 20);
        assert_eq!(EquipField::Enchant.clamp(12), 12);
        // Candy Cane's +25/+30 survive; only absurd values pin to 30.
        assert_eq!(EquipField::Plus.clamp(25), 25);
        assert_eq!(EquipField::Plus.clamp(30), 30);
        assert_eq!(EquipField::Plus.clamp(99), 30);
        assert_eq!(EquipField::GemLevel.clamp(9_999), 9_999);
    }
}
