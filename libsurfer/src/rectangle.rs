//use egui::{Ui, Response, Widget};
use crate::{config::SurferTheme, graphics::GraphicsY, view::DrawingContext, viewport::Viewport, wave_data::WaveData};
use egui::{Id, Rect, Response, Sense, Stroke, Ui, Widget};
use num::BigInt;

//TODO: Add dynamic color change in regards to selected theme. See get_marker_color in marker.rs

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RectAnnotation {
    pub id: Id,
    pub time_at_start: BigInt,
    pub time_at_end: BigInt,
    pub wave_from: Option<GraphicsY>,
    pub wave_to: Option<GraphicsY>,
    pub rect: Rect,
    pub color: egui::Color32,
    pub width: f32,
    pub group_name: Option<String>,
    pub visible: bool,
}

impl RectAnnotation {
    pub fn new(
        id_source: impl std::hash::Hash,
        time_at_start: BigInt,
        time_at_end: BigInt,
        rect: Rect,
    ) -> Self {
        Self {
            id: Id::new(id_source),
            time_at_start,
            time_at_end,
            wave_from: None,
            wave_to: None,
            rect,
            color: egui::Color32::from_rgb(255, 255, 255),
            width: 2.0,
            group_name: None,
            visible: true,
        }
    }
    pub fn get_id(&self) -> Id {
        self.id
    }

    pub fn get_time_at_start(&self) -> BigInt {
        return (&self.time_at_start + &self.time_at_end) / 2;
    }

    pub fn toggle_rectangle_visiblility(&mut self) {
        if self.visible {
            self.visible = false;
        } else {
            self.color = egui::Color32::TRANSPARENT;
        }
    }
}

fn point_inside_rect(p: emath::Pos2, rect: Rect) -> bool {
    if p.x >= rect.min.x && p.x <= rect.max.x && p.y >= rect.min.y && p.y <= rect.max.y {
        return true;
    }

    return false;
}

fn point_on_rect_border(p: emath::Pos2, rect: Rect, width: f32) -> bool {
    let half_width: f32 = width * 4.0; // TODO: fix this temporary width solution
    let outer_rect = Rect {
        min: emath::Pos2 {
            x: rect.min.x - half_width,
            y: rect.min.y - half_width,
        },
        max: emath::Pos2 {
            x: rect.max.x + half_width,
            y: rect.max.y + half_width,
        },
    };

    let inner_rect = Rect {
        min: emath::Pos2 {
            x: rect.min.x + half_width,
            y: rect.min.y + half_width,
        },
        max: emath::Pos2 {
            x: rect.max.x - half_width,
            y: rect.max.y - half_width,
        },
    };

    if point_inside_rect(p, outer_rect) && !point_inside_rect(p, inner_rect) {
        return true;
    }

    return false;
}

impl Widget for RectAnnotation {
    fn ui(self, ui: &mut Ui) -> Response {
        let stroke = Stroke::new(self.width, self.color);

        ui.painter()
            .rect_stroke(self.rect, 0.0, stroke, egui::StrokeKind::Middle);

        //always draw the rectangle but if we are on border we should also register clicks
        //this allows the click to be transferred unto the underlying panel so the rectangle is hollow
        let on_border = ui
            .ctx()
            .pointer_hover_pos()
            .map(|p| point_on_rect_border(p, self.rect, 6.0))
            .unwrap_or(false);

        if on_border {
            return ui.interact(self.rect, self.id, Sense::click_and_drag());
        } else {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::empty());
        }
    }
}

impl Default for RectAnnotation {
    fn default() -> RectAnnotation {
        RectAnnotation {
            // Generates a stable ID from a string hash
            id: egui::Id::new("default_rect_annotation"),
            time_at_start: BigInt::ZERO,
            time_at_end: BigInt::ZERO,
            rect: Rect::ZERO,
            wave_from: None,
            wave_to: None,
            color: egui::Color32::from_rgb(255, 255, 255),
            width: 0.0,
            group_name: None,
            visible: true,
        }
    }
}

impl WaveData {

    pub fn draw_rectangles(
        &self,
        ui: &mut egui::Ui,
        viewport: &Viewport,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
        y_offset: f32,
    ) {
        for rectangle in &self.rectangles {
            let num_timestamps = &self.safe_num_timestamps();

                let mut rectangle_annotation = rectangle.clone();
                rectangle_annotation.color = theme.annotation_rectangle.color;
                rectangle_annotation.rect.min.x = viewport.pixel_from_time(
                    &rectangle.time_at_start, 
                    ctx.cfg.canvas_size.x, 
                    &num_timestamps
                );

                rectangle_annotation.rect.max.x = viewport.pixel_from_time(
                    &rectangle.time_at_end, 
                    ctx.cfg.canvas_size.x, 
                    &num_timestamps
                );

                let from_y = rectangle_annotation
                    .wave_from
                    .as_ref()
                    .and_then(|from| self.get_item_y(from));
                let to_y = rectangle_annotation
                    .wave_to
                    .as_ref()
                    .and_then(|to| self.get_item_y(to));

                if let Some(to_y) = to_y
                    && let Some(from_y) = from_y
                {
                    if from_y < to_y {
                        rectangle_annotation.rect.min.y = from_y + y_offset;
                        rectangle_annotation.rect.max.y = to_y + y_offset;
                    } else {
                        rectangle_annotation.rect.min.y = to_y + y_offset;
                        rectangle_annotation.rect.max.y = from_y + y_offset;
                    }

                    rectangle_annotation.rect.min = (ctx.to_screen)(
                        rectangle_annotation.rect.min.x,
                        rectangle_annotation.rect.min.y,
                    );
                    rectangle_annotation.rect.max = (ctx.to_screen)(
                        rectangle_annotation.rect.max.x,
                        rectangle_annotation.rect.max.y,
                    );

                    let res = ui.add(rectangle_annotation); // clone is fine if needed for ui
                    if res.clicked_by(egui::PointerButton::Primary) {
                        println!("rec id: {:?}", rectangle.id);
                    }
                }
            }
        }
    }
