//use egui::{Ui, Response, Widget};
use crate::{annotation::{Annotatable, AnnotationData}, config::SurferTheme, displayed_item::DisplayedItemRef, graphics::GraphicsY, message::Message, view::DrawingContext, viewport::Viewport, wave_data::WaveData};
use egui::{Id, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use emath::RectTransform;
use num::BigInt;

const DEFAULT_TYPE: &str = "Rectangle";
const GAMMA_FACTOR: f32 = 1.1;
const WIDTH_FACTOR: f32 = 1.3;

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AnchorPoint {
    pub wave: Option<GraphicsY>,
    pub time: BigInt,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RectAnnotation {
    pub annotation_data: AnnotationData,
    pub from: AnchorPoint,
    pub to: AnchorPoint,
    pub rect: Rect,
}

impl RectAnnotation {
    pub(crate) fn new(
        id: Id,
        time_at_start: BigInt,
        time_at_end: BigInt,
        wave_from: Option<GraphicsY>,
        wave_to: Option<GraphicsY>,
        rect: Rect,
        num: i32,
    ) -> Self {
        let name = format!("{} {}", DEFAULT_TYPE, num);
        let annotation_data = AnnotationData::new(id, name);
        Self {
            annotation_data,
            from: AnchorPoint {
                wave: wave_from,
                time: time_at_start,
            },
            to: AnchorPoint {
                wave: wave_to,
                time: time_at_end,
            },
            rect: rect,
        }
    }
    pub fn get_id(&self) -> Id {
        self.annotation_data.id
    }

    pub fn get_pos(
        &self,
        waves: &WaveData,
        viewport: &Viewport,
        ctx: &DrawingContext,
        y_offset: f32,
    ) -> Option<Pos2> {
        if !self.annotation_data.visible {
            return None;
        }

        let num_timestamps = waves.safe_num_timestamps();

        let x1 =
            viewport.pixel_from_time(&self.from.time, ctx.cfg.canvas_size.x, &num_timestamps);
        let x2 = viewport.pixel_from_time(&self.to.time, ctx.cfg.canvas_size.x, &num_timestamps);

        let from_y = self
            .from.wave
            .as_ref()
            .and_then(|from| waves.get_item_y(from))?;
        let to_y = self.to.wave.as_ref().and_then(|to| waves.get_item_y(to))?;

        let min_x = x1.min(x2);
        let min_y = (from_y + y_offset).min(to_y + y_offset);

        Some((ctx.to_screen)(min_x, min_y))
    }
}

pub(crate) fn calculate_y(wave: Option<GraphicsY>, waves: &WaveData) -> Option<f32> {
    wave.as_ref().and_then(|from| waves.get_item_y(from))
}

impl Annotatable for RectAnnotation {
    fn get_id(&self) -> Id {
        self.annotation_data.id
    }

    fn get_type(&self) -> &str {
        DEFAULT_TYPE
    }

    fn set_name(&mut self, name: String) {
        self.annotation_data.name = name;
    }

    fn get_name(&self) -> String {
        self.annotation_data.name.clone()
    }

    fn set_group_name(&mut self, name: Option<String>) {
        self.annotation_data.group_name = name;
    }

    fn get_group_name(&self) -> Option<String> {
        self.annotation_data.group_name.clone()
    }

    fn is_selected(&mut self) {
        self.annotation_data.stroke *= WIDTH_FACTOR;
        self.annotation_data.color.gamma_multiply(GAMMA_FACTOR);
    }

    fn set_visibility(&mut self, visible: bool) {
        self.annotation_data.visible = visible;
    }

    fn is_visible(&self) -> bool {
        self.annotation_data.visible
    }
    //fn toggle_visibility(&mut self);
    fn get_time_at_start(&self) -> BigInt {
        (&self.from.time + &self.to.time) / 2
    }

    fn is_attached(&self, removed_ref: &DisplayedItemRef) -> bool {
        self.from
            .wave
            .as_ref()
            .map_or(false, |wave| &wave.item == removed_ref)
            || self
                .to
                .wave
                .as_ref()
                .map_or(false, |wave| &wave.item == removed_ref)
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
        if self.is_visible() {
            let mut rectangle_annotation = self.clone();
            let viewport = waves.viewports[viewport_idx];

            let from_y = calculate_y(rectangle_annotation.from.wave.clone(), waves);

            let to_y = calculate_y(rectangle_annotation.to.wave.clone(), waves);

            if let Some(to_y) = to_y
                && let Some(from_y) = from_y
            {
                let num_timestamps = &waves.safe_num_timestamps();
                rectangle_annotation.annotation_data.color = theme.annotation_rectangle.color;
                rectangle_annotation.annotation_data.stroke = theme.annotation_rectangle.width;
                if waves.selected_annotation == Some(rectangle_annotation.annotation_data.id) {
                    rectangle_annotation.is_selected();
                }

                let min_y = to_y.min(from_y) + y_offset;
                let max_y = to_y.max(from_y) + y_offset;

                let min_x = viewport.pixel_from_time(
                    &self.from.time,
                    ctx.cfg.canvas_size.x,
                    &num_timestamps,
                );
                let max_x =
                    viewport.pixel_from_time(&self.to.time, ctx.cfg.canvas_size.x, &num_timestamps);

                rectangle_annotation.rect.min = (ctx.to_screen)(min_x, min_y);
                rectangle_annotation.rect.max = (ctx.to_screen)(max_x, max_y);

                let res = ui.add(rectangle_annotation).on_hover_ui(|ui| {
                    self.draw_hover_info(ui);
                });

                if res.clicked_by(egui::PointerButton::Primary) {
                    msgs.push(Message::SetActiveViewport(viewport_idx));
                    //waves.set_annotation_menu_pos_time(res.interact_pointer_pos().unwrap(), to_screen,viewport_idx, ctx.cfg.canvas_width);
                    msgs.push(Message::AnnotationClicked(
                        Some(self.annotation_data.id),
                        res.interact_pointer_pos(),
                        Some(viewport_idx),
                        Some(to_screen),
                        Some(ctx.cfg.canvas_size.x),
                    ));
                    msgs.push(Message::ClickHandled());
                }
            }
        }
    }
    
    fn get_time_at_end(&self) -> BigInt {
        return self.to.time.clone();
    }

    fn get_lowest_y_pos(&self, waves: &WaveData) -> f32 {
        //TODO: Make safer
        calculate_y(self.to.wave.clone(), waves).unwrap()
    }
}

fn point_inside_rect(p: emath::Pos2, rect: Rect) -> bool {
    if p.x >= rect.min.x && p.x <= rect.max.x && p.y >= rect.min.y && p.y <= rect.max.y {
        return true;
    }

    return false;
}

fn point_on_rect_border(p: emath::Pos2, rect: Rect, width: f32) -> (bool, Rect) {
    let half_width: f32 = width * 3.0; // TODO: fix this temporary width solution
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
    return (
        point_inside_rect(p, outer_rect) && !point_inside_rect(p, inner_rect),
        outer_rect,
    );
}

impl Widget for RectAnnotation {
    fn ui(self, ui: &mut Ui) -> Response {
        let stroke = Stroke::new(self.annotation_data.stroke, self.annotation_data.color);

        ui.painter()
            .rect_stroke(self.rect, 0.0, stroke, egui::StrokeKind::Middle);
        //alwayraw the rectangle but if we are on border we should also register clicks
        //this allows the click to be transferred unto the underlying panel so the rectangle is hollows d
        let (on_border, hitbox) = ui
            .ctx()
            .pointer_hover_pos()
            .map(|p| point_on_rect_border(p, self.rect, self.annotation_data.stroke))
            .unwrap_or((false, Rect::ZERO));

        if on_border {
            return ui.interact(hitbox, self.annotation_data.id, Sense::click_and_drag());
        } else {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::empty());
        }
    }
}
