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

/// Persistent dungeon teams — `X.S.<index>` (`PCDCANGLENI`). These are the
/// static team settings. Per-depth difficulty: `e`=D1, `f`=D2, `g`=D3
/// (player-confirmed in-game; D4 not cleanly placeable from current saves —
/// `h` is a list, not a difficulty int). `c` = the team's loot/inventory (see
/// PENDING_LOOT_FIELDS); loot isn't actually rolled until the run completes.
pub const DUNGEON_TEAM_FIELDS: &[FieldLabel] = &[
    lblr!("b", "Dungeon Id", Resolve::Dungeon),
    lbl!("d", "Depth"),
    lbl!("e", "D1 Difficulty"),
    lbl!("f", "D2 Difficulty"),
    lbl!("g", "D3 Difficulty"),
    lbl!("i", "Dungeon Name"),
    lbl!("a", "Member Pet Type Ids"),
    lbl!("c", "Pending Loot"),
];

/// A team's pending-loot / inventory entry — `X.S.<i>.c.<index>` (`GCJMGGFGKBN`,
/// same shape as the material inventory). `a`=item id, `b`=count.
pub const PENDING_LOOT_FIELDS: &[FieldLabel] =
    &[lblr!("a", "Item Id", Resolve::Material), lbl!("b", "Count")];

/// The Challenge team — `X.Z` (a single `PCDCANGLENI`, same class as a dungeon
/// team; C# `NMGIGAGPLCL`). `a` = members (`&`-joined pet ids). Its own inventory
/// lives at `c` (same shape as a team's pending loot), but it's empty in the
/// reference save so it isn't labeled here. (Challenges have no
/// difficulty/depth/timer, so those team fields are unused.)
pub const CHALLENGE_TEAM_FIELDS: &[FieldLabel] = &[lbl!("a", "Member Pet Type Ids")];

/// Active dungeon runs — `X.P.<index>` (`MKDNAHGDLPI`). `a`=dungeon id,
/// `b`=elapsed ms (counts up to `c`), `c`=target duration ms (43,200,000 = 12 h),
/// `d`=depth, `f`=team index (ties the run to its `X.S` team). `e`/`j` are RNG
/// seeds. To force near-completion, set `b` just under `c`.
pub const ACTIVE_DUNGEON_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Dungeon Id", Resolve::Dungeon),
    lbl!("b", "Elapsed (ms)"),
    lbl!("c", "Target Duration (ms)"),
    lbl!("d", "Depth"),
    lbl!("e", "RNG seed (e)"),
    lbl!("f", "Team Index"),
    lbl!("j", "RNG seed (j)"),
];

/// Museum statues — `024.f.a.<index>` (`MCEIHMMCDNH`). `a` = level (20 when
/// maxed), `b` = statue id (event commemoratives; you can own two of each).
pub const MUSEUM_STATUE_FIELDS: &[FieldLabel] =
    &[lbl!("a", "Level"), lblr!("b", "Statue", Resolve::Statue)];

/// Planet system — `root.T` (`AIDFNOPNJGK`, marker "Planet"). `d` = **planet
/// level** (drives the planet name tiers; level 1-5 from feeding planet/earthlike/
/// sun/solar-system/universe, then +1 per Ultimate Universe Challenge; the
/// effective level for power adds the UUC count on top). `h` = unspent **Baal
/// Power** (player-confirmed; spent on Light Clones that fight the UBs).
/// The Planet Multiplier is computed (base 100% + Powersurge `T.k` + UB-kill
/// `T.f` contributions), not a stored scalar.
pub const PLANET_FIELDS: &[FieldLabel] = &[
    lbl!("d", "Planet Level"),
    lbl!("h", "Unspent Baal Power"),
];

/// Planet — per-UB state — `T.k.<index>` (`FPBMNCNKPHN`), one per UB. `c` = UB id,
/// `b` = **kill/defeat count** (incremented on each defeat, `AIDFNOPNJGK:256-257`)
/// — this is what drives the tooltip's "Multi from Ultimate Beings" (each UB adds
/// a fixed % per defeat: Planet Eater 1% / Godly Tribunal 12% / Living Sun 21% /
/// God Above All 32% / ITRTG 45%). `a` = a per-UB state value reset to 100.0/0.0
/// in OfflineCalc (~100 = full; exact role unconfirmed — *not* the displayed
/// multiplier). NOT the (single) Powersurge — that's a separate `T` scalar (TBD).
pub const UB_MULTIPLIER_FIELDS: &[FieldLabel] = &[
    lblr!("c", "UB", Resolve::UltimateBeing),
    lbl!("b", "Kill Count"),
    lbl!("a", "State (~100)"),
];

/// Planet — Ultimate Beings — `T.f.<index>` (`CEFAAPALBMD`). The 5 UBs that
/// attack your planet on staggered spawn timers. `c` = UB id (1 Planet Eater …
/// 5 ITRTG), `b` = kill count, `d` = spawn countdown ms (counts DOWN; spawns at
/// ≤0 — set 0 to force a spawn), `e` = alive flag, `f` = god power gained.
pub const ULTIMATE_BEING_FIELDS: &[FieldLabel] = &[
    lblr!("c", "UB", Resolve::UltimateBeing),
    lbl!("b", "Kill Count"),
    lbl!("d", "Spawn Countdown (ms)"),
    lbl!("e", "Alive"),
    lbl!("f", "God Power Gained"),
];

/// Village building-state list — `024.a.<index>` (`AFELNLGMCAB`, marker
/// "VillageBuilding"). One entry per building feature, keyed by `g` = building
/// type (`IMBOLMEHKCG`). `c` = level, `f` = assigned pet (special-pet enum); other
/// fields are unlock/flag state (mostly default in the ref save).
pub const VILLAGE_BUILDING_FIELDS: &[FieldLabel] =
    &[lblr!("g", "Building Type", Resolve::VillageBuilding)];

/// Worker buildings — Material Factory `024.g` (`CHDGDEINMHO`) and Alchemy Hut
/// `024.h` (`GABIFCBBMPH`), both extending `ANECMNGBLNI`. `a` = level, `e` =
/// manager slot (pet type id; 999 = empty), `d` = worker pet-slot list.
pub const WORKER_BUILDING_FIELDS: &[FieldLabel] = &[
    lbl!("a", "Level"),
    lblr!("e", "Manager (pet type id)", Resolve::PetType),
];

/// A worker building's pet slot — `024.{g,h}.d.<index>` (`FGKIILDKMEA`). `a` =
/// pet type id (999 = empty), `d` = work progress/exp. `b`/`c` are the
/// in-progress craft (nested sub-structs, unconfirmed).
pub const WORKER_SLOT_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Pet Type Id", Resolve::PetType),
    lbl!("d", "Work Progress"),
];

/// Pet Village Tavern — `024.b` (`IOBPPFGEBCD`). Runs pet quests. Player-mapped:
/// `b` = level, `c` = upgrade-elapsed timer, `d` = **Quest Points**, `i` = quests
/// per day, `j` = max concurrent quests, `u` = Tavern Keeper slot (999 = empty),
/// `x` = favorite quests (`&`-list). `a`/`t` are quest lists (active / pool);
/// other scalars (`e`/`g`/`l`/`m`/`p`/`q`/`r`/`v`/`w`…) unconfirmed.
pub const TAVERN_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Level"),
    // `c` = upgrade-elapsed timer (same as other buildings) but it's empty when
    // not upgrading (as in the ref save), so it isn't labeled here.
    lbl!("d", "Quest Points"),
    lbl!("i", "Quests Per Day"),
    lbl!("j", "Max Concurrent Quests"),
    lbl!("u", "Tavern Keeper (slot)"),
    lbl!("x", "Favorite Quests (&-list)"),
];

/// Pet Village Dojo — `024.d` (`JKDCFKCLCKH`). `b` = level (player-confirmed:
/// 8 in the ref save), `c` = **elapsed upgrade time** (`LDMJEPGEOME`, the same
/// universal elapsed-timer field as a dungeon run's `b`): it accumulates until
/// `c >= target`, then the upgrade completes and resets to 0 — so set `c` to a
/// large value to force-complete an in-progress upgrade. The four `999` fields
/// (`s`/`t`/`v`/`w`) are its 4 pet slots (2 Dojo Master + 2 pupil); the many
/// other fields are per-stat training buffs (unconfirmed).
pub const DOJO_FIELDS: &[FieldLabel] = &[lbl!("b", "Level"), lbl!("c", "Upgrade Elapsed (ms)")];

/// Pet Village Strategy Room — `024.e` (`CJACGIIPNIG`). The three multipliers
/// were player-confirmed by tweaking them in-game.
pub const STRATEGY_ROOM_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Level"),
    lbl!("c", "Upgrade Elapsed (ms)"), // accumulates to target then resets; set large to finish

    lbl!("e", "Physical Multi %"),
    lbl!("f", "Mystic Multi %"),
    lbl!("g", "Battle Multi %"),
    lbl!("h", "Pet Slots (&-list, 8)"),
];

/// Fishing block — `root.025` (`KACINBICCNH`). `a` = Fish Power (labeled
/// separately in Resources), `b` = current exp (resets to 0 on level-up), `c` =
/// level, `d`/`e` = selected bait/rod (material ids), `f` = current pond. Lists:
/// `g` = rods, `h` = bait, `i` = fish caught (see the *_FIELDS below).
pub const FISHING_FIELDS: &[FieldLabel] = &[
    lbl!("b", "Fishing Exp"),
    lbl!("c", "Fishing Level"),
    lblr!("d", "Selected Bait", Resolve::Material),
    lblr!("e", "Selected Rod", Resolve::Material),
    lblr!("f", "Current Pond", Resolve::Pond),
];

/// Owned fishing rods — `025.g.<index>` (`ANCPDAFDBPP`). `a` = rod material id
/// (500-504), `b` = owned (0/1).
pub const FISHING_ROD_FIELDS: &[FieldLabel] =
    &[lblr!("a", "Rod", Resolve::Material), lbl!("b", "Owned")];

/// Bait stacks — `025.h.<index>` (`ANCPDAFDBPP`). `a` = bait material id
/// (520-524), `b` = count.
pub const FISHING_BAIT_FIELDS: &[FieldLabel] =
    &[lblr!("a", "Bait", Resolve::Material), lbl!("b", "Count")];

/// Fish-caught records — `025.i.<index>` (`PNPLCJJOPIO`). `a` = fish material id
/// (525+), `c` = lifetime caught count.
pub const FISHING_FISH_FIELDS: &[FieldLabel] =
    &[lblr!("a", "Fish", Resolve::Material), lbl!("c", "Caught")];

/// Campaign slots — `X.x.<index>` (`FMOLELEHAFD`). `a` = campaign type
/// (`AGGDKICFOAI`), `c` = elapsed ms (counts up to `e`), `e` = target duration ms
/// (43,200,000 = 12 h) — same elapsed/target shape as a dungeon run, so setting
/// `c` = `e` completes the campaign. `d` = `&`-joined pet type ids.
pub const CAMPAIGN_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Campaign Type", Resolve::CampaignType),
    lbl!("c", "Elapsed (ms)"),
    lbl!("d", "Pet Type Ids"),
    lbl!("e", "Target Duration (ms)"),
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
];

/// Per-challenge completion record (`KPLPGPEOFNB`), one per element of the
/// `root.x.242` list. `a` is the challenge id, `b` the lifetime completion count
/// (the number shown in the Challenges menu), `c` the difficulty, `d` an ms
/// epoch (last completion time — inferred from per-challenge recency vs. count),
/// `e` a UI sort flag. Validated against an in-game capture 2026-06-20.
pub const CHALLENGE_COMPLETION_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Challenge", Resolve::Challenge),
    lbl!("b", "Completions"),
    lblr!("c", "Difficulty", Resolve::ChallengeDifficulty),
    lbl!("d", "Last Completed (ms)"),
    lbl!("e", "Flag (e)"),
];

/// Overflow-Point upgrade levels (`HNFHEBJIPEL`, `root.013`). Each stored field
/// is the bought upgrade amount; the in-game effect getter adds a base on top.
/// Labels are the literal "OfP …" names from the Challenge-Points debug tooltip
/// (`LLMCMCKAABP.cs:4063`), mapped to save keys via each getter's underlying
/// field (`HNFHEBJIPEL.cs:39–63`). Field `h` has no getter/label there (vestigial
/// here — the external matches are a name collision in another class).
pub const OFP_UPGRADE_FIELDS: &[FieldLabel] = &[
    lbl!("a", "OfP Black Hole"),
    lbl!("b", "OfP Black Hole Upgrade"),
    lbl!("c", "OfP Gem Cap"),
    lbl!("d", "OfP Gem Gain"),
    lbl!("e", "OfP V2 Auto Kill"),
    lbl!("f", "OfP Hp Regen"),
    lbl!("g", "OfP Crystal Power"),
    lbl!("h", "OfP Upgrade (h, unlabeled)"),
    lbl!("i", "OfP Creating Stat"),
    lbl!("j", "OfP Powersurge"),
    lbl!("k", "OfP Creation Count"),
    lbl!("l", "OfP Might Speed"),
    lbl!("m", "OfP Stats Multi"),
    lbl!("n", "OfP Space Dim"),
];

/// RTI (Road to Infinity) bonus entry (`HEIPGLPOGEJ`, marker `RtiElement`; one
/// per element of the `root.014.a` list — 10 entries, one per `BDAFIPJBPFN` stat
/// type). `a` = stat type, `e` = elapsed timer (`LDMJEPGEOME`, the universal
/// elapsed-timer field). `b` feeds the in-game "Increases your <stat> by …"
/// tooltip (the stored bonus amount); `c`/`d`/`g`/`h` are per-type values whose
/// exact roles aren't separately anchored — labeled neutrally, not guessed.
pub const RTI_BONUS_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Bonus Type", Resolve::RtiBonus),
    lbl!("b", "Bonus Amount"),
    lbl!("c", "Value (c)"),
    lbl!("d", "Value (d)"),
    lbl!("e", "Elapsed (ms)"),
    lbl!("g", "Value (g)"),
    lbl!("h", "Value (h)"),
];

/// Ultimate-Overflow upgrade entry (`FDJCCPFCJAO`, one per element of the
/// `root.029.a` list; parent `CDNMNLIAPKA` marker `UltimateOverflowBoosts`).
/// `a` = upgrade type (`IDFOIHJPCHP`), `b` = bought level. These are the boosts
/// purchased with Ultimate Overflow Points (the fixture holds all 6 types at 0).
pub const UOFP_UPGRADE_FIELDS: &[FieldLabel] = &[
    lblr!("a", "Upgrade Type", Resolve::UltimateOverflowUpgrade),
    lbl!("b", "Level"),
];

/// Challenge-Point upgrade levels — scalar fields directly on `root.X`
/// (`MLILKGIALMB`, the `FIHAENJIDAO` accessor). Names are the literal "Chp …"
/// labels from the Challenge-Points debug tooltip (`LLMCMCKAABP.cs:4063`);
/// each maps straight to a save key in `MLILKGIALMB.EBOFJJHOOLP` (the real
/// deserializer at line 10036 — the `n19`/`n41`/`-36` copies elsewhere are
/// decoys). Total ChP spent = Σ(level × cost) per `MLILKGIALMB.cs:894`; the
/// stored value here is the bought level. `035`/`038` are bools.
pub const CHP_UPGRADE_FIELDS: &[FieldLabel] = &[
    lbl!("E", "ChP Planet Level"),
    lbl!("I", "ChP Divinity boost"),
    lbl!("D", "ChP Damage Reduction UBs"),
    lbl!("041", "ChP Faster UB spawn"),
    lbl!("G", "ChP Crystal Upgrade boost"),
    lbl!("H", "ChP Damage Boost V2s"),
    lbl!("J", "ChP CP boost"),
    lbl!("039", "ChP Crystal Sacrifice boost"),
    lbl!("029", "ChP BS boost"),
    lbl!("030", "ChP CS boost"),
    lbl!("K", "ChP TBS Level Loss decrease"),
    lbl!("L", "ChP Pet Stone Drop boost"),
    lbl!("035", "ChP Stone Pet improvement"),
    lbl!("019", "ChP Adv EXP boost"),
    lbl!("V", "ChP Dungeon Drop boost"),
    lbl!("W", "ChP Dungeon Exp boost"),
    lbl!("037", "ChP Dungeon Overtime"),
    lbl!("038", "ChP Quest Overtime"),
    lbl!("034", "ChP D4 boss room (stored; shown as 60 − x)"),
    lbl!("X", "ChP Crafting boost"),
    lbl!("014", "ChP SpaceDim boost"),
    lbl!("040", "ChP Self Replicating AI boost"),
];

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
