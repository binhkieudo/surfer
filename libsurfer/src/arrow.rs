use crate::annotation::Annotation;
use crate::annotation::{Annotatable, AnnotationData};
use crate::config::SurferTheme;
use crate::displayed_item::{DisplayedItem, DisplayedItemRef};
use crate::graphics::Graphic;
use crate::message::Message;
use crate::view::GroupDrawingInfo;
use crate::{Viewport, view::DrawingContext, wave_data::WaveData};

use chrono::{DateTime, Local, offset};
use egui::{Color32, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use num::BigInt;
use serde::{Deserialize, Serialize};

const DEFAULT_TYPE: &str = "Arrow";
const GAMMA_FACTOR: f32 = 1.1;
const WIDTH_FACTOR: f32 = 1.3;


#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum ArrowDisplayMode {
    FullArrow,
    Dot,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ArrowHeadMode {
    End,    // vanlig pil
    Double, // dubbelriktad pil
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WavePoint {
    pub time: BigInt,
    pub attached_item: Option<DisplayedItemRef>,
    pub screen_pos: Pos2,
}

#[derive(Clone, Copy, Debug)]
struct ArrowSegments {
    shaft_start: Pos2,
    shaft_end: Pos2,
    end_tip: Pos2,
    end_left: Pos2,
    end_right: Pos2,
    start_tip: Option<Pos2>,
    start_left: Option<Pos2>,
    start_right: Option<Pos2>,
}

fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;

    let ab_len_sq = ab.length_sq();
    if ab_len_sq <= 0.0001 {
        return ap.length();
    }

    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

fn rect_from_points(points: &[Pos2], pad: f32) -> Rect {
    let mut min_x = points[0].x;
    let mut max_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_y = points[0].y;

    for p in points.iter().skip(1) {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y)).expand(pad)
}
// NYYY yNYNYNYNYN YNYNYNNY

fn arrow_geometry(from: Pos2, to: Pos2, width: f32) -> Option<(Pos2, Pos2, Pos2)> {
    let v = to - from;
    let len = v.length();

    if len <= 0.1 {
        return None;
    }

    let dir = v / len;
    let perp = Vec2::new(-dir.y, dir.x);

    let head_len = (width * 4.0).max(10.0);
    let head_half_width = (width * 2.0).max(6.0);

    let base = to - dir * head_len;
    let left = base + perp * head_half_width;
    let right = base - perp * head_half_width;

    Some((base, left, right))
}

fn item_center_y(waves: &WaveData, item_ref: &DisplayedItemRef) -> Option<f32> {
    match waves.get_displayed_item_index(item_ref) {
        Some(vidx) => {
            let info = waves.drawing_infos.get(vidx.0)?;
            Some((info.top() + info.bottom()) * 0.5)
        }
        None => None,
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArrowAnnotation {
    pub from: WavePoint,
    pub to: WavePoint,
    pub display_mode: ArrowDisplayMode,
    pub created_at: DateTime<Local>,
    pub y_offset: f32,
    pub head_mode: ArrowHeadMode,
    pub annotation_data: AnnotationData,
}

impl Annotatable for ArrowAnnotation {
    fn get_id(&self) -> Id {
        self.annotation_data.id
    }
    fn get_type(&self) -> &str {
        "Arrow"
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
        self.annotation_data.stroke = WIDTH_FACTOR * 2.0;
        self.annotation_data.color.gamma_multiply(GAMMA_FACTOR);
    }

    fn set_visibility(&mut self, visible: bool) {
        if visible {
            self.show();
            self.annotation_data.visible = true;
        } else {
            self.hide();
            self.annotation_data.visible = false;
        }
    }
    fn is_visible(&self) -> bool {
        self.annotation_data.visible
    }
    //fn toggle_visibility(&mut self);
    fn get_time_at_start(&self) -> BigInt {
        (&self.from.time + &self.to.time) / 2
    }

    fn menu_position(
        &self,
        waves: &WaveData,
        viewport: &Viewport,
        ctx: &mut DrawingContext,
        y_offset: f32,
    ) -> Pos2 {
        self.to.screen_pos
    }

    fn is_attached(&self, removed_ref: &DisplayedItemRef) -> bool {
        self.to.attached_item.as_ref() == Some(removed_ref)
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
    ) {
        let mut arrow_annotation = self.clone();

        arrow_annotation.annotation_data.color = theme.annotation_arrow.color;
        arrow_annotation.annotation_data.stroke = theme.annotation_arrow.width;

        if waves.selected_annotation == Some(self.annotation_data.id) {
            arrow_annotation.is_selected();
        }

        arrow_annotation.annotation_data.id =
            egui::Id::new(("arrow", self.annotation_data.id, viewport_idx));

        let to_y = match self.to.attached_item.as_ref() {
            Some(item_ref) => match item_center_y(waves, item_ref) {
                Some(y) => y,
                None => return,
            },
            None => return,
        };

        let from_y = match self.head_mode {
            ArrowHeadMode::End => to_y - self.y_offset, //TODO: Change this to input so we dont have to save as variable.
            ArrowHeadMode::Double => match self.from.attached_item.as_ref() {
                Some(item_ref) => match item_center_y(waves, item_ref) {
                    Some(y) => y,
                    None => return,
                },
                None => return,
            },
        };

        let num_timestamps: BigInt = waves.safe_num_timestamps();
        let viewport = waves.viewports[viewport_idx];
        let frame_width = ctx.cfg.canvas_size.x;

        let new_to_x =
            viewport.pixel_from_time(&arrow_annotation.to.time, frame_width, &num_timestamps);

        let new_from_x =
            viewport.pixel_from_time(&arrow_annotation.from.time, frame_width, &num_timestamps);

        let mut new_to = (ctx.to_screen)(new_to_x, to_y);
        let mut new_from = (ctx.to_screen)(new_from_x, from_y);

        new_to.y = to_y;
        new_from.y = from_y;

        if !new_to.x.is_finite()
            || !new_to.y.is_finite()
            || !new_from.x.is_finite()
            || !new_from.y.is_finite()
        {
            return;
        }

        arrow_annotation.to.screen_pos = new_to;
        arrow_annotation.from.screen_pos = new_from;

        let pointer_hover_pos = ui.input(|i| i.pointer.hover_pos());
        let pointer_click_pos = ui.input(|i| i.pointer.interact_pos());
        let primary_clicked = ui.input(|i| i.pointer.primary_clicked());

        let exact_hovered = pointer_hover_pos
            .and_then(|p| arrow_annotation.hit_distance_screen(p))
            .is_some();

        let exact_clicked = primary_clicked
            && pointer_click_pos
                .and_then(|p| arrow_annotation.hit_distance_screen(p))
                .is_some();

        //let _response = ui.add(arrow_annotation);

        let _response = ui.add(arrow_annotation);

        if exact_clicked {
            //waves.selected_annotation = Some(arrow_annotation.annotation_data.id);
            msgs.push(Message::SetActiveViewport(viewport_idx));
            msgs.push(Message::AnnotationClicked(Some(self.annotation_data.id)));
            println!("clicked");
        }

        if exact_hovered {
            if let Some(pointer_pos) = pointer_hover_pos {
                let hover_rect = egui::Rect::from_center_size(pointer_pos, egui::vec2(1.0, 1.0));

                let hover_response = ui.interact(
                    hover_rect,
                    egui::Id::new(("arrow_hover_info", self.annotation_data.id, viewport_idx)),
                    egui::Sense::hover(),
                );

                hover_response.on_hover_ui(|ui| {
                    self.draw_hover_info(ui);
                });
            }
        }
    }
}

impl ArrowAnnotation {
    pub(crate) fn new(
        id: Id,
        from: WavePoint,
        to: WavePoint,
        head_mode: ArrowHeadMode,
        num: i32,
    ) -> Self {
        let name = format!("{} {}", DEFAULT_TYPE, num);
        let annotation_data = AnnotationData::new(id, name);

        ArrowAnnotation {
            from: from.clone(),
            to: to.clone(),
            display_mode: ArrowDisplayMode::FullArrow,
            created_at: Local::now(),
            y_offset: to.screen_pos.y - from.screen_pos.y,
            head_mode,
            annotation_data,
        }
    }

    pub fn hide(&mut self) {
        self.display_mode = ArrowDisplayMode::Dot;
    }

    pub fn show(&mut self) {
        self.display_mode = ArrowDisplayMode::FullArrow;
    }

    // pub fn marker_pos(&self) -> Pos2 {
    //     self.to.screen_pos
    // }

    pub fn created_at_string(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
    }
    pub fn toggle_arrow_visibility(&mut self) {
        if self.display_mode == ArrowDisplayMode::FullArrow {
            self.hide();
            self.annotation_data.visible = false;
        } else {
            self.show();
            self.annotation_data.visible = true;
        }
    }

    fn hit_radius(&self) -> f32 {
        self.annotation_data.stroke + 4.0
    }

    fn segments(&self) -> Option<ArrowSegments> {
        //få frame pilens viktiga punkter start slut pilens huvud
        let end_head = arrow_geometry(
            self.from.screen_pos,
            self.to.screen_pos,
            self.annotation_data.stroke,
        )?;
        let (end_base, end_left, end_right) = end_head;

        let start_head: Option<(Pos2, Pos2, Pos2)> = match self.head_mode {
            ArrowHeadMode::End => None,
            ArrowHeadMode::Double => arrow_geometry(
                self.to.screen_pos,
                self.from.screen_pos,
                self.annotation_data.stroke,
            ),
        };

        let shaft_start = match start_head {
            Some((start_base, _, _)) => start_base,
            None => self.from.screen_pos,
        };

        let shaft_end = end_base;

        let (start_tip, start_left, start_right) = match start_head {
            Some((_base, left, right)) => (Some(self.from.screen_pos), Some(left), Some(right)),
            None => (None, None, None),
        };

        Some(ArrowSegments {
            shaft_start,
            shaft_end,
            end_tip: self.to.screen_pos,
            end_left,
            end_right,
            start_tip,
            start_left,
            start_right,
        })
    }

    fn coarse_hit_rect(&self) -> Rect {
        match self.display_mode {
            ArrowDisplayMode::Dot => {
                let radius = (self.annotation_data.stroke * 2.0).max(4.0);
                match self.head_mode {
                    ArrowHeadMode::End => {
                        Rect::from_center_size(self.to.screen_pos, Vec2::splat(radius * 2.0 + 8.0))
                    }
                    ArrowHeadMode::Double => {
                        rect_from_points(&[self.from.screen_pos, self.to.screen_pos], radius + 8.0)
                    }
                }
            }
            ArrowDisplayMode::FullArrow => {
                if let Some(seg) = self.segments() {
                    let mut points = vec![
                        seg.shaft_start,
                        seg.shaft_end,
                        seg.end_tip,
                        seg.end_left,
                        seg.end_right,
                    ];

                    if let Some(p) = seg.start_tip {
                        points.push(p);
                    }
                    if let Some(p) = seg.start_left {
                        points.push(p);
                    }
                    if let Some(p) = seg.start_right {
                        points.push(p);
                    }

                    rect_from_points(&points, self.hit_radius())
                } else {
                    Rect::from_center_size(
                        self.from.screen_pos,
                        Vec2::splat(self.hit_radius() * 2.0),
                    )
                }
            }
        }
    }

    pub fn hit_distance_screen(&self, pointer: Pos2) -> Option<f32> {
        match self.display_mode {
            ArrowDisplayMode::Dot => {
                let radius = (self.annotation_data.stroke * 2.0).max(4.0) + 4.0;
                let mut best = (pointer - self.to.screen_pos).length();

                if let ArrowHeadMode::Double = self.head_mode {
                    best = best.min((pointer - self.from.screen_pos).length());
                }

                if best <= radius { Some(best) } else { None }
            }

            ArrowDisplayMode::FullArrow => {
                let seg = self.segments()?;
                let hit_radius = self.hit_radius();

                let mut best = f32::INFINITY;

                // Skaftet
                best = best.min(distance_to_segment(pointer, seg.shaft_start, seg.shaft_end));

                // Slut-huvudet, 3 segment
                best = best.min(distance_to_segment(pointer, seg.end_tip, seg.end_left));
                best = best.min(distance_to_segment(pointer, seg.end_tip, seg.end_right));
                best = best.min(distance_to_segment(pointer, seg.end_left, seg.end_right));

                // Start-huvudet om dubbelpil
                if let (Some(start_tip), Some(start_left), Some(start_right)) =
                    (seg.start_tip, seg.start_left, seg.start_right)
                {
                    best = best.min(distance_to_segment(pointer, start_tip, start_left));
                    best = best.min(distance_to_segment(pointer, start_tip, start_right));
                    best = best.min(distance_to_segment(pointer, start_left, start_right));
                }

                if best <= hit_radius { Some(best) } else { None }
            }
        }
    }
    pub fn get_pos(
        &self,
        waves: &WaveData,
        viewport: &Viewport,
        ctx: &DrawingContext,
        offset_y: f32,
    ) -> Option<Pos2> {
        let num_timestamps = waves.safe_num_timestamps();

        let to_x = viewport.pixel_from_time(&self.to.time, ctx.cfg.canvas_size.x, &num_timestamps);
        // let x2 =
        //     viewport.pixel_from_time(&self.from.time, ctx.cfg.canvas_width, &num_timestamps);

        //let from_y = self.wave_from.as_ref().and_then(|from| waves.get_item_y(from))?;
        let to_y = self.to.screen_pos.y;
        let mut position = (ctx.to_screen)(to_x, to_y);
        position.y = to_y + offset_y; // Justera y-positionen baserat på offset

        Some(position)
    }
}

impl Widget for ArrowAnnotation {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = ui.allocate_response(egui::Vec2::ZERO, egui::Sense::empty());
        match self.display_mode {
            ArrowDisplayMode::Dot => {
                // rita först sen kalla på coarse_hit_rect för att lägga till hit boxarna
                let radius = (self.annotation_data.stroke * 2.0).max(4.0);

                ui.painter()
                    .circle_filled(self.to.screen_pos, radius, self.annotation_data.color);

                if let ArrowHeadMode::Double = self.head_mode {
                    ui.painter().circle_filled(
                        self.from.screen_pos,
                        radius,
                        self.annotation_data.color,
                    );
                }

                let hit_rect = self.coarse_hit_rect();
                ui.interact(hit_rect, self.annotation_data.id, Sense::click())
            }

            ArrowDisplayMode::FullArrow => {
                let stroke = Stroke::new(self.annotation_data.stroke, self.annotation_data.color);

                if let Some(seg) = self.segments() {
                    // Skaftet
                    ui.painter()
                        .line_segment([seg.shaft_start, seg.shaft_end], stroke);

                    // Slut-huvudet
                    ui.painter()
                        .line_segment([seg.end_tip, seg.end_left], stroke);
                    ui.painter()
                        .line_segment([seg.end_tip, seg.end_right], stroke);
                    ui.painter()
                        .line_segment([seg.end_left, seg.end_right], stroke);

                    // Start-huvudet om dubbelpil
                    if let (Some(start_tip), Some(start_left), Some(start_right)) =
                        (seg.start_tip, seg.start_left, seg.start_right)
                    {
                        ui.painter().line_segment([start_tip, start_left], stroke);
                        ui.painter().line_segment([start_tip, start_right], stroke);
                        ui.painter().line_segment([start_left, start_right], stroke);
                    }

                    let hit_rect = self.coarse_hit_rect();
                    ui.interact(hit_rect, self.annotation_data.id, Sense::click())
                } else {
                    let rect = Rect::from_center_size(
                        self.from.screen_pos,
                        Vec2::splat(self.hit_radius() * 2.0),
                    );
                    let response = ui.interact(rect, self.annotation_data.id, Sense::click());
                    ui.painter().circle_filled(
                        self.from.screen_pos,
                        self.annotation_data.stroke,
                        self.annotation_data.color,
                    );
                    response
                }
            }
        }
    }
}


impl WaveData {
    pub fn item_ref_at_canvas_y(&self, y: f32) -> Option<DisplayedItemRef> {
        let vidx = self.get_item_at_y(y)?;
        let node = self.items_tree.get_visible(vidx)?;
        Some(node.item_ref)
    }

    // pub fn show_arrow(arrows: &mut Vec<ArrowAnnotation>, arrow_id: Id) {
    //     if let Some(arrow) = arrows.iter_mut().find(|a| a.id == arrow_id) {
    //         arrow.show();
    //     }
    // }
}
