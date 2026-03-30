use crate::config::SurferTheme;
use crate::displayed_item::DisplayedItemRef;
use crate::graphics::Graphic;
use crate::message::Message;
use crate::view::GroupDrawingInfo;
use crate::{Viewport, view::DrawingContext, wave_data::WaveData};

use chrono::{DateTime, Local, offset};
use egui::{Color32, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use num::BigInt;
use serde::{Deserialize, Serialize};

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
    pub id: Id,
    pub from: WavePoint,
    pub to: WavePoint,
    pub color: Color32,
    pub width: f32,
    pub display_mode: ArrowDisplayMode,
    pub created_at: DateTime<Local>,
    pub y_offset: f32,
    pub head_mode: ArrowHeadMode,
    pub group_name: Option<String>,
    pub visible: bool,
    pub name: String,
    pub open_quick_menu: bool,
}

impl ArrowAnnotation {
    pub(crate) fn new(
        from: WavePoint,
        to: WavePoint,
        color: egui::Color32,
        width: f32,
        head_mode: ArrowHeadMode,
        group_name: Option<String>,
    ) -> Self {
        ArrowAnnotation {
            id: Id::new(format!("arrow_{:?}_{:?}", from.screen_pos, to.screen_pos)),
            from: from.clone(),
            to: to.clone(),
            color,
            width,
            display_mode: ArrowDisplayMode::FullArrow,
            created_at: Local::now(),
            y_offset: to.screen_pos.y - from.screen_pos.y,
            head_mode,
            group_name,
            visible: true,
            name: "".to_string(),
            open_quick_menu: false,
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
            self.visible = false;
        } else {
            self.show();
            self.visible = true;
        }
    }

    pub fn get_time_at_start(&self) -> BigInt {
        return (&self.from.time + &self.to.time) / 2;
    }

    fn hit_radius(&self) -> f32 {
        self.width + 4.0
    }

    fn segments(&self) -> Option<ArrowSegments> {
        //få frame pilens viktiga punkter start slut pilens huvud
        let end_head = arrow_geometry(self.from.screen_pos, self.to.screen_pos, self.width)?;
        let (end_base, end_left, end_right) = end_head;

        let start_head: Option<(Pos2, Pos2, Pos2)> = match self.head_mode {
            ArrowHeadMode::End => None,
            ArrowHeadMode::Double => {
                arrow_geometry(self.to.screen_pos, self.from.screen_pos, self.width)
            }
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
                let radius = (self.width * 2.0).max(4.0);
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
                let radius = (self.width * 2.0).max(4.0) + 4.0;
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
        match self.display_mode {
            ArrowDisplayMode::Dot => {
                // rita först sen kalla på coarse_hit_rect för att lägga till hit boxarna
                let radius = (self.width * 2.0).max(4.0);

                ui.painter()
                    .circle_filled(self.to.screen_pos, radius, self.color);

                if let ArrowHeadMode::Double = self.head_mode {
                    ui.painter()
                        .circle_filled(self.from.screen_pos, radius, self.color);
                }

                let hit_rect = self.coarse_hit_rect();
                ui.interact(hit_rect, self.id, Sense::click())
            }

            ArrowDisplayMode::FullArrow => {
                let stroke = Stroke::new(self.width, self.color);

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
                    ui.interact(hit_rect, self.id, Sense::click())
                } else {
                    let rect = Rect::from_center_size(
                        self.from.screen_pos,
                        Vec2::splat(self.hit_radius() * 2.0),
                    );
                    let response = ui.interact(rect, self.id, Sense::click());
                    ui.painter()
                        .circle_filled(self.from.screen_pos, self.width, self.color);
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

    pub fn draw_arrow(
        &self,
        arrow: &ArrowAnnotation,
        ui: &mut egui::Ui,
        viewport: Viewport,
        viewport_idx: usize,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
        msgs: &mut Vec<Message>,
    ) {
        let mut arrow_annotation = arrow.clone();
        //arrow_annotation.color = theme.annotation_arrow.color;
        arrow_annotation.id = egui::Id::new(("arrow", arrow.id, viewport_idx));
        arrow_annotation.color = theme.annotation_arrow.color;
        arrow_annotation.width = theme.annotation_arrow.width;

        let to_y = match arrow.to.attached_item.as_ref() {
            Some(item_ref) => match item_center_y(self, item_ref) {
                Some(y) => y,
                None => return,
            },
            None => return,
        };

        let from_y = match arrow.head_mode {
            ArrowHeadMode::End => to_y - arrow.y_offset,
            ArrowHeadMode::Double => match arrow.from.attached_item.as_ref() {
                Some(item_ref) => match item_center_y(self, item_ref) {
                    Some(y) => y,
                    None => return,
                },
                None => return,
            },
        };

        let num_timestamps: BigInt = self.safe_num_timestamps();
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

        if exact_clicked {
            arrow_annotation.color = Color32::RED;
            msgs.push(Message::AnnotationClicked(arrow.id));
            println!("clicked");
        }

        if exact_hovered {
            println!("time: {}", arrow.created_at_string());
            println!("hovered id: {:?},", arrow.id);
        }

        let _response = ui.add(arrow_annotation);
    }
}
