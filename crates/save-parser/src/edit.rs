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

/// Grant a material: set the count of the `X.Q` inventory entry with item id
/// `id` to `count`, **adding the entry if the save doesn't have it yet** (a
/// fresh account has no stack for most materials, so a plain scalar set can't
/// reach them).
#[derive(Debug, Clone)]
pub struct MaterialGrant {
    pub id: String,
    pub count: String,
}

/// Grant a piece of equipment to a pet: create a new instance in the `X.R`
/// equipment list and equip it in the pet's chosen slot. Instance ids are
/// allocated above the highest existing one. The new instance is built like the
/// game's (`a` type, `b` plus, `c` quality, `d`/`h` instance id, `e=20` plus
/// cap, `f`/`g` gem = 0, `i=0`).
#[derive(Debug, Clone)]
pub struct EquipGrant {
    /// Pet to equip on (index in `X.b`) and its slot (`e` weapon, `f` armor,
    /// `g` accessory). Set **both** to equip; leave **both** `None` to add the
    /// instance to the `X.R` inventory unequipped. (A half-specified pair —
    /// one `Some`, one `None` — is treated as inventory-only.)
    pub pet_index: Option<u32>,
    pub slot: Option<char>,
    /// Equipment type id (`a`), e.g. 51 = Magic Stick.
    pub type_id: u32,
    /// Plus level (`b`).
    pub plus: u32,
    /// Quality (`c`): 8 = SSS, 6 = S, 5 = A, …
    pub quality: u32,
}

/// What to do to the scalar a [`ScalarEdit`] points at.
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Overwrite with this verbatim text.
    Set(String),
    /// Multiply the current numeric value by this factor.
    Mul(f64),
}

/// One field edit: the dotted raw-tree path plus the operation to apply.
#[derive(Debug, Clone)]
pub struct ScalarEdit {
    pub path: Vec<String>,
    pub op: EditOp,
}

impl ScalarEdit {
    fn split_path(path: &str) -> Vec<String> {
        path.split('.').map(str::to_string).collect()
    }

    /// Set a dotted path (e.g. `"p.025"`) to a verbatim value.
    pub fn set(path: &str, value: &str) -> Self {
        ScalarEdit {
            path: Self::split_path(path),
            op: EditOp::Set(value.to_string()),
        }
    }

    /// Multiply the value at a dotted path by `factor`.
    pub fn mul(path: &str, factor: f64) -> Self {
        ScalarEdit {
            path: Self::split_path(path),
            op: EditOp::Mul(factor),
        }
    }

    /// Back-compat alias for [`ScalarEdit::set`].
    pub fn parse(path: &str, value: &str) -> Self {
        Self::set(path, value)
    }
}

/// Unwrap any [`raw::Raw::Base64`] wrappers, by value.
fn peel_owned(r: raw::Raw) -> raw::Raw {
    match r {
        raw::Raw::Base64(inner) => peel_owned(*inner),
        other => other,
    }
}

/// Borrow the `X.<key>` list, normalizing an empty field or a lone struct (a
/// 1-element list with no `&` separator) into a real list first. Used for the
/// material inventory (`Q`) and equipment (`R`).
fn ensure_list<'a>(root: &'a mut raw::Raw, key: &str) -> Result<&'a mut Vec<raw::Raw>> {
    let x = root.get_path_mut(&["X"]).context("save has no X block")?;
    let raw::Raw::Struct(fields) = x else {
        anyhow::bail!("X is not a struct");
    };
    let entry = fields
        .iter_mut()
        .find(|(k, _)| k == key)
        .with_context(|| format!("X has no {key} field"))?;
    match &mut entry.1 {
        raw::Field::Value(raw::Raw::List(_)) => {}
        f @ (raw::Field::EmptyColon | raw::Field::EmptyBare) => {
            *f = raw::Field::Value(raw::Raw::List(Vec::new()));
        }
        raw::Field::Value(v) if matches!(v.peel(), raw::Raw::Struct(_)) => {
            let only = std::mem::replace(v, raw::Raw::List(Vec::new()));
            if let raw::Raw::List(items) = v {
                items.push(peel_owned(only));
            }
        }
        raw::Field::Value(_) => anyhow::bail!("X.{key} is present but not a list"),
    }
    match &mut entry.1 {
        raw::Field::Value(raw::Raw::List(items)) => Ok(items),
        _ => unreachable!("X.{key} normalized to a list above"),
    }
}

/// A material-inventory element `{a:id, b:count}`.
fn material_entry(id: &str, count: &str) -> raw::Raw {
    let val = |s: &str| raw::Field::Value(raw::Raw::Scalar(s.to_string()));
    raw::Raw::Struct(vec![("a".into(), val(id)), ("b".into(), val(count))])
}

/// An equipment instance shaped like the game's (see the `EquipmentItem` docs):
/// `a` type, `b` plus, `c` quality, `d`/`h` instance id, `e=20` plus cap,
/// `f`/`g` gem = 0, `i=0`.
fn equip_instance(id: u32, eq: &EquipGrant) -> raw::Raw {
    let val = |s: String| raw::Field::Value(raw::Raw::Scalar(s));
    raw::Raw::Struct(vec![
        ("a".into(), val(eq.type_id.to_string())),
        ("b".into(), val(eq.plus.to_string())),
        ("c".into(), val(eq.quality.to_string())),
        ("d".into(), val(id.to_string())),
        ("e".into(), val("20".into())),
        ("f".into(), val("0".into())),
        ("g".into(), val("0".into())),
        ("h".into(), val(id.to_string())),
        ("i".into(), val("0".into())),
    ])
}

/// Highest equipment instance id (`d`) in `X.R`, or 0 if none. Tolerates an
/// empty/absent `R`, a list, or a lone struct (1-element list).
fn max_instance_id(root: &raw::Raw) -> u32 {
    let Some(r) = root.get_path(&["X", "R"]) else {
        return 0;
    };
    let elems: &[raw::Raw] = match r {
        raw::Raw::List(items) => items,
        raw::Raw::Struct(_) => std::slice::from_ref(r),
        _ => return 0,
    };
    elems
        .iter()
        .filter_map(|e| match e.get("d") {
            Some(raw::Raw::Scalar(s)) => s.parse::<u32>().ok(),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// Multiply a numeric save value (verbatim text) by `factor`, returning the new
/// verbatim text. Integer-looking inputs that stay whole stay integers; anything
/// else is formatted as a float (the game re-parses doubles, so an exact byte
/// match isn't required). Reused by the GUI's bulk pet editor for "× growth".
pub fn apply_factor(current: &str, factor: f64) -> Result<String> {
    let v: f64 = current
        .parse()
        .with_context(|| format!("value {current:?} is not numeric — can't multiply"))?;
    let r = v * factor;
    anyhow::ensure!(r.is_finite(), "multiplying {current:?} by {factor} is not finite");
    let looks_integer = !current.contains(['.', 'e', 'E']);
    if looks_integer && r.fract() == 0.0 && r.abs() < 9.0e18 {
        Ok(format!("{}", r as i64))
    } else {
        Ok(format!("{r}"))
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
        // Pet stones (root `X.y`), verified against the Main Stats export.
        "stones" => Some(&["X", "y"]),
        _ => None,
    }
}

/// Decode `raw_save`, apply every edit to the lossless tree, and re-encode the
/// container. Returns the new save text plus a record of each change.
///
/// After encoding, it decodes the result again and confirms each edited path
/// now reads the requested value — so a serializer/encoder bug surfaces here
/// rather than as a corrupt save the game silently rejects.
pub fn edit_save(
    raw_save: &str,
    edits: &[ScalarEdit],
    materials: &[MaterialGrant],
    equips: &[EquipGrant],
) -> Result<(String, Vec<AppliedEdit>)> {
    let decoded = container::decode_container(raw_save).context("decode save container")?;
    let mut root = raw::parse(&decoded.plaintext);

    let mut applied = Vec::with_capacity(edits.len() + materials.len() + equips.len());

    // Material grants: set-or-add an entry in the X.Q inventory list.
    for mat in materials {
        let items = ensure_list(&mut root, "Q")?;
        let existing = items
            .iter()
            .position(|e| matches!(e.get("a"), Some(raw::Raw::Scalar(s)) if *s == mat.id));
        let old = match existing {
            Some(i) => {
                let prev = items[i].get("b").map_or_else(|| "0".into(), raw::Raw::serialize);
                items[i].set_scalar_path(&["b"], &mat.count)?;
                prev
            }
            None => {
                items.push(material_entry(&mat.id, &mat.count));
                "(absent)".into()
            }
        };
        applied.push(AppliedEdit {
            path: format!("X.Q.a={}.b", mat.id),
            old,
            new: mat.count.clone(),
        });
    }

    // Equipment grants: append a new instance to X.R and equip it on the pet.
    if !equips.is_empty() {
        let mut next_id = max_instance_id(&root) + 1;
        for eq in equips {
            // Append the instance to X.R (created if the list is empty).
            let id = next_id;
            next_id += 1;
            ensure_list(&mut root, "R")?.push(equip_instance(id, eq));
            let id_str = id.to_string();
            match (eq.pet_index, eq.slot) {
                (Some(pet), Some(slot)) => {
                    let pet = pet.to_string();
                    let slot = slot.to_string();
                    let old = root
                        .set_scalar_path(&["X", "b", &pet, "w", &slot], &id_str)
                        .with_context(|| format!("equip pet {pet} slot {slot}"))?;
                    applied.push(AppliedEdit {
                        path: format!("X.b.{pet}.w.{slot}"),
                        old,
                        new: id_str,
                    });
                }
                // Inventory-only: instance added to X.R, not equipped. Report a
                // verifiable path (the new instance's type, found by its id) so
                // the self-check below can confirm it.
                _ => applied.push(AppliedEdit {
                    path: format!("X.R.d={id_str}.a"),
                    old: "(added)".into(),
                    new: eq.type_id.to_string(),
                }),
            }
        }
    }

    for edit in edits {
        let segs: Vec<&str> = edit.path.iter().map(String::as_str).collect();
        // Resolve the new value (Mul reads the current scalar first).
        let new_value = match &edit.op {
            EditOp::Set(v) => v.clone(),
            EditOp::Mul(factor) => {
                let cur = match root.get_path(&segs) {
                    Some(raw::Raw::Scalar(s)) => s.clone(),
                    _ => anyhow::bail!("{} is not a scalar to multiply", edit.path.join(".")),
                };
                apply_factor(&cur, *factor)?
            }
        };
        let old = root
            .set_scalar_path(&segs, &new_value)
            .with_context(|| format!("edit {}", edit.path.join(".")))?;
        applied.push(AppliedEdit {
            path: edit.path.join("."),
            old,
            new: new_value,
        });
    }

    let reserialized = root.serialize();
    let encoded = container::encode_container(&reserialized, &decoded.prefix);

    // Self-check: the re-encoded save must decode back and read the new values.
    let check_plaintext = container::decode_to_plaintext(&encoded)
        .context("re-decode the edited save for verification")?;
    let check_root = raw::parse(&check_plaintext);
    for a in &applied {
        let segs: Vec<&str> = a.path.split('.').collect();
        match check_root.get_path(&segs) {
            Some(raw::Raw::Scalar(s)) if *s == a.new => {}
            other => anyhow::bail!(
                "verification failed for {}: expected {:?}, found {:?}",
                a.path,
                a.new,
                other
            ),
        }
    }

    Ok((encoded, applied))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_factor_keeps_integers_integer() {
        assert_eq!(apply_factor("192164", 10.0).unwrap(), "1921640");
        assert_eq!(apply_factor("5", 3.0).unwrap(), "15");
    }

    #[test]
    fn apply_factor_handles_floats() {
        // Growth-style value stays fractional.
        assert_eq!(apply_factor("66841.5", 2.0).unwrap(), "133683");
        assert_eq!(apply_factor("100", 1.5).unwrap(), "150"); // int that stays whole
        assert!(apply_factor("100.0", 1.5).unwrap().starts_with("150"));
    }

    #[test]
    fn apply_factor_rejects_non_numeric() {
        assert!(apply_factor("True", 2.0).is_err());
        assert!(apply_factor("Salamander", 2.0).is_err());
    }

    #[test]
    fn grants_a_single_material_to_empty_inventory() {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD;
        // Minimal save: X (a base64 nested struct) with an EMPTY Q field.
        let x_inner = b64.encode("y:0;Q:;".as_bytes());
        let plaintext = format!("X:{x_inner};");
        let save = container::encode_container(&plaintext, "V2");

        // Granting ONE material builds a 1-element list — the case that broke
        // the self-verify before the singleton-selector fix.
        let (encoded, applied) = edit_save(
            &save,
            &[],
            &[MaterialGrant { id: "5".into(), count: "400000".into() }],
            &[],
        )
        .expect("single grant to empty X.Q should succeed");
        assert_eq!(applied[0].old, "(absent)");

        let root = raw::parse(&container::decode_to_plaintext(&encoded).unwrap());
        assert_eq!(
            root.get_path(&["X", "Q", "a=5", "b"]),
            Some(&raw::Raw::Scalar("400000".into()))
        );
    }
}
