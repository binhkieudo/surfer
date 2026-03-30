use egui::{
    Align, Area, Button, Color32, Frame, Id, Layout, Order, Pos2, RichText, Stroke, Ui, UiBuilder, Vec2, debug_text::print
};
use egui_remixicon::icons;
use emath::RectTransform;
use num::BigInt;
use serde::Serialize;

use crate::{
    SystemState,
    arrow::ArrowAnnotation,
    config::SurferTheme,
    displayed_item::{DisplayedItem, DisplayedItemRef},
    message::Message,
    rectangle::RectAnnotation,
    system_state,
    view::{self, DrawingContext},
    viewport::Viewport,
    wave_data::WaveData,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationData {
    pub id: Id,
    pub color: Color32,
    pub group_name: Option<String>,
    pub visible: bool,
    pub name: String,
    pub stroke: f32,
}

impl AnnotationData {
    pub(crate) fn new(id_source: impl std::hash::Hash, name: String) -> Self {
        AnnotationData {
            id: Id::new(id_source),
            group_name: None,
            visible: true,
            name: name,
            color: Color32::from_rgb(255, 255, 255),
            stroke: 2.0,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Annotation {
    Arrow(ArrowAnnotation),
    Rect(RectAnnotation),
}
impl Annotatable for Annotation {
    fn get_id(&self) -> Id {
        match self {
            Annotation::Arrow(a) => a.get_id(),
            Annotation::Rect(r) => r.get_id(),
        }
    }

    fn get_type(&self) -> &str {
        match self {
            Annotation::Arrow(a) => a.get_type(),
            Annotation::Rect(r) => r.get_type(),
        }
    }

    fn set_name(&mut self, name: String) {
        match self {
            Annotation::Arrow(a) => a.set_name(name),
            Annotation::Rect(r) => r.set_name(name),
        }
    }

    fn get_name(&self) -> String {
        match self {
            Annotation::Arrow(a) => a.get_name(),
            Annotation::Rect(r) => r.get_name(),
        }
    }

    fn set_group_name(&mut self, name: Option<String>) {
        match self {
            Annotation::Arrow(a) => a.set_group_name(name),
            Annotation::Rect(r) => r.set_group_name(name),
        }
    }

    fn get_group_name(&self) -> Option<String> {
        match self {
            Annotation::Arrow(a) => a.get_group_name(),
            Annotation::Rect(r) => r.get_group_name(),
        }
    }

    fn is_selected(&mut self) {
        match self {
            Annotation::Arrow(a) => a.is_selected(),
            Annotation::Rect(r) => r.is_selected(),
        }
    }

    fn set_visibility(&mut self, visible: bool) {
        match self {
            Annotation::Arrow(a) => a.set_visibility(visible),
            Annotation::Rect(r) => r.set_visibility(visible),
        }
    }

    fn is_visible(&self) -> bool {
        match self {
            Annotation::Arrow(a) => a.is_visible(),
            Annotation::Rect(r) => r.is_visible(),
        }
    }

    fn get_time_at_start(&self) -> BigInt {
        match self {
            Annotation::Arrow(a) => a.get_time_at_start(),
            Annotation::Rect(r) => r.get_time_at_start(),
        }
    }

    fn is_attached(&self, removed_ref: &DisplayedItemRef) -> bool {
        match self {
            Annotation::Arrow(a) => a.is_attached(removed_ref),
            Annotation::Rect(r) => r.is_attached(removed_ref),
        }
    }

    fn draw(
        &self,
        ui: &mut Ui,
        waves: &WaveData,
        viewport_idx: usize,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
        msgs: &mut Vec<Message>,
        y_offset: f32,
        to_screen: RectTransform,
    ) {
        match self {
            Annotation::Arrow(a) => a.draw(
                ui,
                waves,
                viewport_idx,
                ctx,
                theme,
                msgs,
                y_offset,
                to_screen,
            ),
            Annotation::Rect(r) => r.draw(
                ui,
                waves,
                viewport_idx,
                ctx,
                theme,
                msgs,
                y_offset,
                to_screen,
            ),
        }
    }
    fn get_time_at_end(&self) -> BigInt {
        match self {
            Annotation::Arrow(a) => a.get_time_at_end(),
            Annotation::Rect(r) => r.get_time_at_end(),
        }
    }

    fn get_lowest_y_pos(&self, waves: &WaveData) -> f32 {
        match self {
            Annotation::Arrow(a) => a.get_lowest_y_pos(waves),
            Annotation::Rect(r) => r.get_lowest_y_pos(waves),
        }
    }
}

pub trait Annotatable {
    fn get_id(&self) -> Id;
    fn get_type(&self) -> &str;
    fn set_name(&mut self, name: String);
    fn get_name(&self) -> String;
    fn set_group_name(&mut self, name: Option<String>);
    fn get_group_name(&self) -> Option<String>;
    fn is_selected(&mut self);
    fn set_visibility(&mut self, visible: bool);
    fn is_visible(&self) -> bool;
    //fn toggle_visibility(&mut self);
    fn get_time_at_start(&self) -> BigInt; //?
    fn is_attached(&self, removed_ref: &DisplayedItemRef) -> bool;
    fn draw(
        &self,
        ui: &mut Ui,
        waves: &WaveData,
        viewport_idx: usize,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
        msgs: &mut Vec<Message>,
        y_offset: f32,
        to_screen: RectTransform,
    );
    fn draw_quick_menu(
        &self,
        ui: &mut egui::Ui,
        msgs: &mut Vec<Message>,
        waves: &WaveData,
        viewport: &Viewport,
        ctx: &mut DrawingContext,
        y_offset: f32,
        viewport_rect: egui::Rect,
        position: Pos2,
    ) {
        let id = self.get_id();
        let num_timestamps = &waves.safe_num_timestamps();

        let menu_rect = egui::Rect::from_min_size(position, egui::vec2(0.0, 0.0)); //TODO: Magic nums

        if !viewport_rect.intersects(menu_rect) {
            // msgs.push(Message::AnnotationClicked(None));
            return;
        }

        egui::Area::new(egui::Id::new(("annotation_quick_menu", id)))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
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
                                msgs.push(Message::GoToAnnotationPosition(
                                    id,
                                    waves.last_active_viewport_idx,
                                ));
                            }

                            let vis_icon = if !self.is_visible() {
                                icons::EYE_OFF_LINE
                            } else {
                                icons::EYE_LINE
                            };

                            if ui.button(vis_icon).clicked() {
                                msgs.push(Message::ToggleAnnotationVisiblility(id));
                            }

                            if ui.button(icons::DELETE_BIN_LINE).clicked() {
                                msgs.push(Message::RemoveAnnotation(id));
                            }
                            if ui.button(icons::CHAT_4_LINE).clicked() {
                            let cursor_pos = ui.ctx().pointer_hover_pos().unwrap_or_default();
                            let time_anchor = viewport.as_time_bigint(
                                cursor_pos.x,
                                ctx.cfg.canvas_size.x,
                                num_timestamps,
                            );

                            msgs.push(Message::AddComment {
                                time_anchor: time_anchor,
                                y_anchor: cursor_pos.y,
                                annotation_id: id,
                            })
                        }
                        });
                    });
            });
    }
    fn draw_hover_info(&self, ui: &mut egui::Ui) {
        ui.label(format!("Type: {}", self.get_type()));
        ui.label(format!("Name: {}", self.get_name()));
        ui.label(format!("ID: {:?}", self.get_id()));
        ui.label(format!(
            "Group: {}",
            self.get_group_name()
                .unwrap_or_else(|| "Ungrouped".to_string())
        ));
        ui.label(format!("Visible: {}", self.is_visible()));
    }
    fn get_time_at_end(&self) -> BigInt;
    fn get_lowest_y_pos(&self, waves: &WaveData) -> f32;

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
        msgs: &mut Vec<Message>,
        y_offset: f32,
        viewport_rect: egui::Rect,
        to_screen: RectTransform,
        frame_size: Vec2,
    ) {
        let num_timestamps = self.safe_num_timestamps();

        for annotation in &self.annotations {
            annotation.draw(
                ui,
                &self,
                viewport_idx,
                ctx,
                theme,
                msgs,
                y_offset,
                to_screen,
            );

            if self.selected_annotation == Some(annotation.get_id())
                && viewport_idx == self.last_active_viewport_idx
            {
                let mut menu_position = self.annotation_menu_pos.unwrap();
                let menu_time = self.annotation_menu_time.clone().unwrap();

                menu_position.x = viewport.pixel_from_time(
                    &menu_time,
                    ctx.cfg.canvas_size.x,
                    &self.safe_num_timestamps(),
                );
                let temp_y = menu_position.y;
                menu_position = (ctx.to_screen)(menu_position.x, menu_position.y);
                menu_position.y = temp_y;

                annotation.draw_quick_menu(
                    ui,
                    msgs,
                    &self,
                    viewport,
                    ctx,
                    y_offset,
                    viewport_rect,
                    menu_position,
                );

                //annotation.is_selected();
            }
        }
    }

    // TODO: is not used, should we let the logic stay in lib?
    pub fn set_annotation_menu_pos_time(
        &mut self,
        menu_pos: Pos2,
        to_screen: RectTransform,
        viewport_idx: usize,
        frame_width: f32,
    ) {
        let num_timestamps: BigInt = self.safe_num_timestamps();

        let menu_pos_local = to_screen.inverse().transform_pos(menu_pos);

        let menu_pos_time: BigInt = self.viewports[viewport_idx].as_time_bigint(
            menu_pos_local.x, // HÄR blir felet. jag måste få in x lokalt
            frame_width,
            &num_timestamps,
        );

        self.annotation_menu_time = Some(menu_pos_time);

        self.annotation_menu_pos = Some(menu_pos);
    }
}
