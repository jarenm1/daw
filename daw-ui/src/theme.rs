use eframe::egui;

pub const APP_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(10, 10, 11);
pub const SURFACE_0: egui::Color32 = egui::Color32::from_rgb(16, 16, 18);
pub const SURFACE_1: egui::Color32 = egui::Color32::from_rgb(24, 24, 27);
pub const SURFACE_2: egui::Color32 = egui::Color32::from_rgb(34, 34, 38);
pub const SURFACE_3: egui::Color32 = egui::Color32::from_rgb(48, 48, 54);

pub const TEXT: egui::Color32 = egui::Color32::from_rgb(233, 236, 240);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(145, 149, 156);

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(111, 184, 255);
pub const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(137, 197, 255);
pub const ACCENT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(88, 170, 249);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(58, 58, 64);
pub const TOOL_ACTIVE_WASH: egui::Color32 = egui::Color32::from_rgb(18, 23, 29);
pub const GRID_MINOR: egui::Color32 = egui::Color32::from_rgb(28, 28, 31);
pub const GRID_MAJOR: egui::Color32 = egui::Color32::from_rgb(45, 45, 50);
pub const GRID_ROW: egui::Color32 = egui::Color32::from_rgb(38, 38, 43);

pub fn accent_soft() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(111, 184, 255, 36)
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = APP_BACKGROUND;
    visuals.window_fill = SURFACE_0;
    visuals.extreme_bg_color = SURFACE_2;
    visuals.faint_bg_color = SURFACE_1;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = accent_soft();
    visuals.selection.stroke.color = TEXT;
    visuals.window_stroke.color = BORDER;
    visuals.widgets.noninteractive.bg_fill = SURFACE_0;
    visuals.widgets.noninteractive.bg_stroke.color = BORDER;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_MUTED;
    visuals.widgets.inactive.bg_fill = SURFACE_1;
    visuals.widgets.inactive.bg_stroke.color = BORDER;
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.hovered.bg_fill = SURFACE_2;
    visuals.widgets.hovered.bg_stroke.color = ACCENT;
    visuals.widgets.hovered.fg_stroke.color = TEXT;
    visuals.widgets.active.bg_fill = SURFACE_3;
    visuals.widgets.active.bg_stroke.color = ACCENT_ACTIVE;
    visuals.widgets.active.fg_stroke.color = TEXT;
    visuals.widgets.open.bg_fill = SURFACE_2;
    visuals.widgets.open.bg_stroke.color = ACCENT_ACTIVE;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    style.spacing.indent = 10.0;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT_MUTED;
    style.visuals.widgets.hovered.weak_bg_fill = accent_soft();
    style.visuals.widgets.active.weak_bg_fill = accent_soft();
    style.visuals.selection.bg_fill = accent_soft();
    style.visuals.selection.stroke.color = ACCENT_HOVER;
    ctx.set_style(style);
}
