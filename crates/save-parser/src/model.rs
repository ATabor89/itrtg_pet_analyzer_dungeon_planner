//! Typed extraction of the identified parts of a save tree.
//!
//! Field meanings come from cross-referencing a save against same-session
//! in-game exports — see `reference/save_file_deserialization/FINDINGS.md`.
//! Unknown fields stay accessible through the raw [`Node`]s kept on
//! [`SaveFile`] and [`SavePet`], which is the main tool for closing the
//! remaining gaps.

use anyhow::Context;
use itrtg_models::{Class, Element};

use crate::tree::Node;

/// Sentinel in the pet `F` field meaning "no partner".
const PARTNER_NONE: u32 = 999;

/// A fully parsed save file: the typed parts we understand, plus the raw
/// tree for everything else.
#[derive(Debug, Clone)]
pub struct SaveFile {
    /// Unix timestamp (seconds) the save was written (root `c`).
    pub saved_at_unix: Option<i64>,
    /// In-game god (deity) name — the name the player gave their god (root
    /// `W`; reads `RedactedGod` in the committed, redacted fixtures).
    pub god_name: Option<String>,
    /// Linked platform account login name (root `s`) — the Steam/Kongregate
    /// account the save is tied to, *not* the god name. (Earlier mislabeled as
    /// the god name; corrected after the player confirmed the in-game god name
    /// is the `W` value and `s` is their account login.)
    pub account_name: Option<String>,
    /// Pet stones (root `X.y`).
    pub pet_stones: Option<u64>,
    /// Cumulative pet stones **spent** (root `X.z`). Confirmed 2026-06-16 by a
    /// fresh-save diff: buying 2 Dungeon Loot + 1 Dungeon Exp moved `X.y` down
    /// by 750,000 and `X.z` up by exactly 750,000 (= 2·275k + 200k, the wiki
    /// costs).
    pub pet_stones_spent: Option<u64>,
    /// "Crafting Queue Slot" pet-stone upgrades bought (root `X.032`) — extra
    /// crafting queue slots for blacksmith pets. Confirmed 2026-06-16 by a
    /// fresh-save buy (0 → 1, X.y −500,000 = the wiki cost).
    pub crafting_queue_slots: u32,
    /// Pet Tokens (root `p.I`) — the currency for unlocking/evolving pets.
    /// Confirmed 2026-06-16 by a fresh-save diff (5 → 6 across a +1-token save
    /// pair). NB: `p.I` was *not* the TBS-pixels twin of `p.D` — they were both
    /// 3 in the main save by coincidence.
    pub pet_tokens: Option<u64>,
    /// Class Change Tokens (root `p.023`) — re-class an evolved pet for free.
    /// Confirmed 2026-06-16 by the same diff pair (8 → 10).
    pub class_change_tokens: Option<u64>,
    /// Free experience (root `X.Y`, capital) — the pool of pet exp you can
    /// freely apply. Confirmed 2026-06-16: a save-edit of `X.Y` to 1e9 showed
    /// the matching free-exp value in-game. (NB: `X.Y` ≠ `X.y`, which is pet
    /// stones.)
    pub free_experience: Option<u64>,
    /// Unopened Lucky Draws (root `p.K`). Confirmed 2026-06-16 by a 3-save diff
    /// (6 → 2 → 0 as the player used them). The *opened* lifetime count is the
    /// separate tracker `x.071`.
    pub lucky_draws: Option<u64>,
    /// Godly Liquid (regular, root `p.b`) — the ×2-creating-speed consumable.
    /// Confirmed 2026-06-16 by the same diff (0 → 1 when a draw yielded one),
    /// re-confirmed by the Steam consumables diff (151 → 141).
    pub godly_liquid: Option<u64>,
    /// The remaining boost consumables, all in `root.p`, confirmed 2026-06-16 by
    /// a Steam save diff (`Steam/Consumables/`) with distinct deltas. NB: this
    /// corrected two earlier guesses — `p.e` was tentatively a TBS field and
    /// `p.d` looked creation-count-ish (both just happened to match by value).
    pub godly_liquid_v2: Option<u64>,
    pub chakra_pill: Option<u64>,
    pub chakra_pill_v2: Option<u64>,
    /// Ultimate Shadow Summon (root `p.e`), confirmed by the same diff (19→18).
    pub ultimate_shadow_summon: Option<u64>,
    /// Pet food counts (root `X.c`/`X.d`/`X.e`) and chocolate (root `X.v`).
    /// These are dedicated fields, not material-inventory entries. An absent
    /// field reads as 0.
    pub puny_food: u64,
    pub strong_food: u64,
    pub mighty_food: u64,
    pub chocolate: u64,
    /// Gem inventory (root `X.002`).
    pub gems: Vec<GemStack>,
    /// All pets (root `X.b`), in save order.
    pub pets: Vec<SavePet>,
    /// Owned pet equipment instances (root `X.R`).
    pub equipment: Vec<EquipmentItem>,
    /// Material/item stacks (root `X.Q`).
    pub materials: Vec<MaterialStack>,
    /// The three persistent dungeon teams (root `X.S`).
    pub dungeon_teams: Vec<DungeonTeam>,
    /// Campaign slots (root `X.x`).
    pub campaigns: Vec<CampaignSlot>,
    /// Available god power (root `p.j`).
    pub gp_available: Option<u64>,
    /// Total god power spent (root `p.v`).
    pub gp_spent: Option<u64>,
    /// Total might (root `p.F`).
    pub total_might: Option<u64>,
    /// Crystal power (root `p.002` — the numeric keys are siblings of the
    /// letter keys inside `p`).
    pub crystal_power: Option<u64>,
    /// Rebirth count (root `x.k`).
    pub rebirths: Option<u64>,
    /// Light clones (root `O.030`). (Bought with Baal Power, not GP — but
    /// the count is mirrored near the god-power data.)
    pub light_clones: Option<u64>,
    /// Statistics multi (root `p.C`) — the rebirth-multiplier input tracked
    /// on the statistics page. 2^50 on the reference account (to the save's
    /// 15-significant-digit text precision), cross-validating the 50 paid
    /// doublings (2,500 GP ÷ 50 per double) recorded at `p.017`/`p.019`.
    pub statistics_multi: Option<f64>,
    /// Creation count from god power (root `p.q`). The export's "Creation
    /// Count" adds the base 1 (and equipped-crystal bonuses are separate).
    /// Milestones key off this from-god-power value.
    pub creation_count_gp: Option<u64>,
    /// Earth Eater: Earthlike planets eaten *this rebirth* (root `018`),
    /// at 1/s while eating. Moves in lockstep with the lifetime total at
    /// `x.185` ([`trackers::EARTH_EATER_PLANETS_TOTAL`]) — identical
    /// deltas across the reference saves (+42,574).
    pub earth_eater_planets_rebirth: Option<u64>,
    /// Anni Cake's current stat bonus in percent (root `033`), stored
    /// directly as a fractional float (948.969… displays as 949%). Grows by
    /// 10% (+0.1%×CL when evolved) per hour in food campaigns, fractional
    /// from early-cancelled campaigns; resets on rebirth, capped at 3653%.
    pub anni_cake_bonus_percent: Option<f64>,
    /// Adventure-mode researches (root `032.H.a`), in id order.
    pub researches: Vec<Research>,
    /// Creations (root `i`), in id order.
    pub creations: Vec<Creation>,
    /// Monuments (root `D`), in id order.
    pub monuments: Vec<Monument>,
    /// Mights (root `V`), in id order.
    pub mights: Vec<Might>,
    /// SpaceDim / Light-Dimension elements (root `009.b`), in display order.
    pub spacedim: Vec<SpaceDimElement>,
    /// Physical conditioning exercises (root `h`), in display order — these
    /// raise the Physical stat and have no usage count of their own.
    pub physical_trainings: Vec<TrainingEntry>,
    /// Skills (root `j`), in display order — these raise the Mystic stat and
    /// carry the "Special"-menu usage count that drives both their own and the
    /// index-matched Physical's training cap.
    pub skills: Vec<TrainingEntry>,
    /// Monsters fought for Battle/Divinity (root `k`), in display order.
    pub monsters: Vec<Monster>,
    /// Total Divinity (root `a`) — the running divinity balance, a very large
    /// float. Player-confirmed 2026-06-18 by editing it (E+19 → E+29) and seeing
    /// the in-game total change. (The Divinity Generator's `K.g` is only the
    /// amount currently held in the generator, not this total.)
    pub total_divinity: Option<f64>,
    /// Divinity Generator (root `K`): capacity in use, worker clones, stone
    /// storage, and the three upgrade tracks. `None` if the block is absent.
    pub divinity_generator: Option<DivinityGenerator>,
    /// Unspent Baal Power (root `T.h`) — the Baal Slayer currency.
    pub baal_power: Option<u64>,
    /// Current god number (root `P.c`): the P. Baal the player is now
    /// fighting, which is one past the highest one defeated. See
    /// [`SaveFile::pbaal_defeated`].
    pub current_god_number: Option<u32>,
    /// GP-purchased creating-speed % (root `p.h`).
    pub gp_creating_speed_pct: Option<u64>,
    /// GP-purchased building-speed % (root `p.i`).
    pub gp_building_speed_pct: Option<u64>,
    /// Unused-GP god-stat allocation split (root `p.r/s/t/u`). `None` if the
    /// god-power block is absent.
    pub gp_allocation: Option<GpAllocation>,
    /// Permanent account upgrades stored in the `root.p` block (god-power +
    /// pet-stone purchases live together there). `None` if `p` is absent.
    pub permanent_upgrades: Option<PermanentUpgrades>,
    /// The Baal Slayer (TBS) component levels (root `S`). `None` if absent.
    pub tbs_levels: Option<TbsLevels>,
    /// The full raw tree, for exploring not-yet-identified fields.
    pub root: Node,
}

/// One pet from the save (`X.b[i]`).
///
/// The save stores the pet's *display* name (e.g. "Rudolph", "Chicken",
/// "Pandora's box"), which differs from the export names for ~24 pets and
/// strips no spaces. Combat stats (HP/Attack/...) are not stored in the
/// save at all — they are derived at runtime by the game.
#[derive(Debug, Clone)]
pub struct SavePet {
    /// Display name (`a`).
    pub name: String,
    /// Internal pet type id (`k`) — the id used by team/campaign lists.
    pub type_id: u32,
    /// Unlocked flag (`l`).
    pub unlocked: bool,
    /// Growth (`E`). Fractional; in-game exports show it rounded.
    pub growth: f64,
    /// Normal level (`g`) — resets at rebirth; drives normal stats.
    pub normal_level: u32,
    /// Current normal Health (`j`), recomputed live by the game. Health is
    /// exactly 10 × Physical ("each physical increases 10 Hp"), so
    /// [`SavePet::physical_stat`] derives from this. Mystic and Battle are
    /// not stored; in-game they differ from Physical only by the Strategy
    /// Room multiplier ratio. See
    /// `reference/save_file_deserialization/normal_stats_investigation.md`.
    pub normal_health: f64,
    /// Training-clone stats — a *snapshot* taken when training was last
    /// configured (bit-identical across saves a day apart, while
    /// `normal_health` moved): the clones this pet fights have these stats.
    /// `o` = clone Physical (Physical‰ × pet Battle / 1000), `p` = clone
    /// Mystic (Mystic‰ setting), `q` = clone Battle (Battle‰ setting),
    /// `r` = clone HP (= 10 × clone Physical, per the Health rule).
    pub clone_physical: f64,
    pub clone_mystic: f64,
    pub clone_battle: f64,
    pub clone_hp: f64,
    /// Dungeon team slot 1–6 (`v`), `None` when not on a team.
    pub team_slot: Option<u8>,
    /// Element (`w.a`).
    pub element: Option<Element>,
    /// Dungeon level (`w.b`).
    pub dungeon_level: u32,
    /// Dungeon exp (`w.c`): **current exp toward the next dungeon level**,
    /// resetting on level-up — matches the in-game "current / needed"
    /// display exactly. The requirement side is
    /// [`crate::formulas::dungeon_exp_to_next`].
    pub dungeon_exp: f64,
    /// Class (`w.d.a`), `None` for classless pets (id 0).
    pub class: Option<Class>,
    /// Class level (`w.d.b`).
    pub class_level: u32,
    /// Class exp (`w.d.c`): current exp toward the next class level, same
    /// semantics as `dungeon_exp` (verified across saves: Salamander hit
    /// CL 25 between the two reference saves and the counter reset).
    /// Requirement: [`crate::formulas::class_exp_to_next`].
    pub class_exp: f64,
    /// Equipment instance ids (`w.e`/`w.f`/`w.g`), `None` when empty (0).
    pub weapon_id: Option<u32>,
    pub armor_id: Option<u32>,
    pub accessory_id: Option<u32>,
    /// Partner pet type id (`F`), `None` when 999. Pairs are mutual
    /// (Cat↔Dog, Vampire↔Succubus, ...). Note id 0 (Mouse) is valid.
    pub partner_type_id: Option<u32>,
    /// Days partnered (`G`) — incremented by exactly 1 per day for every
    /// partnered pet (verified across the two reference saves).
    pub partner_days: u64,
    /// Current exp toward the next normal level (`h`). Matches the in-game
    /// "Current exp" display; only moves while the pet trains.
    pub current_exp: f64,
    /// Village working experience (`H`), in **milliseconds** of total time
    /// worked. Matches the in-game working-time display to the second
    /// (Lamb 9,375,772,300 ms ↔ ~108d 12h).
    pub working_experience_ms: u64,
    /// The pet's raw node, for the still-unidentified fields
    /// (`d,e,f,n,s,t,u,x,y,z,A–D`).
    pub raw: Node,
}

/// One owned equipment instance (`X.R[i]`).
#[derive(Debug, Clone)]
pub struct EquipmentItem {
    /// Item type id (`a`) — resolve with [`EquipmentItem::type_name`].
    pub type_id: u32,
    /// Upgrade ("+") level (`b`).
    pub plus: u32,
    /// Quality (`c`): observed 8=SSS, 6=S, 4=B (likely 4=B..8=SSS).
    pub quality: u32,
    /// Instance id (`d`, mirrored in `h`) — referenced by pets' equip slots.
    pub instance_id: u32,
    /// `e`: 20 on items whose export shows a "(20)" suffix, else 0.
    pub plus_cap: u32,
    /// Gem level (`f`), 0 = no gem.
    pub gem_level: u32,
    /// Gem element (`g`) when a gem is socketed.
    pub gem_element: Option<Element>,
}

/// A material/item stack (`X.Q[i]`), e.g. id 159 = Strategy Books.
#[derive(Debug, Clone, Copy)]
pub struct MaterialStack {
    pub item_id: u32,
    pub count: u64,
}

impl MaterialStack {
    /// Display name from the known id table, if this id has been identified.
    pub fn name(&self) -> Option<&'static str> {
        crate::items::material_name(self.item_id)
    }
}

/// One gem stack (`X.002[i]`): element id + gem level + count.
/// Uses the same element ids as pets (0=Neutral, 1=Fire, 2=Water, 3=Earth,
/// 4=Wind).
#[derive(Debug, Clone, Copy)]
pub struct GemStack {
    /// Raw element id (`a`), kept so an unrecognized id stays diagnosable.
    pub element_id: u32,
    /// Decoded element, `None` if the id is unknown.
    pub element: Option<Element>,
    pub level: u32,
    pub count: u64,
}

/// One persistent dungeon team (`X.S[i]`).
#[derive(Debug, Clone)]
pub struct DungeonTeam {
    /// Dungeon id (`b`).
    pub dungeon_id: u32,
    /// Depth (`d`).
    pub depth: u32,
    /// Dungeon display name (`i`), possibly truncated ("Water Temp").
    pub dungeon_name: String,
    /// Member pet type ids (`a`); slot order lives on each pet (`team_slot`).
    pub pet_type_ids: Vec<u32>,
    /// Pending loot as `(item id, count)` pairs (`c`).
    pub loot: Vec<(u32, u64)>,
}

/// One creation (root `i[n]`).
#[derive(Debug, Clone, Copy)]
pub struct Creation {
    /// Creation id (`a`) — resolve with [`crate::items::creation_name`].
    pub id: u32,
    /// "Next at" clone count (`i`), as shown in the Next Ats export.
    pub next_at: u64,
    /// Current amount owned (`d`). The Shadow Clone entry equals the
    /// current clone count (capped at max clones).
    pub current_amount: f64,
    /// Total created (`g`) — matches the in-game mouseover. Counts actual
    /// creation only: divinity-bought copies do **not** increment it
    /// (which is why it can sit frozen while the creation is consumed and
    /// re-bought continuously).
    pub created: f64,
    /// Clone cost to create one (`e`).
    pub clone_cost: u64,
}

/// One monument (root `D[n]`).
#[derive(Debug, Clone, Copy)]
pub struct Monument {
    /// Monument id (`a`) — resolve with [`crate::items::monument_name`].
    pub id: u32,
    /// Current level (`b`). Equals `next_at` once the monument has reached
    /// it (clones then spill to the next thing in the list); diverges while
    /// building (Black Hole: level 110 vs next-at 140 in save 2).
    pub level: u64,
    /// "Next at" level (`g`). (The paired upgrade's next-at/level from the
    /// export is not stored in this entry — location unknown.)
    pub next_at: u64,
    /// Clone-spread ratio (`h`) used by the spread-clones button.
    pub spread: u32,
    /// Currently being built (`f`).
    pub building: bool,
    /// Clones allocated to the build (`c`).
    pub clones_allocated: u64,
    /// Build progress (`d`).
    pub progress: f64,
}

/// One might (root `V[n]`).
#[derive(Debug, Clone, Copy)]
pub struct Might {
    /// Might id (`a`) — resolve with [`crate::items::might_name`].
    pub id: u32,
    /// Current level (`b`) — resets each rebirth. The sum across all
    /// mights is the White Tiger unlock progress (25,000 needed; this
    /// account: 3,200 ✓ matching the in-game unlock screen).
    pub level: u64,
    /// "Next at" level (`m`).
    pub next_at: u64,
    /// Clone-spread ratio (`n`).
    pub spread: u32,
    /// True for the special "Unleash Might" abilities (ids 8–13) (`e`).
    pub special: bool,
    /// Base unleash duration in seconds (`g`); each might level adds +1 s
    /// (level 64 Focused Breathing: 30 + 64 = 94 s ✓ in-game).
    pub base_duration_s: u32,
    /// Unleash effect percentages (`i`/`j`/`k`): HP recovery / Attack /
    /// Mystic. Zero for normal mights.
    pub hp_recovery_pct: u32,
    pub attack_pct: u32,
    pub mystic_pct: u32,
}

/// One SpaceDim / Light-Dimension element (root `009.b[i]`).
///
/// SpaceDim levels reset each rebirth. The list is in the in-game display
/// order, and `id` (1-based) *is* that order — resolve a name with
/// [`SpaceDimElement::name`] / [`crate::items::spacedim_name`]. Decoded
/// 2026-06-13: the levels/next-at/spread match the player's notes exactly
/// (Fusion Torch level 18→70, Dyson 22→23, Quantum Genesis 2→6).
#[derive(Debug, Clone, Copy)]
pub struct SpaceDimElement {
    /// Element id = display order, 1 (Controlled Entropy) … 20 (Self
    /// Replicating AI) (`a`).
    pub id: u32,
    /// Light clones allocated to this element (`b`); only the actively-fed
    /// element is nonzero.
    pub clones: u64,
    /// Current level (`c`).
    pub level: u64,
    /// "Next at" clone count (`d`).
    pub next_at: u64,
    /// Accumulated clones toward the next level (`e`).
    pub progress: f64,
    /// Clone-spread priority (`f`), the 20…1 value shown in-game.
    pub spread: u32,
}

impl SpaceDimElement {
    /// Display name from the id table, if recognized.
    pub fn name(&self) -> Option<&'static str> {
        crate::items::spacedim_name(self.id)
    }
}

/// One training entry — a Physical conditioning exercise (root `h`) or a Skill
/// (root `j`). Both blocks share the same struct shape; resolve the name with
/// [`crate::items::physical_training_name`] / [`crate::items::skill_name`] (the
/// id `a` is the 0-based list position = the screen order).
///
/// `level` (`b`) and `clones` (`c`) were player-confirmed 2026-06-18 by taking
/// clones off some Physicals while leaving the synced Skills alone and watching
/// both fields diverge as expected. The byte-identical `b` between Physical[i]
/// and Skill[i] in a fully-reduced Steam save is the in-game "Sync" toggle
/// keeping clone counts (and thus levels) equal, not a shared value.
///
/// Only **Skills** carry the `e` sub-struct (the Physical side has none): `e.a`
/// is the skill id again and `e.b` is the [`usage_count`](Self::usage_count).
/// The game derives the training **cap** (clones needed to max training speed)
/// from that usage count, and applies it to *both* the Skill and the
/// index-matched Physical — which is why the data lives only on the Skills side.
/// The `d` field (0 in every observed entry) and `e.c` (a small stable int) stay
/// in [`Self::raw`] pending identification.
#[derive(Debug, Clone)]
pub struct TrainingEntry {
    /// Skill / training id = list position (`a`).
    pub id: u32,
    /// Current level (`b`).
    pub level: u64,
    /// Clones allocated to this entry (`c`). All `1` on a fully-reduced Steam
    /// save (training caps drop to a single clone over time); a Kongregate save
    /// shows the real per-entry spread.
    pub clones: u32,
    /// "Special"-menu usage count (`e.b`) — how many times the Skill has been
    /// used (auto-trains ~1/min; manual fights add more). Drives the training
    /// cap for this Skill *and* the index-matched Physical. `None` on Physical
    /// entries, which have no `e` sub-struct. Player-confirmed 2026-06-18 by
    /// copying one save's `e.b` onto a fresh save and watching both the Skill's
    /// and the matching Physical's caps drop to 1 clone, matching the in-game
    /// "Usage Count" tooltip.
    pub usage_count: Option<u64>,
    /// The raw node, for the unidentified `d` field (and `e.c` on Skills).
    pub raw: Node,
}

/// One monster fought to generate Battle and Divinity (root `k`). Same outer
/// shape as [`TrainingEntry`]; resolve the name with
/// [`crate::items::monster_name`] (`a` is the 0-based list position).
#[derive(Debug, Clone)]
pub struct Monster {
    /// Monster id / list position (`a`).
    pub id: u32,
    /// Number defeated (`b`).
    pub defeated: u64,
    /// Clones allocated to fighting this monster (`c`).
    pub clones: u32,
    /// The raw node, for the unidentified `d` field.
    pub raw: Node,
}

/// The Divinity Generator (root `K`). Decoded 2026-06-13 (upgrade levels moved
/// 81 → 188 together); the capacity/clones/storage fields were player-confirmed
/// 2026-06-18. The running **total** divinity is *not* here — it is the root `a`
/// scalar ([`SaveFile::total_divinity`]); `K.g` is only the amount currently
/// held in the generator.
#[derive(Debug, Clone)]
pub struct DivinityGenerator {
    /// Capacity currently in use (`K.g`) — how much divinity is held in the
    /// generator right now, not the total or the cap. A very large float. The
    /// total *capacity* (the cap) isn't stored nearby and is likely computed at
    /// runtime from the Capacity upgrade level.
    pub capacity_in_use: f64,
    /// Worker Clones allocated to the Divinity Generator (`K.c`).
    pub worker_clones: u64,
    /// Stones held in the generator's Stone Storage (`K.n`); a large float. As
    /// with capacity, the storage *cap* isn't stored nearby (likely computed).
    pub stone_storage: f64,
    /// The three upgrade tracks (`K.l`), in id order.
    pub upgrades: Vec<DivinityUpgrade>,
}

/// One Divinity Generator upgrade (`K.l[i]`). The three tracks are
/// 0 = Capacity, 1 = Divinity Gain, 2 = Converting Speed
/// ([`crate::items::divinity_upgrade_name`]); identical struct shape.
#[derive(Debug, Clone, Copy)]
pub struct DivinityUpgrade {
    /// Upgrade id (`a`), 0–2.
    pub id: u32,
    /// Current level (`b`).
    pub level: u64,
    /// "Next at" level (`f`) — player-confirmed 2026-06-18.
    pub next_at: u64,
    /// Clone-spread priority (`g`): 1, 2, 2 for the three tracks
    /// (player-confirmed 2026-06-18 — earlier mislabeled a per-level multiplier).
    pub spread: u32,
}

impl DivinityUpgrade {
    /// Display name (Capacity / Divinity Gain / Converting Speed), if recognized.
    pub fn name(&self) -> Option<&'static str> {
        crate::items::divinity_upgrade_name(self.id)
    }
}

/// Unused-GP god-stat allocation split (root `p.r/s/t/u`), in percent.
/// Resolved 2026-06-13 by skewing the in-game split to 25/21/22/27 and
/// watching `r/s/t/u` follow.
#[derive(Debug, Clone, Copy)]
pub struct GpAllocation {
    /// `p.r` — bonus physical god-stat %.
    pub physical: u32,
    /// `p.s` — bonus mystic god-stat %.
    pub mystic: u32,
    /// `p.t` — bonus battle god-stat %.
    pub battle: u32,
    /// `p.u` — bonus creating god-stat %.
    pub creating: u32,
}

/// Permanent account upgrades, read from the numeric keys of the `root.p`
/// block. Despite FINDINGS historically calling `p` the "god-power block", it
/// holds *all* permanent purchases — god-power buys **and** pet-stone buys
/// sit side by side. These fields are the pet-stone permanent upgrades, keyed
/// off the wiki's purchase list.
///
/// Confidence varies (see each field). The block as a whole was validated when
/// `p.001` ticked **5 → 6** between the 2026-06-13 and 2026-06-16 saves, the
/// exact rebirth-independent move of buying the last "Max Crystal".
#[derive(Debug, Clone, Copy)]
pub struct PermanentUpgrades {
    /// `p.001` — "Max Crystal": number of crystals equippable at once (caps at
    /// 6). **Confirmed** by the 5 → 6 move across the 06-13 → 06-16 saves.
    pub max_crystal: u32,
    /// `p.018` — "Inventory Space": the equipment-storage limit (250 here).
    /// **High confidence** (exact match to the in-game limit, permanent).
    pub inventory_limit: u32,
    /// `p.021` — "Item Slot": distinct dungeon party-item slots, caps at 8
    /// (maxed here). **High confidence** (exact, and the equipped-item loadout
    /// list `X.013` has exactly this many entries).
    pub item_slots: u32,
    /// `p.025` — "Camp Exp Boost": the extra % class XP adventurer pets earn in
    /// campaigns (+25%/level, caps at +100% = maxed here). This is the value the
    /// Growth Chamber sim's `adv_xp_mult` wants (maxed ⇒ ×2). **Confirmed**
    /// 2026-06-16 by a `save-edit` diff: setting `p.025` to 75 lowered the
    /// in-game Camp Exp Boost to +75% while the Baal-Slayer double-points chance
    /// (the colliding `p.E`, also 100) stayed at 100% — so `p.025` is Camp Exp
    /// Boost and `p.E` is the unrelated TBS field.
    pub camp_exp_boost_pct: u32,
    /// `p.017` — "Dungeon Loot": +% loot found in dungeons (+25%/level, caps at
    /// +50%). **Confirmed** 2026-06-16 by a fresh-save diff: buying 2 Dungeon
    /// Loot moved `p.017` 0 → 50.
    pub dungeon_loot_pct: u32,
    /// `p.019` — "Dungeon Exp": +% exp received in dungeons (+25%/level, caps at
    /// +50%). **Confirmed** by the same diff: buying 1 Dungeon Exp moved `p.019`
    /// 0 → 25. (Resolves which of the two `50`s in the main save is which, and
    /// supersedes the earlier "stat-multi doubling count" guess for these keys.)
    pub dungeon_exp_pct: u32,
    /// `p.020` — "Crafting Boost": +% crafting quality/speed for blacksmiths &
    /// alchemists (+25%, single purchase). **Confirmed** 2026-06-16 by a
    /// fresh-save buy (0 → 25). Crystal Improve — the other +25% candidate for
    /// this key — is a *different* field (untested: needs crystals to unlock).
    pub crafting_boost_pct: u32,
}

impl PermanentUpgrades {
    fn from_p(p: &Node) -> Self {
        PermanentUpgrades {
            max_crystal: get_u32(p, "001"),
            inventory_limit: get_u32(p, "018"),
            item_slots: get_u32(p, "021"),
            camp_exp_boost_pct: get_u32(p, "025"),
            dungeon_loot_pct: get_u32(p, "017"),
            dungeon_exp_pct: get_u32(p, "019"),
            crafting_boost_pct: get_u32(p, "020"),
        }
    }
}

/// The Baal Slayer (TBS) component levels, stored in the `root.S` block. Each
/// of the five body parts (`S.b/c/d/e/f`) levels independently and the value
/// is the displayed level directly; the parts reset partially on rebirth.
/// Resolved 2026-06-16: the player set each part to a distinct level
/// (125/132/127/128/130) so the letter→part mapping is unambiguous — earlier
/// saves had all five at 126 (the "all five 126" reading in FINDINGS).
///
/// `S.a` (a constant 99.56…) and `S.g` (0) are not levels and stay
/// unidentified. The displayed crit-chance, crit-damage and **score** are all
/// *derived* from these levels (+ SpaceDim), not stored — see
/// [`TbsLevels::score`].
#[derive(Debug, Clone, Copy)]
pub struct TbsLevels {
    /// `S.b` — Eyes. The player levels this "mirrored" for a bigger bonus;
    /// mirrored eyes count 4× toward the score.
    pub eyes: u32,
    /// `S.d` — Wings.
    pub wings: u32,
    /// `S.e` — Tail.
    pub tail: u32,
    /// `S.f` — Feet.
    pub feet: u32,
    /// `S.c` — Mouth.
    pub mouth: u32,
}

impl TbsLevels {
    fn from_node(s: &Node) -> Self {
        TbsLevels {
            eyes: get_u32(s, "b"),
            mouth: get_u32(s, "c"),
            wings: get_u32(s, "d"),
            tail: get_u32(s, "e"),
            feet: get_u32(s, "f"),
        }
    }

    /// The in-game Baal-Slayer "score": eyes count **4×** (this account levels
    /// them mirrored), every other part 1×. Verified against the displayed
    /// 1017 = 4·125 + 127 + 128 + 130 + 132. Assumes mirrored eyes; the mirror
    /// flag itself has not been located in the save.
    pub fn score(&self) -> u32 {
        4 * self.eyes + self.wings + self.tail + self.feet + self.mouth
    }
}

/// One adventure-mode research (root `032.H.a[i]`).
///
/// Verified 43/43 against the Main Stats export's "Researches" section
/// (same order as the export, 1-based: id 1 = God HP … id 43 = Core
/// Removal Cost). Exactly two entries had `in_progress` set, matching
/// "Research Slots Level: 2".
#[derive(Debug, Clone, Copy)]
pub struct Research {
    /// Research id (`a`). See [`research_name`] / [`researches`].
    pub id: u32,
    /// Current level (`b`).
    pub level: u32,
    /// Maximum level (`f`).
    pub max_level: u32,
    /// Currently being researched (`c` = 1).
    pub in_progress: bool,
    /// Accumulated research progress toward the next level (`d`).
    pub progress: f64,
}

/// Research id constants (the ones the planner is likely to care about).
pub mod researches {
    /// "Multiplies the stats your pets gain from growth (not dungeon
    /// stats)" — +1% per level. This is the ×1.05 factor in the
    /// normal-stats global multiplier at level 5.
    pub const PET_STATS: u32 = 28;
    pub const CRAFTING_EXP: u32 = 16;
    pub const CRAFTING_SPEED: u32 = 17;
    pub const SMITHING_EXP: u32 = 18;
    pub const SMITHING_SPEED: u32 = 19;
    pub const ALCHEMY_EXP: u32 = 20;
    pub const ALCHEMY_SPEED: u32 = 21;
}

/// Display name for a research id, in Main Stats export order.
pub fn research_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 43] = [
        "God HP",
        "God Attack",
        "God Mystic",
        "Building Speed",
        "Creating Speed",
        "Core Drop Rate",
        "Core Quality",
        "Drop Rate",
        "Exp Gain",
        "Equip Attack",
        "Equip Def",
        "Equip Int",
        "Equip Res",
        "Equip Hit",
        "Equip Speed",
        "Crafting Exp",
        "Crafting Speed",
        "Smithing Exp",
        "Smithing Speed",
        "Alchemy Exp",
        "Alchemy Speed",
        "Equip Quality Min",
        "Equip Quality Multi",
        "Active Skill Slot",
        "Passive Skill Slot",
        "Research Speed",
        "Research Slots",
        "Pet Stats",
        "Equip Hp",
        "Might Speed",
        "Spacedim Speed",
        "Multiverse Speed",
        "Max Class Lv",
        "Side Crafting Speed",
        "Min Pow",
        "Enemy Min Lv",
        "Reduce Class Lv Req",
        "Crit Chance",
        "Crit Dam",
        "No Overkill Crit",
        "Max Skill Lv",
        "Skill Experience",
        "Core Removal Cost",
    ];
    // id 0 is an unused placeholder entry in the save.
    if id >= 1 && (id as usize) <= NAMES.len() {
        Some(NAMES[id as usize - 1])
    } else {
        None
    }
}

/// One campaign slot (`X.x[i]`).
#[derive(Debug, Clone)]
pub struct CampaignSlot {
    /// Slot index (`a`).
    pub index: u32,
    /// Pet type ids assigned to the campaign (`d`).
    pub pet_type_ids: Vec<u32>,
    /// Campaign duration in ms (`e`), 43,200,000 = 12 h.
    pub duration_ms: u64,
    /// Bonus value (`f`) — semantics not yet pinned down.
    pub bonus: u64,
}

/// Identified keys in the global tracker block (root `x`), for use with
/// [`SaveFile::global_tracker`]. All confirmed 2026-06-11 by diffing the two
/// reference saves against in-game tooltip readings and the Main Stats
/// exports (each per-pet value matched the user's predicted day-over-day
/// delta).
pub mod trackers {
    // -- per-pet special trackers --
    /// Chocobear: hours banked for the non-food-campaign bonus.
    pub const CHOCOBEAR_BANKED_HOURS: &str = "089";
    /// Pandora's box: feedings counter this rebirth. Observed negative
    /// (-28) right after a rebirth, so it is not a plain count.
    pub const PANDORA_FEEDINGS: &str = "169";
    /// Earth Eater: Earthlike planets eaten, lifetime total.
    pub const EARTH_EATER_PLANETS_TOTAL: &str = "185";
    /// Aether: Delirious Essence of the Forgotten kills (the Aether Ring's
    /// "+N" suffix shows the same number).
    pub const AETHER_BOSS_KILLS: &str = "186";
    /// Pignata: times bashed open.
    pub const PIGNATA_BASHES: &str = "216";
    /// God Power (pet): hours spent in God Power campaigns.
    pub const GOD_POWER_CAMPAIGN_HOURS: &str = "218";
    /// Meteor: total hours spent in campaigns (drives its campaign bonus).
    pub const METEOR_CAMPAIGN_HOURS: &str = "234";
    /// Caterpillar: total materials upgraded.
    pub const CATERPILLAR_MATERIALS_UPGRADED: &str = "259";
    /// Pack Mule: quests participated in.
    pub const MULE_QUESTS: &str = "310";
    /// Gold Dragon: bonus growth granted since July 2024.
    pub const GOLD_DRAGON_BONUS_GROWTH: &str = "311";
    /// Serow: total items saved in dungeons.
    pub const SEROW_ITEMS_SAVED: &str = "324";
    /// Bag: total bonus growth granted.
    pub const BAG_BONUS_GROWTH: &str = "336";
    /// Pet stones bought with Baal Power (global across rebirths) — the
    /// Vermillion Pheasant unlock progress (10,000 needed).
    pub const PET_STONES_BAAL_POWER: &str = "270";
    // NOTE: x.138 was briefly misidentified as Anni Cake's bonus because
    // floor(x.138/3600) happened to equal the displayed 949% in save 2 — a
    // genuine coincidence (save 1 disagrees: 911 vs the actual 709). The
    // real bonus is stored directly at root `033` (see
    // [`crate::SaveFile::anni_cake_bonus_percent`]). x.138 remains an
    // unidentified food/campaign-time-shaped counter.

    // -- global counters (cross-checked against the Main Stats exports) --
    /// AFK clones killed (lifetime).
    pub const AFK_CLONES_KILLED: &str = "013";
    /// Lucky Draws opened.
    pub const LUCKY_DRAWS_OPENED: &str = "071";
    /// Crystal power.
    pub const CRYSTAL_POWER: &str = "074";
    /// Dungeon bosses defeated.
    pub const DUNGEON_BOSSES: &str = "078";
    /// Dungeon enemies defeated.
    pub const DUNGEON_ENEMIES: &str = "079";
    /// Dungeon rooms beaten.
    pub const DUNGEON_ROOMS: &str = "080";
    /// Total might.
    pub const TOTAL_MIGHT: &str = "129";
}

pub fn element_from_id(id: u32) -> Option<Element> {
    match id {
        0 => Some(Element::Neutral),
        1 => Some(Element::Fire),
        2 => Some(Element::Water),
        3 => Some(Element::Earth),
        4 => Some(Element::Wind),
        _ => None,
    }
}

pub fn class_from_id(id: u32) -> Option<Class> {
    match id {
        1 => Some(Class::Blacksmith),
        2 => Some(Class::Alchemist),
        3 => Some(Class::Adventurer),
        4 => Some(Class::Defender),
        5 => Some(Class::Supporter),
        6 => Some(Class::Rogue),
        7 => Some(Class::Assassin),
        8 => Some(Class::Mage),
        _ => None, // 0 = no class
    }
}

impl SaveFile {
    /// Build the typed model from a parsed tree (see [`crate::parse_save`]
    /// for the one-call entry point).
    pub fn from_tree(root: Node) -> anyhow::Result<Self> {
        let x = root.get("X").context("save has no pet block (root key X)")?;

        let pets: Vec<SavePet> = x
            .get("b")
            .map(|b| b.list_or_single().iter().map(SavePet::from_node).collect())
            .unwrap_or_default();

        let equipment = x
            .get("R")
            .map(|r| {
                r.list_or_single()
                    .iter()
                    .map(EquipmentItem::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let materials = x
            .get("Q")
            .map(|q| {
                q.list_or_single()
                    .iter()
                    .map(|n| MaterialStack {
                        item_id: get_u32(n, "a"),
                        count: get_u64(n, "b"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let dungeon_teams = x
            .get("S")
            .map(|s| {
                s.list_or_single()
                    .iter()
                    .map(DungeonTeam::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let campaigns = x
            .get("x")
            .map(|c| {
                c.list_or_single()
                    .iter()
                    .map(CampaignSlot::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let gems = x
            .get("002")
            .map(|g| {
                g.list_or_single()
                    .iter()
                    .map(|n| {
                        let element_id = get_u32(n, "a");
                        GemStack {
                            element_id,
                            element: element_from_id(element_id),
                            level: get_u32(n, "b"),
                            count: get_u64(n, "c"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let researches = root
            .get_path(&["032", "H", "a"])
            .map(|r| {
                r.list_or_single()
                    .iter()
                    .map(|n| Research {
                        id: get_u32(n, "a"),
                        level: get_u32(n, "b"),
                        max_level: get_u32(n, "f"),
                        in_progress: get_u32(n, "c") == 1,
                        progress: get_f64(n, "d"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let creations = root
            .get("i")
            .map(|l| {
                l.list_or_single()
                    .iter()
                    .map(|n| Creation {
                        id: get_u32(n, "a"),
                        next_at: get_u64(n, "i"),
                        current_amount: get_f64(n, "d"),
                        created: get_f64(n, "g"),
                        clone_cost: get_u64(n, "e"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let monuments = root
            .get("D")
            .map(|l| {
                l.list_or_single()
                    .iter()
                    .map(|n| Monument {
                        id: get_u32(n, "a"),
                        level: get_u64(n, "b"),
                        next_at: get_u64(n, "g"),
                        spread: get_u32(n, "h"),
                        building: n.get("f").and_then(Node::as_bool).unwrap_or(false),
                        clones_allocated: get_u64(n, "c"),
                        progress: get_f64(n, "d"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mights = root
            .get("V")
            .map(|l| {
                l.list_or_single()
                    .iter()
                    .map(|n| Might {
                        id: get_u32(n, "a"),
                        level: get_u64(n, "b"),
                        next_at: get_u64(n, "m"),
                        spread: get_u32(n, "n"),
                        special: n.get("e").and_then(Node::as_bool).unwrap_or(false),
                        base_duration_s: get_u32(n, "g"),
                        hp_recovery_pct: get_u32(n, "i"),
                        attack_pct: get_u32(n, "j"),
                        mystic_pct: get_u32(n, "k"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let spacedim = root
            .get_path(&["009", "b"])
            .map(|l| {
                l.list_or_single()
                    .iter()
                    .map(|n| SpaceDimElement {
                        id: get_u32(n, "a"),
                        clones: get_u64(n, "b"),
                        level: get_u64(n, "c"),
                        next_at: get_u64(n, "d"),
                        progress: get_f64(n, "e"),
                        spread: get_u32(n, "f"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let parse_trainings = |key: &str| {
            root.get(key)
                .map(|l| {
                    l.list_or_single()
                        .iter()
                        .map(|n| TrainingEntry {
                            id: get_u32(n, "a"),
                            level: get_u64(n, "b"),
                            clones: get_u32(n, "c"),
                            usage_count: n.get("e").and_then(|e| e.get("b")).and_then(Node::as_u64),
                            raw: n.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let physical_trainings = parse_trainings("h");
        let skills = parse_trainings("j");

        let monsters = root
            .get("k")
            .map(|l| {
                l.list_or_single()
                    .iter()
                    .map(|n| Monster {
                        id: get_u32(n, "a"),
                        defeated: get_u64(n, "b"),
                        clones: get_u32(n, "c"),
                        raw: n.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let divinity_generator = root.get("K").map(|k| DivinityGenerator {
            capacity_in_use: k.get("g").and_then(Node::as_f64).unwrap_or(0.0),
            worker_clones: get_u64(k, "c"),
            stone_storage: k.get("n").and_then(Node::as_f64).unwrap_or(0.0),
            upgrades: k
                .get("l")
                .map(|l| {
                    l.list_or_single()
                        .iter()
                        .map(|n| DivinityUpgrade {
                            id: get_u32(n, "a"),
                            level: get_u64(n, "b"),
                            next_at: get_u64(n, "f"),
                            spread: get_u32(n, "g"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });

        let gp_allocation = root.get("p").map(|p| GpAllocation {
            physical: get_u32(p, "r"),
            mystic: get_u32(p, "s"),
            battle: get_u32(p, "t"),
            creating: get_u32(p, "u"),
        });

        let permanent_upgrades = root.get("p").map(PermanentUpgrades::from_p);
        let tbs_levels = root.get("S").map(TbsLevels::from_node);

        Ok(SaveFile {
            saved_at_unix: root.get("c").and_then(Node::as_i64),
            god_name: root.get("W").and_then(Node::as_str).map(str::to_string),
            account_name: root.get("s").and_then(Node::as_str).map(str::to_string),
            pet_stones: x.get("y").and_then(Node::as_u64),
            pet_stones_spent: x.get("z").and_then(Node::as_u64),
            crafting_queue_slots: get_u32(x, "032"),
            pet_tokens: root.get_path(&["p", "I"]).and_then(Node::as_u64),
            class_change_tokens: root.get_path(&["p", "023"]).and_then(Node::as_u64),
            free_experience: root.get_path(&["X", "Y"]).and_then(Node::as_u64),
            lucky_draws: root.get_path(&["p", "K"]).and_then(Node::as_u64),
            godly_liquid: root.get_path(&["p", "b"]).and_then(Node::as_u64),
            godly_liquid_v2: root.get_path(&["p", "m"]).and_then(Node::as_u64),
            chakra_pill: root.get_path(&["p", "d"]).and_then(Node::as_u64),
            chakra_pill_v2: root.get_path(&["p", "n"]).and_then(Node::as_u64),
            ultimate_shadow_summon: root.get_path(&["p", "e"]).and_then(Node::as_u64),
            puny_food: get_u64(x, "c"),
            strong_food: get_u64(x, "d"),
            mighty_food: get_u64(x, "e"),
            chocolate: get_u64(x, "v"),
            gems,
            gp_available: root.get_path(&["p", "j"]).and_then(Node::as_u64),
            gp_spent: root.get_path(&["p", "v"]).and_then(Node::as_u64),
            total_might: root.get_path(&["p", "F"]).and_then(Node::as_u64),
            crystal_power: root.get_path(&["p", "002"]).and_then(Node::as_u64),
            rebirths: root.get_path(&["x", "k"]).and_then(Node::as_u64),
            light_clones: root.get_path(&["O", "030"]).and_then(Node::as_u64),
            statistics_multi: root.get_path(&["p", "C"]).and_then(Node::as_f64),
            creation_count_gp: root.get_path(&["p", "q"]).and_then(Node::as_u64),
            earth_eater_planets_rebirth: root.get("018").and_then(Node::as_u64),
            anni_cake_bonus_percent: root.get("033").and_then(Node::as_f64),
            researches,
            creations,
            monuments,
            mights,
            spacedim,
            physical_trainings,
            skills,
            monsters,
            total_divinity: root.get("a").and_then(Node::as_f64),
            divinity_generator,
            baal_power: root.get_path(&["T", "h"]).and_then(Node::as_u64),
            current_god_number: root.get_path(&["P", "c"]).and_then(Node::as_u32),
            gp_creating_speed_pct: root.get_path(&["p", "h"]).and_then(Node::as_u64),
            gp_building_speed_pct: root.get_path(&["p", "i"]).and_then(Node::as_u64),
            gp_allocation,
            permanent_upgrades,
            tbs_levels,
            pets,
            equipment,
            materials,
            dungeon_teams,
            campaigns,
            root,
        })
    }

    /// Read a numeric value from the global tracker block (root `x`) — a
    /// flat struct of numeric-keyed game counters, including the per-pet
    /// special trackers. See [`trackers`] for the identified keys.
    pub fn global_tracker(&self, key: &str) -> Option<f64> {
        self.root.get_path(&["x", key]).and_then(Node::as_f64)
    }

    /// Sum of all current might levels — the White Tiger unlock progress
    /// (25,000 combined levels needed in one rebirth).
    pub fn might_level_total(&self) -> u64 {
        self.mights.iter().map(|m| m.level).sum()
    }

    /// Level of a research by id (0 if absent). See [`researches`] for ids.
    pub fn research_level(&self, id: u32) -> u32 {
        self.researches
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.level)
            .unwrap_or(0)
    }

    pub fn pet_by_type_id(&self, type_id: u32) -> Option<&SavePet> {
        self.pets.iter().find(|p| p.type_id == type_id)
    }

    pub fn pet_by_name(&self, name: &str) -> Option<&SavePet> {
        self.pets.iter().find(|p| p.name == name)
    }

    pub fn equipment_by_instance_id(&self, instance_id: u32) -> Option<&EquipmentItem> {
        self.equipment
            .iter()
            .find(|e| e.instance_id == instance_id)
    }

    /// Highest P. Baal defeated. The save stores the *current* target
    /// ([`current_god_number`](Self::current_god_number) = `P.c`), which is
    /// one past the highest defeated, so this subtracts 1. `None` if absent.
    pub fn pbaal_defeated(&self) -> Option<u32> {
        self.current_god_number.map(|c| c.saturating_sub(1))
    }

    /// A SpaceDim element by its 1-based display id (1 = Controlled Entropy).
    pub fn spacedim_by_id(&self, id: u32) -> Option<&SpaceDimElement> {
        self.spacedim.iter().find(|e| e.id == id)
    }
}

impl SavePet {
    /// Current normal Physical stat (= Health / 10).
    pub fn physical_stat(&self) -> f64 {
        self.normal_health / 10.0
    }

    fn from_node(node: &Node) -> Self {
        let w = node.get("w");
        let class_node = w.and_then(|w| w.get("d"));
        let partner_raw = node.get("F").and_then(Node::as_u32);
        SavePet {
            name: node
                .get("a")
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string(),
            type_id: get_u32(node, "k"),
            unlocked: node.get("l").and_then(Node::as_bool).unwrap_or(false),
            growth: node.get("E").and_then(Node::as_f64).unwrap_or(0.0),
            normal_level: get_u32(node, "g"),
            normal_health: get_f64(node, "j"),
            clone_physical: get_f64(node, "o"),
            clone_mystic: get_f64(node, "p"),
            clone_battle: get_f64(node, "q"),
            clone_hp: get_f64(node, "r"),
            team_slot: match get_u32(node, "v") {
                0 => None,
                slot => u8::try_from(slot).ok(),
            },
            element: w.map(|w| get_u32(w, "a")).and_then(element_from_id),
            dungeon_level: w.map(|w| get_u32(w, "b")).unwrap_or(0),
            dungeon_exp: w.map(|w| get_f64(w, "c")).unwrap_or(0.0),
            class: class_node.map(|d| get_u32(d, "a")).and_then(class_from_id),
            class_level: class_node.map(|d| get_u32(d, "b")).unwrap_or(0),
            class_exp: class_node.map(|d| get_f64(d, "c")).unwrap_or(0.0),
            weapon_id: w.and_then(|w| nonzero_u32(w, "e")),
            armor_id: w.and_then(|w| nonzero_u32(w, "f")),
            accessory_id: w.and_then(|w| nonzero_u32(w, "g")),
            partner_type_id: partner_raw.filter(|&id| id != PARTNER_NONE),
            partner_days: get_u64(node, "G"),
            current_exp: get_f64(node, "h"),
            working_experience_ms: get_u64(node, "H"),
            raw: node.clone(),
        }
    }
}

/// Save quality id for quality "A" — the wiki's stat baseline. Each step
/// above/below shifts the multiplier ±10% (additively).
const QUALITY_A: u32 = 5;

impl EquipmentItem {
    /// Display name of the item type, if identified.
    pub fn type_name(&self) -> Option<&'static str> {
        crate::items::equipment_type_name(self.type_id)
    }

    /// Quality letter (F E D C B A S SS SSS for 0…8) — see
    /// [`crate::items::quality_name`].
    pub fn quality_name(&self) -> Option<&'static str> {
        crate::items::quality_name(self.quality)
    }

    /// Quality multiplier on the catalogued A+0 stat percentages:
    /// ±10% per quality step around A (wiki: C ⇒ 0.8, SSS ⇒ 1.3).
    pub fn quality_multiplier(&self) -> f64 {
        1.0 + (self.quality as f64 - QUALITY_A as f64) * 0.1
    }

    /// Upgrade multiplier: +5% per "+" level (wiki: +8 ⇒ 1.4, +20 ⇒ 2.0).
    pub fn upgrade_multiplier(&self) -> f64 {
        1.0 + 0.05 * self.plus as f64
    }

    /// Combined multiplier applied to the item's A+0 stat percentages
    /// (quality × upgrade; wiki: SSS +20 ⇒ 1.3 × 2.0 = 2.6).
    pub fn stat_multiplier(&self) -> f64 {
        self.quality_multiplier() * self.upgrade_multiplier()
    }

    fn from_node(node: &Node) -> Self {
        let gem_level = get_u32(node, "f");
        EquipmentItem {
            type_id: get_u32(node, "a"),
            plus: get_u32(node, "b"),
            quality: get_u32(node, "c"),
            instance_id: get_u32(node, "d"),
            plus_cap: get_u32(node, "e"),
            gem_level,
            gem_element: if gem_level > 0 {
                element_from_id(get_u32(node, "g"))
            } else {
                None
            },
        }
    }
}

impl DungeonTeam {
    fn from_node(node: &Node) -> Self {
        DungeonTeam {
            dungeon_id: get_u32(node, "b"),
            depth: get_u32(node, "d"),
            dungeon_name: node
                .get("i")
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string(),
            pet_type_ids: node
                .get("a")
                .and_then(Node::as_int_list)
                .unwrap_or_default(),
            loot: node
                .get("c")
                .map(|c| {
                    c.list_or_single()
                        .iter()
                        .map(|n| (get_u32(n, "a"), get_u64(n, "b")))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl CampaignSlot {
    fn from_node(node: &Node) -> Self {
        CampaignSlot {
            index: get_u32(node, "a"),
            pet_type_ids: node
                .get("d")
                .and_then(Node::as_int_list)
                .unwrap_or_default(),
            duration_ms: get_u64(node, "e"),
            bonus: get_u64(node, "f"),
        }
    }
}

fn get_u32(node: &Node, key: &str) -> u32 {
    node.get(key).and_then(Node::as_u32).unwrap_or(0)
}

fn get_u64(node: &Node, key: &str) -> u64 {
    node.get(key).and_then(Node::as_u64).unwrap_or(0)
}

fn get_f64(node: &Node, key: &str) -> f64 {
    node.get(key).and_then(Node::as_f64).unwrap_or(0.0)
}

fn nonzero_u32(node: &Node, key: &str) -> Option<u32> {
    match get_u32(node, key) {
        0 => None,
        v => Some(v),
    }
}
