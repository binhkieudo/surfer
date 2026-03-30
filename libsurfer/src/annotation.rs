use egui::{
    Align, Area, Button, Color32, Frame, Id, Layout, Order, Pos2, RichText, Stroke, UiBuilder, debug_text::print
};
use egui_remixicon::icons;
use num::BigInt;

use crate::{
    SystemState, arrow::ArrowAnnotation, config::SurferTheme, message::Message,
    rectangle::RectAnnotation, system_state, view::DrawingContext, viewport::Viewport,
    wave_data::WaveData,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Annotation {
    Arrow(ArrowAnnotation),
    Rect(RectAnnotation),
}

impl Annotation {
    pub fn draw_quick_menu(
        &self,
        annotation_id: Id,
        position: Pos2,
        ui: &mut egui::Ui,
        msgs: &mut Vec<Message>,
    ) {
        let menu_rect = egui::Rect::from_min_size(position, egui::vec2(90.0, 28.0));

        ui.scope_builder(egui::UiBuilder::new().max_rect(menu_rect), |ui| {
            Frame::popup(ui.style())
                .fill(ui.visuals().extreme_bg_color)
                .stroke(Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);

                    ui.horizontal(|ui| {
                        if ui.button(icons::SEARCH_LINE).clicked() {
                            msgs.push(Message::GoToAnnotationPosition(annotation_id, 0));
                        }

                        let vis_icon = if !self.is_visible() {
                            icons::EYE_OFF_LINE
                        } else {
                            icons::EYE_LINE
                        };

                        if ui.button(vis_icon).clicked() {
                            msgs.push(Message::ToggleAnnotationVisiblility(annotation_id));
                        }

                        if ui.button(icons::DELETE_BIN_LINE).clicked() {
                            msgs.push(Message::RemoveAnnotation(annotation_id));
                        }
                    });
                });
        });
    }

    pub fn draw_annotation_quick_menu(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut DrawingContext,
        msgs: &mut Vec<Message>,
        viewport: &Viewport,
        waves: &WaveData,
    ) {
        match self {
            Annotation::Arrow(arrow) => {
                let mut arrow_pos = arrow.get_pos(waves, viewport, ctx, 10.0).unwrap();
                arrow_pos.x -= 20.0; // Justera positionen för att inte överlappa pilen
                self.draw_quick_menu(arrow.id, arrow_pos, ui, msgs);
            }
            Annotation::Rect(rect) => {
                let rect_pos = rect.get_pos(waves, viewport, ctx, -20.0).unwrap();
                self.draw_quick_menu(rect.id, rect_pos, ui, msgs);
            }
        }
    }

    pub fn clicked(id: Id, waves: &mut WaveData) {
        for annotation in waves.annotations.iter_mut() {
            match annotation {
                Annotation::Arrow(arrow) => {
                    if arrow.id == id {
                        arrow.open_quick_menu = !arrow.open_quick_menu;
                    }
                }
                Annotation::Rect(rect) => {
                    if rect.id == id {
                        rect.open_quick_menu = !rect.open_quick_menu;
                    }
                }
            }
        }
    }
    pub fn get_type(&self) -> i32 {
        match self {
            Annotation::Arrow(_) => 0,
            Annotation::Rect(_) => 1,
        }
    }

    pub fn get_id(&self) -> egui::Id {
        match self {
            Annotation::Arrow(a) => a.id,
            Annotation::Rect(r) => r.id,
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            Annotation::Arrow(a) => a.name.clone(),
            Annotation::Rect(r) => r.name.clone(),
        }
    }

    pub fn group_name(&self) -> Option<String> {
        match self {
            Annotation::Arrow(a) => a.group_name.clone(),
            Annotation::Rect(r) => r.group_name.clone(),
        }
    }

    pub fn is_visible(&self) -> bool {
        match self {
            Annotation::Arrow(a) => a.visible,
            Annotation::Rect(r) => r.visible,
        }
    }

    pub fn toggle_visibility(&mut self) {
        match self {
            Annotation::Arrow(a) => a.toggle_arrow_visibility(),
            Annotation::Rect(r) => r.toggle_rectangle_visiblility(),
        }
    }

    pub fn set_visibility(&mut self, visible: bool){
        match self {
            Annotation::Arrow(a) => a.visible = visible,
            Annotation::Rect(r) => r.visible = visible,
        }
    }

    pub fn set_group_name(&mut self, name: Option<String>) {
        match self {
            Annotation::Arrow(a) => a.group_name = name,
            Annotation::Rect(r) => r.group_name = name,
        }
    }

    pub fn set_name(&mut self, name: String) {
        match self {
            Annotation::Arrow(a) => a.name = name,
            Annotation::Rect(r) => r.name = name,
        }
    }

    pub fn get_time_at_start(&self) -> BigInt {
        match self {
            Annotation::Arrow(a) => a.get_time_at_start(),
            Annotation::Rect(r) => r.get_time_at_start(),
        }
    }

    pub fn open_quick_menu(&self) -> bool {
        match self {
            Annotation::Arrow(a) => a.open_quick_menu,
            Annotation::Rect(r) => r.open_quick_menu,
        }
    }
}

impl WaveData {
    pub fn delete_annotation(&mut self, id: egui::Id) {
        self.annotations
            .retain(|annotation| annotation.get_id() != id);
    }

    pub fn draw_annotations(
        &self,
        ui: &mut egui::Ui,
        viewport: &Viewport,
        viewport_idx: usize,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
        y_offset: f32,
        msgs: &mut Vec<Message>,
    ) {
        for annotation in &self.annotations {
            match annotation {
                Annotation::Arrow(arrow) => {
                    self.draw_arrow(arrow, ui, *viewport, viewport_idx, ctx, theme, msgs);
                }
                Annotation::Rect(rect) => {
                    self.draw_rectangle(rect, ui, viewport, ctx, theme, y_offset, msgs);
                }
            }
            if annotation.open_quick_menu() {
                annotation.draw_annotation_quick_menu(ui, ctx, msgs, viewport, self);
            }
        }
    }
}
