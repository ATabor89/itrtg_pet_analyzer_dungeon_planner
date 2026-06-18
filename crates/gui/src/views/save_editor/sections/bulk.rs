//! Shared helpers for the table-style bulk-edit sections (pets, equipment):
//! the op kinds, sortable column headers, and the editable per-item override
//! cell. Filters, columns, and the field sets stay per-section — only the
//! generic mechanics live here.

use std::collections::HashMap;
use std::hash::Hash;

use eframe::egui::{self, RichText};

use crate::style;

/// How a bulk numeric op transforms the current value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Set,
    /// Multiply (growth only).
    Mul,
    Add,
}

pub fn op_label(k: OpKind) -> &'static str {
    match k {
        OpKind::Set => "Set",
        OpKind::Mul => "× Mul",
        OpKind::Add => "+ Add",
    }
}

/// A clickable column header showing the current sort arrow. Returns whether it
/// was clicked (the caller cycles the sort). Generic over the column enum.
pub fn sort_header<C: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: Option<(C, bool)>,
    title: &str,
    col: C,
) -> bool {
    let arrow = match current {
        Some((c, true)) if c == col => " ▲",
        Some((c, false)) if c == col => " ▼",
        _ => "",
    };
    ui.add(
        egui::Button::new(RichText::new(format!("{title}{arrow}")).strong().size(12.0)).frame(false),
    )
    .on_hover_text("Click to sort (asc → desc → off)")
    .clicked()
}

/// Cycle a column's sort: none → ascending → descending → none.
pub fn cycle_sort<C: PartialEq + Copy>(sort: &mut Option<(C, bool)>, col: C) {
    *sort = match *sort {
        Some((c, true)) if c == col => Some((col, false)),
        Some((c, false)) if c == col => None,
        _ => Some((col, true)),
    };
}

/// An editable override cell. Shows `default` (the bulk-op result) until the user
/// types something else, which becomes a per-item override; editing back to
/// `default` clears the override. A ● marks the cell when the effective value
/// differs from `current` (the stored value), with a `current → new` tooltip.
pub fn override_cell<K: Eq + Hash + Clone>(
    ui: &mut egui::Ui,
    key: K,
    default: &str,
    current: &str,
    cell_buffers: &mut HashMap<K, String>,
    overrides: &mut HashMap<K, String>,
) {
    let overridden = overrides.contains_key(&key);
    // Scope the cell_buffers borrow so it doesn't overlap `overrides` below.
    let (changed, buf_val) = {
        let buf = cell_buffers.entry(key.clone()).or_insert_with(|| default.to_string());
        if !overridden {
            // Track the bulk-op default until the user explicitly overrides.
            *buf = default.to_string();
        }
        let resp = ui.add(
            egui::TextEdit::singleline(buf)
                .desired_width(96.0)
                .font(egui::TextStyle::Monospace),
        );
        (resp.changed(), buf.clone())
    };
    if changed {
        if buf_val.trim() == default.trim() {
            overrides.remove(&key);
        } else {
            overrides.insert(key.clone(), buf_val);
        }
    }
    let target = overrides.get(&key).cloned().unwrap_or_else(|| default.to_string());
    if target.trim() != current.trim() {
        ui.label(RichText::new("●").color(style::SUCCESS).small())
            .on_hover_text(format!("{current} → {target}"));
    }
}
