use eframe::egui::{self, Color32, FontId, Stroke, TextStyle, Visuals};
use itrtg_models::Element;

// =============================================================================
// Color palette
// =============================================================================

/// Deep background.
pub const BG_DEEP: Color32 = Color32::from_rgb(0x08, 0x08, 0x0e);
/// Panel background.
pub const BG_PANEL: Color32 = Color32::from_rgb(0x10, 0x10, 0x1a);
/// Slightly lighter surface for cards/rows.
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x18, 0x18, 0x24);
/// Row hover.
pub const BG_HOVER: Color32 = Color32::from_rgb(0x22, 0x22, 0x32);
/// Primary accent (purple).
pub const ACCENT: Color32 = Color32::from_rgb(0xc9, 0xa0, 0xff);
/// Muted text.
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x88, 0x88, 0xaa);
/// Normal text.
pub const TEXT_NORMAL: Color32 = Color32::from_rgb(0xcc, 0xcc, 0xdd);
/// Bright text.
pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(0xee, 0xee, 0xf4);
/// Success green.
pub const SUCCESS: Color32 = Color32::from_rgb(0x55, 0xdd, 0x88);
/// Warning amber.
pub const WARNING: Color32 = Color32::from_rgb(0xdd, 0xaa, 0x44);
/// Error red.
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x66, 0x55);

// Element colors
pub const ELEM_FIRE: Color32 = Color32::from_rgb(0xff, 0x88, 0x55);
pub const ELEM_WATER: Color32 = Color32::from_rgb(0x55, 0xaa, 0xff);
pub const ELEM_WIND: Color32 = Color32::from_rgb(0x55, 0xdd, 0x88);
pub const ELEM_EARTH: Color32 = Color32::from_rgb(0xcc, 0xaa, 0x55);
pub const ELEM_NEUTRAL: Color32 = Color32::from_rgb(0x99, 0x99, 0xbb);
pub const ELEM_ALL: Color32 = Color32::from_rgb(0xdd, 0x88, 0xdd);

/// Element background colors (darker, for badges).
pub const ELEM_FIRE_BG: Color32 = Color32::from_rgb(0x2e, 0x1a, 0x1a);
pub const ELEM_WATER_BG: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x2e);
pub const ELEM_WIND_BG: Color32 = Color32::from_rgb(0x1a, 0x2e, 0x1a);
pub const ELEM_EARTH_BG: Color32 = Color32::from_rgb(0x2e, 0x2a, 0x1a);
pub const ELEM_NEUTRAL_BG: Color32 = Color32::from_rgb(0x22, 0x22, 0x2e);
pub const ELEM_ALL_BG: Color32 = Color32::from_rgb(0x2a, 0x1a, 0x2e);

pub fn element_color(element: &Element) -> Color32 {
    match element {
        Element::Fire => ELEM_FIRE,
        Element::Water => ELEM_WATER,
        Element::Wind => ELEM_WIND,
        Element::Earth => ELEM_EARTH,
        Element::Neutral => ELEM_NEUTRAL,
        Element::All => ELEM_ALL,
    }
}

pub fn element_bg(element: &Element) -> Color32 {
    match element {
        Element::Fire => ELEM_FIRE_BG,
        Element::Water => ELEM_WATER_BG,
        Element::Wind => ELEM_WIND_BG,
        Element::Earth => ELEM_EARTH_BG,
        Element::Neutral => ELEM_NEUTRAL_BG,
        Element::All => ELEM_ALL_BG,
    }
}

// =============================================================================
// Theme setup
// =============================================================================

pub fn configure_style(ctx: &egui::Context) {
    // Load a system symbol font as fallback so ✓ ✗ ◆ etc. render properly.
    // egui's built-in font subset doesn't include these glyphs.
    let mut fonts = egui::FontDefinitions::default();
    let symbol_paths = [
        "C:\\Windows\\Fonts\\seguisym.ttf",  // Segoe UI Symbol (Windows)
        "C:\\Windows\\Fonts\\segmdl2.ttf",    // Segoe MDL2 Assets (fallback)
    ];
    for path in &symbol_paths {
        if let Ok(font_data) = std::fs::read(path) {
            let name = "system_symbols".to_owned();
            fonts.font_data.insert(
                name.clone(),
                egui::FontData::from_owned(font_data).into(),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push(name.clone());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.push(name);
            }
            break; // Use the first available font
        }
    }
    ctx.set_fonts(fonts);

    let mut visuals = Visuals::dark();

    visuals.panel_fill = BG_DEEP;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_SURFACE;

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_NORMAL);

    visuals.widgets.inactive.bg_fill = BG_SURFACE;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_NORMAL);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_BRIGHT);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0x30, 0x28, 0x48);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);

    visuals.selection.bg_fill = Color32::from_rgb(0x3a, 0x2a, 0x55);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    ctx.set_style(style);
}
