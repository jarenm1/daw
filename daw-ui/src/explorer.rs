use eframe::egui;

use crate::theme;

const EXPLORER_HEADER_HEIGHT: f32 = 38.0;

pub struct ExplorerView {
    sections: Vec<ExplorerSection>,
}

impl ExplorerView {
    pub fn new() -> Self {
        Self {
            sections: vec![
                ExplorerSection::new(
                    "Libraries",
                    vec![
                        ExplorerItem::status("Drumkits Folder", "Not set", ItemState::Pending),
                        ExplorerItem::status("VST Search Paths", "Not set", ItemState::Pending),
                        ExplorerItem::status("Samples Library", "Included", ItemState::Ready),
                    ],
                ),
                ExplorerSection::new(
                    "Instruments",
                    vec![
                        ExplorerItem::status("Built-in Devices", "3 available", ItemState::Ready),
                        ExplorerItem::status("VST Plugins", "Scan later", ItemState::Muted),
                        ExplorerItem::status("Generators", "Coming soon", ItemState::Muted),
                    ],
                ),
                ExplorerSection::new(
                    "Sounds",
                    vec![
                        ExplorerItem::status("Included Kits", "Factory", ItemState::Ready),
                        ExplorerItem::status("One-shots", "Factory", ItemState::Ready),
                        ExplorerItem::status("Loops", "Factory", ItemState::Ready),
                    ],
                ),
            ],
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::default()
            .fill(theme::SURFACE_0)
            .corner_radius(egui::CornerRadius::ZERO)
            .inner_margin(egui::Margin::ZERO);

        frame.show(ui, |ui| {
            self.show_header(ui);
            self.show_sections(ui);
        });
    }

    fn show_header(&self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, EXPLORER_HEADER_HEIGHT),
            egui::Sense::hover(),
        );
        let painter = ui.painter();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        painter.text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Explorer",
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );
    }

    fn show_sections(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                for section in &self.sections {
                    draw_section(ui, section);
                }
            });
    }
}

impl Default for ExplorerView {
    fn default() -> Self {
        Self::new()
    }
}

struct ExplorerSection {
    title: &'static str,
    items: Vec<ExplorerItem>,
}

impl ExplorerSection {
    fn new(title: &'static str, items: Vec<ExplorerItem>) -> Self {
        Self { title, items }
    }
}

struct ExplorerItem {
    label: &'static str,
    meta: &'static str,
    state: ItemState,
}

impl ExplorerItem {
    fn status(label: &'static str, meta: &'static str, state: ItemState) -> Self {
        Self { label, meta, state }
    }
}

#[derive(Clone, Copy)]
enum ItemState {
    Ready,
    Pending,
    Muted,
}

impl ItemState {
    fn color(self) -> egui::Color32 {
        match self {
            Self::Ready => theme::ACCENT,
            Self::Pending => theme::TEXT_MUTED,
            Self::Muted => theme::GRID_MAJOR,
        }
    }
}

fn draw_section(ui: &mut egui::Ui, section: &ExplorerSection) {
    let width = ui.available_width();
    let (header_rect, _) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(header_rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
    painter.line_segment(
        [header_rect.left_bottom(), header_rect.right_bottom()],
        egui::Stroke::new(1.0, theme::GRID_ROW),
    );
    painter.text(
        egui::pos2(header_rect.left() + 12.0, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        section.title,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );

    for item in &section.items {
        draw_item(ui, item);
    }
}

fn draw_item(ui: &mut egui::Ui, item: &ExplorerItem) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 52.0), egui::Sense::click());
    let painter = ui.painter();

    let fill = if response.hovered() {
        theme::SURFACE_1
    } else {
        theme::SURFACE_0
    };

    painter.rect_filled(rect, egui::CornerRadius::ZERO, fill);
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, theme::GRID_ROW),
    );

    let marker_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 10.0, rect.top() + 18.0),
        egui::pos2(rect.left() + 14.0, rect.bottom() - 18.0),
    );
    painter.rect_filled(marker_rect, egui::CornerRadius::ZERO, item.state.color());

    painter.text(
        egui::pos2(rect.left() + 24.0, rect.top() + 18.0),
        egui::Align2::LEFT_CENTER,
        item.label,
        egui::FontId::proportional(12.0),
        theme::TEXT,
    );
    painter.text(
        egui::pos2(rect.left() + 24.0, rect.top() + 36.0),
        egui::Align2::LEFT_CENTER,
        item.meta,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
}
