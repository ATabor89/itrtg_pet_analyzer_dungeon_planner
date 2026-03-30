use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, StrokeKind, Ui, Vec2};

use crate::log_parser::{self, DungeonLog};
use crate::style;

// =============================================================================
// State
// =============================================================================

/// Which sub-section of the log viewer is active.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum LogSection {
    #[default]
    Overview,
    RoomStats,
    Combat,
}

#[derive(Default)]
pub struct LogViewerState {
    /// The currently loaded dungeon log (if any).
    log: Option<DungeonLog>,
    /// The filename of the loaded log.
    filename: Option<String>,
    /// Error message from last load attempt.
    error: Option<String>,
    /// Active sub-section tab.
    section: LogSection,
    /// Which rooms are expanded in combat view.
    expanded_rooms: Vec<bool>,
    /// Which pet is selected for room stats chart (index into log.pets).
    selected_pet: usize,
}

impl LogViewerState {
    pub fn load_html(&mut self, html: &str, filename: Option<String>) {
        match log_parser::parse_dungeon_log(html) {
            Ok(log) => {
                let room_count = log.rooms.len().max(log.room_count as usize);
                self.expanded_rooms = vec![false; room_count];
                self.selected_pet = 0;
                self.log = Some(log);
                self.filename = filename;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
                self.log = None;
                self.filename = filename;
            }
        }
    }

}

// =============================================================================
// Colors for the log viewer
// =============================================================================

const CLASS_COLORS: &[(&str, Color32)] = &[
    ("Rogue", Color32::from_rgb(0xff, 0xdd, 0x55)),
    ("Supporter", Color32::from_rgb(0xff, 0xaa, 0xcc)),
    ("Mage", Color32::from_rgb(0xcc, 0x99, 0xff)),
    ("Defender", Color32::from_rgb(0x88, 0xbb, 0xff)),
    ("Assassin", Color32::from_rgb(0xff, 0x88, 0x88)),
    ("Adventurer", Color32::from_rgb(0xaa, 0xdd, 0xff)),
    ("Blacksmith", Color32::from_rgb(0xff, 0xbb, 0x77)),
    ("Alchemist", Color32::from_rgb(0xbb, 0xff, 0x99)),
    ("None", Color32::from_rgb(0x99, 0x99, 0xbb)),
];

fn class_color(class: &str) -> Color32 {
    CLASS_COLORS
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, c)| *c)
        .unwrap_or(style::TEXT_NORMAL)
}

// Distinct pet colors for charts
const PET_COLORS: &[Color32] = &[
    Color32::from_rgb(0xff, 0x88, 0x55), // orange
    Color32::from_rgb(0x55, 0xaa, 0xff), // blue
    Color32::from_rgb(0x55, 0xdd, 0x88), // green
    Color32::from_rgb(0xcc, 0x99, 0xff), // purple
    Color32::from_rgb(0xff, 0xdd, 0x55), // yellow
    Color32::from_rgb(0xff, 0x88, 0xbb), // pink
];

fn pet_color(idx: usize) -> Color32 {
    PET_COLORS[idx % PET_COLORS.len()]
}

// =============================================================================
// Main entry point
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut LogViewerState) {
    if state.log.is_none() {
        show_empty_state(ui, state);
        return;
    }

    // Extract header info before entering closures to avoid borrow conflicts.
    let header_text = {
        let log = state.log.as_ref().unwrap();
        format!("{} (Level {})", log.dungeon_name, log.dungeon_level)
    };
    let sub_text = {
        let log = state.log.as_ref().unwrap();
        format!("{} \u{2022} {} rooms", log.rating, log.room_count)
    };
    let fname_display = state.filename.clone();

    // Header area
    let mut clear_requested = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&header_text)
                .color(style::ACCENT)
                .size(18.0)
                .strong(),
        );

        ui.label(
            RichText::new(&sub_text)
                .color(style::TEXT_MUTED)
                .size(14.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("Clear Log").size(12.0))
                .clicked()
            {
                clear_requested = true;
            }

            if let Some(fname) = &fname_display {
                ui.label(
                    RichText::new(fname)
                        .color(style::TEXT_MUTED)
                        .size(11.0),
                );
            }
        });
    });

    if clear_requested {
        state.log = None;
        state.filename = None;
        state.error = None;
        return;
    }

    ui.add_space(4.0);

    // Section tabs
    ui.horizontal(|ui| {
        let sections = [
            (LogSection::Overview, "Overview"),
            (LogSection::RoomStats, "Room Stats"),
            (LogSection::Combat, "Combat Log"),
        ];
        for (sec, label) in sections {
            if ui
                .selectable_label(
                    state.section == sec,
                    RichText::new(label).size(13.0),
                )
                .clicked()
            {
                state.section = sec;
            }
        }
    });

    ui.separator();

    let section = state.section;
    let selected_pet = &mut state.selected_pet;
    let expanded_rooms = &mut state.expanded_rooms;
    let log = state.log.as_ref().unwrap();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| match section {
            LogSection::Overview => show_overview(ui, log),
            LogSection::RoomStats => show_room_stats(ui, log, selected_pet),
            LogSection::Combat => show_combat(ui, log, expanded_rooms),
        });
}

// =============================================================================
// Empty / Drop target state
// =============================================================================

fn show_empty_state(ui: &mut Ui, state: &LogViewerState) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);

        // Drop zone box
        let available = ui.available_size();
        let box_width = (available.x * 0.6).min(500.0);
        let box_height = 200.0;

        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(box_width, box_height),
            egui::Sense::hover(),
        );

        // Dashed border effect
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(2.0, style::TEXT_MUTED.linear_multiply(0.4)),
            StrokeKind::Inside,
        );

        // Icon and text
        let center = rect.center();
        ui.painter().text(
            center + Vec2::new(0.0, -30.0),
            egui::Align2::CENTER_CENTER,
            "\u{1F4C2}",
            egui::FontId::new(40.0, egui::FontFamily::Proportional),
            style::TEXT_MUTED,
        );
        ui.painter().text(
            center + Vec2::new(0.0, 15.0),
            egui::Align2::CENTER_CENTER,
            "Drop a dungeon log HTML file here",
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
            style::TEXT_NORMAL,
        );
        ui.painter().text(
            center + Vec2::new(0.0, 40.0),
            egui::Align2::CENTER_CENTER,
            "or click \"Open Log File\" above",
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
            style::TEXT_MUTED,
        );

        if let Some(err) = &state.error {
            ui.add_space(20.0);
            ui.label(RichText::new(err).color(style::ERROR).size(12.0));
        }
    });
}

// =============================================================================
// Overview section
// =============================================================================

fn show_overview(ui: &mut Ui, log: &DungeonLog) {
    // Pet cards
    ui.label(
        RichText::new("Party")
            .color(style::TEXT_BRIGHT)
            .size(15.0)
            .strong(),
    );
    ui.add_space(4.0);

    let totals = log_parser::compute_totals(log);

    // Pet cards in a grid
    let available_width = ui.available_width();
    let card_width = 200.0_f32;
    let cols = ((available_width / (card_width + 8.0)) as usize).max(1).min(6);

    egui::Grid::new("pet_cards_grid")
        .num_columns(cols)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (i, pet) in log.pets.iter().enumerate() {
                show_pet_card(ui, pet, &totals, card_width);
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(12.0);

    // Summary section
    egui::CollapsingHeader::new(
        RichText::new("Summary")
            .color(style::TEXT_BRIGHT)
            .size(14.0)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        // XP
        for (name, xp) in &log.summary.xp_gained {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).color(style::TEXT_NORMAL).size(12.0));
                ui.label(RichText::new(format!("+{xp} XP")).color(style::SUCCESS).size(12.0));
            });
        }

        if let Some((turns, room)) = log.summary.longest_turn {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Longest fight: {turns} turns (Room {room})"))
                    .color(style::TEXT_MUTED)
                    .size(12.0),
            );
        }

        if let Some(line) = &log.summary.leech_line {
            ui.label(RichText::new(line).color(style::TEXT_MUTED).size(11.0));
        }
        if let Some(line) = &log.summary.free_exp_line {
            ui.label(RichText::new(line).color(style::TEXT_MUTED).size(11.0));
        }
    });

    ui.add_space(8.0);

    // Events
    if !log.events.is_empty() || !log.deaths.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new("Events & Deaths")
                .color(style::TEXT_BRIGHT)
                .size(14.0)
                .strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            for event in &log.events {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Room {}", event.room))
                            .color(style::ACCENT)
                            .size(12.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(&event.event_type)
                            .color(style::TEXT_NORMAL)
                            .size(12.0),
                    );

                    let success = event.outcome.contains("success");
                    let (icon, color) = if success {
                        ("\u{2713}", style::SUCCESS)
                    } else {
                        ("\u{2717}", style::ERROR)
                    };
                    ui.label(RichText::new(icon).color(color).size(12.0));
                    ui.label(RichText::new(&event.outcome).color(color).size(12.0));

                    if let Some(detail) = &event.detail {
                        ui.label(
                            RichText::new(format!("({detail})"))
                                .color(style::TEXT_MUTED)
                                .size(11.0),
                        );
                    }
                });
            }

            if !log.deaths.is_empty() {
                ui.add_space(4.0);
                for death in &log.deaths {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("\u{2620}").color(style::ERROR).size(12.0));
                        ui.label(
                            RichText::new(&death.pet_name)
                                .color(style::ERROR)
                                .size(12.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "killed by {} (Room {}, Turn {})",
                                death.killed_by, death.room, death.turn
                            ))
                            .color(style::TEXT_MUTED)
                            .size(12.0),
                        );
                    });
                }
            }
        });

        ui.add_space(8.0);
    }

    // Items
    let has_items = !log.items_used.is_empty()
        || !log.items_found.is_empty()
        || !log.items_from_events.is_empty();
    if has_items {
        egui::CollapsingHeader::new(
            RichText::new("Items")
                .color(style::TEXT_BRIGHT)
                .size(14.0)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            if !log.items_used.is_empty() {
                ui.label(
                    RichText::new("Used")
                        .color(style::WARNING)
                        .size(12.0)
                        .strong(),
                );
                for item in &log.items_used {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}x", item.count))
                                .color(style::TEXT_MUTED)
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(&item.name)
                                .color(style::TEXT_NORMAL)
                                .size(12.0),
                        );
                        if let Some(detail) = &item.detail {
                            ui.label(
                                RichText::new(detail)
                                    .color(style::TEXT_MUTED)
                                    .size(11.0),
                            );
                        }
                    });
                }
                ui.add_space(4.0);
            }

            if !log.items_found.is_empty() {
                ui.label(
                    RichText::new("Found")
                        .color(style::SUCCESS)
                        .size(12.0)
                        .strong(),
                );
                for item in &log.items_found {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}x", item.count))
                                .color(style::TEXT_MUTED)
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(&item.name)
                                .color(style::TEXT_NORMAL)
                                .size(12.0),
                        );
                    });
                }
                ui.add_space(4.0);
            }

            if !log.items_from_events.is_empty() {
                ui.label(
                    RichText::new("From Events")
                        .color(style::ACCENT)
                        .size(12.0)
                        .strong(),
                );
                for item in &log.items_from_events {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}x", item.count))
                                .color(style::TEXT_MUTED)
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(&item.name)
                                .color(style::TEXT_NORMAL)
                                .size(12.0),
                        );
                    });
                }
            }
        });

        ui.add_space(8.0);
    }

    // Depth stats
    if !log.depth_stats.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new("Depth Stats")
                .color(style::TEXT_BRIGHT)
                .size(14.0)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            for (i, pet_depth) in log.depth_stats.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&pet_depth.pet_name)
                            .color(pet_color(i))
                            .size(12.0)
                            .strong(),
                    );
                });
                for d in &pet_depth.depths {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(format!("Depth {}", d.depth))
                                .color(style::TEXT_MUTED)
                                .size(11.0),
                        );
                        ui.label(
                            RichText::new(format!("{} dmg dealt", d.damage_done))
                                .color(style::TEXT_NORMAL)
                                .size(11.0),
                        );
                        ui.label(
                            RichText::new(format!("{} dmg taken", d.damage_taken))
                                .color(if d.damage_taken == "0" {
                                    style::TEXT_MUTED
                                } else {
                                    style::WARNING
                                })
                                .size(11.0),
                        );
                        if let Some(healed) = &d.healed {
                            ui.label(
                                RichText::new(format!("{healed} healed"))
                                    .color(style::SUCCESS)
                                    .size(11.0),
                            );
                        }
                    });
                }
                ui.add_space(2.0);
            }
        });
    }
}

// =============================================================================
// Pet card widget
// =============================================================================

fn show_pet_card(
    ui: &mut Ui,
    pet: &log_parser::PetInfo,
    totals: &std::collections::HashMap<String, (u64, u64, u64)>,
    width: f32,
) {
    let color = class_color(&pet.class);

    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, color.linear_multiply(0.3)))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(width - 20.0);

            // Name + class
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&pet.name)
                        .color(style::TEXT_BRIGHT)
                        .size(13.0)
                        .strong(),
                );
                ui.label(RichText::new(&pet.class).color(color).size(11.0));
            });

            // Stats
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Growth {}", pet.growth))
                        .color(style::TEXT_MUTED)
                        .size(11.0),
                );
                ui.label(
                    RichText::new(format!("Lv{}", pet.level))
                        .color(style::TEXT_NORMAL)
                        .size(11.0),
                );
                ui.label(
                    RichText::new(format!("CLv{}", pet.class_level))
                        .color(style::TEXT_MUTED)
                        .size(11.0),
                );
            });

            // Total damage if available
            if let Some(&(done, taken, healed)) = totals.get(&pet.name) {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format_number(done))
                            .color(style::TEXT_NORMAL)
                            .size(11.0),
                    );
                    ui.label(
                        RichText::new("dmg")
                            .color(style::TEXT_MUTED)
                            .size(10.0),
                    );
                    if taken > 0 {
                        ui.label(
                            RichText::new(format_number(taken))
                                .color(style::WARNING)
                                .size(11.0),
                        );
                        ui.label(
                            RichText::new("taken")
                                .color(style::TEXT_MUTED)
                                .size(10.0),
                        );
                    }
                    if healed > 0 {
                        ui.label(
                            RichText::new(format_number(healed))
                                .color(style::SUCCESS)
                                .size(11.0),
                        );
                        ui.label(
                            RichText::new("healed")
                                .color(style::TEXT_MUTED)
                                .size(10.0),
                        );
                    }
                });
            }
        });
}

// =============================================================================
// Room Stats section
// =============================================================================

fn show_room_stats(ui: &mut Ui, log: &DungeonLog, selected_pet: &mut usize) {
    if log.room_stats.is_empty() {
        ui.label(
            RichText::new("No room stats available in this log.")
                .color(style::TEXT_MUTED)
                .size(13.0),
        );
        return;
    }

    // Pet selector
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Pet:")
                .color(style::TEXT_MUTED)
                .size(13.0),
        );
        for (i, pet_rooms) in log.room_stats.iter().enumerate() {
            if ui
                .selectable_label(
                    *selected_pet == i,
                    RichText::new(&pet_rooms.pet_name)
                        .color(pet_color(i))
                        .size(13.0),
                )
                .clicked()
            {
                *selected_pet = i;
            }
        }
    });

    ui.add_space(8.0);

    if *selected_pet >= log.room_stats.len() {
        *selected_pet = 0;
    }

    let pet_rooms = &log.room_stats[*selected_pet];

    // Bar chart of damage done per room
    show_room_bar_chart(ui, pet_rooms, *selected_pet);

    ui.add_space(12.0);

    // Detailed table
    egui::Grid::new("room_stats_table")
        .num_columns(5)
        .spacing([12.0, 3.0])
        .striped(true)
        .show(ui, |ui| {
            // Header
            ui.label(RichText::new("Room").color(style::TEXT_MUTED).size(11.0).strong());
            ui.label(RichText::new("Dmg Done").color(style::TEXT_MUTED).size(11.0).strong());
            ui.label(RichText::new("Dmg Taken").color(style::TEXT_MUTED).size(11.0).strong());
            ui.label(RichText::new("Healed").color(style::TEXT_MUTED).size(11.0).strong());
            ui.label(RichText::new("Net").color(style::TEXT_MUTED).size(11.0).strong());
            ui.end_row();

            for r in &pet_rooms.rooms {
                let done = r.damage_done.replace(',', "").parse::<i64>().unwrap_or(0);
                let taken = r.damage_taken.replace(',', "").parse::<i64>().unwrap_or(0);
                let healed = r
                    .healed
                    .as_ref()
                    .map(|h| h.replace(',', "").parse::<i64>().unwrap_or(0))
                    .unwrap_or(0);
                let net = done - taken + healed;

                ui.label(
                    RichText::new(format!("{}", r.room))
                        .color(style::ACCENT)
                        .size(11.0),
                );
                ui.label(
                    RichText::new(&r.damage_done)
                        .color(style::TEXT_NORMAL)
                        .size(11.0),
                );
                ui.label(
                    RichText::new(&r.damage_taken)
                        .color(if taken > 0 { style::WARNING } else { style::TEXT_MUTED })
                        .size(11.0),
                );
                ui.label(
                    RichText::new(r.healed.as_deref().unwrap_or("-"))
                        .color(if healed > 0 { style::SUCCESS } else { style::TEXT_MUTED })
                        .size(11.0),
                );
                ui.label(
                    RichText::new(format_number_signed(net))
                        .color(if net >= 0 { style::SUCCESS } else { style::ERROR })
                        .size(11.0),
                );
                ui.end_row();
            }
        });
}

fn show_room_bar_chart(ui: &mut Ui, pet_rooms: &log_parser::PetRoomStats, pet_idx: usize) {
    let values: Vec<f32> = pet_rooms
        .rooms
        .iter()
        .map(|r| r.damage_done.replace(',', "").parse::<f32>().unwrap_or(0.0))
        .collect();
    let max_val = values.iter().copied().fold(0.0f32, f32::max).max(1.0);

    let bar_color = pet_color(pet_idx);
    let chart_height = 120.0;
    let available_width = ui.available_width().min(800.0);
    let bar_width = ((available_width - 40.0) / values.len() as f32).min(30.0).max(8.0);
    let total_width = bar_width * values.len() as f32 + 40.0;

    // Y-axis label
    ui.label(
        RichText::new("Damage Done per Room")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );

    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(total_width, chart_height + 20.0),
        egui::Sense::hover(),
    );

    let chart_left = rect.left() + 40.0;
    let chart_bottom = rect.bottom() - 18.0;
    let chart_top = rect.top();

    // Y axis
    ui.painter().line_segment(
        [
            egui::pos2(chart_left, chart_top),
            egui::pos2(chart_left, chart_bottom),
        ],
        Stroke::new(1.0, style::TEXT_MUTED.linear_multiply(0.3)),
    );

    // Max value label
    ui.painter().text(
        egui::pos2(chart_left - 4.0, chart_top),
        egui::Align2::RIGHT_TOP,
        format_number(max_val as u64),
        egui::FontId::new(9.0, egui::FontFamily::Proportional),
        style::TEXT_MUTED,
    );

    // Bars
    for (i, &val) in values.iter().enumerate() {
        let x = chart_left + i as f32 * bar_width + 2.0;
        let height = (val / max_val) * (chart_bottom - chart_top - 4.0);
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(x, chart_bottom - height),
            Vec2::new(bar_width - 4.0, height),
        );
        ui.painter()
            .rect_filled(bar_rect, CornerRadius::same(2), bar_color.linear_multiply(0.7));

        // Room number label
        ui.painter().text(
            egui::pos2(x + (bar_width - 4.0) / 2.0, chart_bottom + 2.0),
            egui::Align2::CENTER_TOP,
            format!("{}", pet_rooms.rooms.get(i).map(|r| r.room).unwrap_or(0)),
            egui::FontId::new(9.0, egui::FontFamily::Proportional),
            style::TEXT_MUTED,
        );
    }
}

// =============================================================================
// Combat log section
// =============================================================================

fn show_combat(ui: &mut Ui, log: &DungeonLog, expanded_rooms: &mut Vec<bool>) {
    if log.rooms.is_empty() {
        ui.label(
            RichText::new("No detailed combat logs available in this log.")
                .color(style::TEXT_MUTED)
                .size(13.0),
        );
        return;
    }

    // Expand / collapse all
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Expand All").size(11.0))
            .clicked()
        {
            for e in expanded_rooms.iter_mut() {
                *e = true;
            }
        }
        if ui
            .button(RichText::new("Collapse All").size(11.0))
            .clicked()
        {
            for e in expanded_rooms.iter_mut() {
                *e = false;
            }
        }
    });

    ui.add_space(4.0);

    for room in &log.rooms {
        let idx = room.room_number.saturating_sub(1) as usize;
        // Ensure expanded_rooms is big enough
        while expanded_rooms.len() <= idx {
            expanded_rooms.push(false);
        }

        let expanded = &mut expanded_rooms[idx];
        let header_text = format!(
            "Room {} \u{2022} {} monsters \u{2022} {} turns",
            room.room_number,
            room.monsters.len(),
            room.turns.len(),
        );

        let toggle = ui
            .horizontal(|ui| {
                let arrow = if *expanded { "\u{25BC}" } else { "\u{25B6}" };
                let resp = ui.selectable_label(
                    false,
                    RichText::new(format!("{arrow} {header_text}"))
                        .color(style::ACCENT)
                        .size(13.0)
                        .strong(),
                );
                resp
            })
            .inner;

        if toggle.clicked() {
            *expanded = !*expanded;
        }

        if *expanded {
            egui::Frame::new()
                .fill(style::BG_SURFACE)
                .corner_radius(CornerRadius::same(4))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Pet HP
                    if !room.pet_hp.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Pets:")
                                    .color(style::TEXT_MUTED)
                                    .size(11.0),
                            );
                            for (name, hp) in &room.pet_hp {
                                ui.label(
                                    RichText::new(format!("{name} {hp}HP"))
                                        .color(style::TEXT_NORMAL)
                                        .size(11.0),
                                );
                            }
                        });
                    }

                    // Monsters
                    if !room.monsters.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Monsters:")
                                    .color(style::TEXT_MUTED)
                                    .size(11.0),
                            );
                            for (name, hp) in &room.monsters {
                                ui.label(
                                    RichText::new(format!("{name} {hp}HP"))
                                        .color(style::ERROR.linear_multiply(0.8))
                                        .size(11.0),
                                );
                            }
                        });
                    }

                    // Traps
                    if room.traps != "None" {
                        ui.label(
                            RichText::new(format!("Traps: {}", room.traps))
                                .color(style::WARNING)
                                .size(11.0),
                        );
                    }

                    // Notes
                    for note in &room.notes {
                        ui.label(
                            RichText::new(note)
                                .color(style::TEXT_MUTED)
                                .size(11.0)
                                .italics(),
                        );
                    }

                    ui.add_space(4.0);

                    // Turns
                    for turn in &room.turns {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Turn {}", turn.turn_number))
                                    .color(style::ACCENT)
                                    .size(11.0)
                                    .strong(),
                            );
                            if let Some(hp) = &turn.pet_hp_line {
                                // Extract just the HP values
                                let hp_display = hp
                                    .split(": ")
                                    .nth(1)
                                    .unwrap_or(hp);
                                ui.label(
                                    RichText::new(hp_display)
                                        .color(style::TEXT_MUTED)
                                        .size(10.0),
                                );
                            }
                        });

                        for action in &turn.actions {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(16.0);
                                // Color based on action content
                                let color = if action.contains("died!") {
                                    style::ERROR
                                } else if action.contains("healed") {
                                    style::SUCCESS
                                } else if action.contains("summoned") {
                                    style::WARNING
                                } else if action.contains("shielded") || action.contains("defender skill") {
                                    Color32::from_rgb(0x88, 0xbb, 0xff)
                                } else {
                                    style::TEXT_NORMAL
                                };
                                ui.label(RichText::new(action).color(color).size(10.0));
                            });
                        }
                    }

                    // Drops
                    if !room.drops.is_empty() {
                        ui.add_space(4.0);
                        for drop_line in &room.drops {
                            ui.label(
                                RichText::new(drop_line)
                                    .color(style::TEXT_MUTED)
                                    .size(10.0),
                            );
                        }
                    }
                });

            ui.add_space(2.0);
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_number_signed(n: i64) -> String {
    let sign = if n >= 0 { "+" } else { "" };
    let abs = n.unsigned_abs();
    format!("{sign}{}", format_number(abs))
}
