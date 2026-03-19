use eframe::egui::{self, Color32, RichText, CornerRadius, StrokeKind, Stroke, Ui, Vec2};
use itrtg_models::{Class, Element, RecommendedClass};

use crate::style;

/// Render a colored element badge.
pub fn element_badge(ui: &mut Ui, element: &Element) {
    let text = match element {
        Element::Fire => "Fire",
        Element::Water => "Water",
        Element::Wind => "Wind",
        Element::Earth => "Earth",
        Element::Neutral => "Neutral",
        Element::All => "All",
    };
    let fg = style::element_color(element);
    let bg = style::element_bg(element);

    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
        fg,
    );
    let desired = Vec2::new(galley.size().x + 12.0, galley.size().y + 4.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());

    ui.painter().rect_filled(rect, CornerRadius::same(3), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, fg.linear_multiply(0.4)),
        StrokeKind::Outside,
    );
    let text_pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(text_pos, galley, fg);
}

/// Render a class as text with appropriate styling.
pub fn class_label(ui: &mut Ui, class: &Class) {
    let (text, color) = match class {
        Class::Adventurer => ("Adventurer", Color32::from_rgb(0xaa, 0xdd, 0xff)),
        Class::Blacksmith => ("Blacksmith", Color32::from_rgb(0xff, 0xbb, 0x77)),
        Class::Alchemist => ("Alchemist", Color32::from_rgb(0xbb, 0xff, 0x99)),
        Class::Defender => ("Defender", Color32::from_rgb(0x88, 0xbb, 0xff)),
        Class::Supporter => ("Supporter", Color32::from_rgb(0xff, 0xaa, 0xcc)),
        Class::Rogue => ("Rogue", Color32::from_rgb(0xff, 0xdd, 0x55)),
        Class::Assassin => ("Assassin", Color32::from_rgb(0xff, 0x88, 0x88)),
        Class::Mage => ("Mage", Color32::from_rgb(0xcc, 0x99, 0xff)),
        Class::Wildcard => ("Wildcard", style::TEXT_MUTED),
    };
    ui.label(RichText::new(text).color(color).size(12.0));
}

/// Render a recommended class with appropriate formatting.
pub fn recommended_class_label(ui: &mut Ui, rec: &RecommendedClass) {
    match rec {
        RecommendedClass::Single(c) => class_label(ui, c),
        RecommendedClass::Dual(a, b) => {
            class_label(ui, a);
            ui.label(RichText::new("/").color(style::TEXT_MUTED).size(12.0));
            class_label(ui, b);
        }
        RecommendedClass::Wildcard => {
            ui.label(RichText::new("Wildcard").color(style::TEXT_MUTED).size(12.0));
        }
        RecommendedClass::DungeonWildcard => {
            ui.label(RichText::new("Dng Wildcard").color(style::TEXT_MUTED).size(12.0));
        }
        RecommendedClass::Village(role) => {
            ui.label(
                RichText::new(format!("Village ({role})"))
                    .color(Color32::from_rgb(0x99, 0xcc, 0x99))
                    .size(12.0),
            );
        }
        RecommendedClass::AllClasses => {
            ui.label(
                RichText::new("All Classes")
                    .color(style::ACCENT)
                    .size(12.0),
            );
        }
        RecommendedClass::Special => {
            ui.label(
                RichText::new("Special")
                    .color(style::WARNING)
                    .size(12.0),
            );
        }
        RecommendedClass::Alternates => {
            ui.label(
                RichText::new("Alternates")
                    .color(style::WARNING)
                    .size(12.0),
            );
        }
    }
}

/// Status indicator dot.
pub fn status_dot(ui: &mut Ui, active: bool) {
    let color = if active { style::SUCCESS } else { Color32::from_rgb(0x44, 0x44, 0x55) };
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}
