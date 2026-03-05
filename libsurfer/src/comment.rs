use egui::RichText;
use egui_remixicon::icons;

use crate::{SystemState, viewport::Viewport};

pub struct Comment {
    pub id: egui::Id,
    pub rect: egui::Rect,
    pub color: egui::Color32,
}

impl egui::Widget for Comment {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let button = egui::Button::new(RichText::new(icons::CHAT_1_FILL).heading()).frame(false);

        button.min_size(self.rect.size());

        return ui.interact(self.rect, self.id, egui::Sense::click_and_drag());
    }
}
