use crate::config::SurferTheme;
use crate::displayed_item::DisplayedItemRef;
use crate::graphics::Graphic;
use crate::{Viewport, view::DrawingContext, wave_data::WaveData};

use chrono::{DateTime, Local};
use egui::{Color32, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use num::BigInt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
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

// Sample rectangles along a line segment from a to b, with the given radius. This is used for hit testing the arrow.
fn sample_rects_along_segment(a: Pos2, b: Pos2, radius: f32) -> Vec<Rect> {
    let vector = b - a;
    let len = vector.length();

    if len <= 0.1 {
        return vec![Rect::from_center_size(a, Vec2::splat(radius * 2.0))];
    }

    let step = radius.max(4.0);
    let steps = (len / step).ceil() as usize;

    let mut rects = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let fraction_along_segment = i as f32 / steps as f32;
        let point_on_line = a + vector * fraction_along_segment;
        rects.push(Rect::from_center_size(
            point_on_line,
            Vec2::splat(radius * 2.0),
        ));
    }

    rects
}

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
}

impl ArrowAnnotation {
    pub(crate) fn new(
        from: WavePoint,
        to: WavePoint,
        color: egui::Color32,
        width: f32,
        head_mode: ArrowHeadMode,
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
        }
    }

    pub fn hide(&mut self) {
        self.display_mode = ArrowDisplayMode::Dot;
    }

    pub fn show(&mut self) {
        self.display_mode = ArrowDisplayMode::FullArrow;
    }

    pub fn marker_pos(&self) -> Pos2 {
        self.to.screen_pos
    }

    pub fn created_at_string(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
    }
}

impl Widget for ArrowAnnotation {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.display_mode {
            ArrowDisplayMode::Dot => {
                // Arrow is hidden, show dots
                let radius = (self.width * 2.0).max(4.0);
                let rect =
                    Rect::from_center_size(self.to.screen_pos, Vec2::splat(radius * 2.0 + 8.0));
                //let mut response: Option<Response> = None;
                ui.painter()
                    .circle_filled(self.to.screen_pos, radius, self.color);
                let mut response = Some(ui.interact(rect, self.id, Sense::click()));

                match self.head_mode {
                    ArrowHeadMode::End => {
                        // ingen ändring i logiken
                    }
                    ArrowHeadMode::Double => {
                        let from_point_rect = Rect::from_center_size(
                            self.from.screen_pos,
                            Vec2::splat(radius * 2.0 + 8.0),
                        );
                        let from_point_response =
                            ui.interact(from_point_rect, self.id.with("start"), Sense::click());
                        ui.painter()
                            .circle_filled(self.from.screen_pos, radius, self.color);
                        // Kombinera response så att både start och end kan interagera
                        if let Some(acc) = &mut response {
                            *acc |= from_point_response;
                        } else {
                            response = Some(from_point_response);
                        }
                    }
                }

                response.unwrap_or_else(|| {
                    ui.interact(
                        Rect::from_center_size(self.from.screen_pos, Vec2::ZERO),
                        self.id,
                        Sense::hover(),
                    )
                })
            }
            ArrowDisplayMode::FullArrow => {
                let stroke = Stroke::new(self.width, self.color);
                let hit_radius = self.width + 4.0;

                let end_head = arrow_geometry(self.from.screen_pos, self.to.screen_pos, self.width);

                let start_head = match self.head_mode {
                    ArrowHeadMode::End => None,
                    ArrowHeadMode::Double => {
                        arrow_geometry(self.to.screen_pos, self.from.screen_pos, self.width)
                    }
                };

                // Om pilen är för kort
                if end_head.is_none() {
                    let rect =
                        Rect::from_center_size(self.from.screen_pos, Vec2::splat(hit_radius * 2.0));
                    let response = ui.interact(rect, self.id, Sense::click());
                    ui.painter()
                        .circle_filled(self.from.screen_pos, self.width, self.color);
                    return response;
                }

                let (end_base, end_left, end_right) = end_head.unwrap();

                let shaft_start = match start_head {
                    Some((start_base, _, _)) => start_base,
                    None => self.from.screen_pos,
                };

                let shaft_end = end_base;

                // Rita skaftet
                ui.painter().line_segment([shaft_start, shaft_end], stroke);

                // Rita slut-huvudet
                ui.painter()
                    .line_segment([self.to.screen_pos, end_left], stroke);
                ui.painter()
                    .line_segment([self.to.screen_pos, end_right], stroke);
                ui.painter().line_segment([end_left, end_right], stroke);

                // Rita start-huvudet om dubbelpil
                if let Some((start_base, start_left, start_right)) = start_head {
                    ui.painter()
                        .line_segment([self.from.screen_pos, start_left], stroke);
                    ui.painter()
                        .line_segment([self.from.screen_pos, start_right], stroke);
                    ui.painter().line_segment([start_left, start_right], stroke);
                }

                let mut all_rects = Vec::new();

                // Hit test för skaftet
                all_rects.extend(sample_rects_along_segment(
                    shaft_start,
                    shaft_end,
                    hit_radius,
                ));

                // Hit test för slut-huvudet
                all_rects.extend(sample_rects_along_segment(
                    self.to.screen_pos,
                    end_left,
                    hit_radius,
                ));
                all_rects.extend(sample_rects_along_segment(
                    self.to.screen_pos,
                    end_right,
                    hit_radius,
                ));
                all_rects.extend(sample_rects_along_segment(end_left, end_right, hit_radius));

                // Hit test för start-huvudet om dubbelpil
                if let Some((_start_base, start_left, start_right)) = start_head {
                    all_rects.extend(sample_rects_along_segment(
                        self.from.screen_pos,
                        start_left,
                        hit_radius,
                    ));
                    all_rects.extend(sample_rects_along_segment(
                        self.from.screen_pos,
                        start_right,
                        hit_radius,
                    ));
                    all_rects.extend(sample_rects_along_segment(
                        start_left,
                        start_right,
                        hit_radius,
                    ));
                }

                let mut response: Option<Response> = None;

                for (i, rect) in all_rects.into_iter().enumerate() {
                    let r = ui.interact(rect, self.id.with(i), Sense::click());
                    if let Some(acc) = &mut response {
                        *acc |= r;
                    } else {
                        response = Some(r);
                    }
                }

                response.unwrap_or_else(|| {
                    ui.interact(
                        Rect::from_center_size(self.from.screen_pos, Vec2::ZERO),
                        self.id,
                        Sense::hover(),
                    )
                })
            }
        }
    }
}

impl WaveData {
    pub fn attached_item_exists(&mut self, item_ref: &DisplayedItemRef) -> bool {
        self.displayed_items.contains_key(item_ref)
    }

    //should return the item arrow is attached to
    pub fn item_ref_at_canvas_y(&self, y: f32) -> Option<DisplayedItemRef> {
        let vidx = self.get_item_at_y(y)?;
        //kan det vara det some är problemet? att item tree är indexerat på ett annat sätt?
        let node = self.items_tree.get_visible(vidx)?;
        Some(node.item_ref)
    }

    pub fn draw_arrows(
        &self,
        arrows: &Vec<ArrowAnnotation>,
        ui: &mut egui::Ui,
        viewport: Viewport,
        ctx: &mut DrawingContext,
        theme: &SurferTheme,
    ) {
        for arrow in arrows.iter() {
            let mut arrow_annotation = arrow.clone();
            arrow_annotation.color = theme.annotation_arrow.color;

            let to_y = match arrow.to.attached_item.as_ref() {
                Some(item_ref) => match item_center_y(self, item_ref) {
                    Some(y) => y,
                    None => continue, // slut-item är dolt -> rita inte pilen
                },
                None => continue,
            };

            let from_y = match arrow.head_mode {
                ArrowHeadMode::End => {
                    // gammal logik: räkna start från slut med offset
                    to_y - arrow.y_offset
                }
                ArrowHeadMode::Double => {
                    match arrow.from.attached_item.as_ref() {
                        Some(item_ref) => match item_center_y(self, item_ref) {
                            Some(y) => y,
                            None => continue, // start-item är dolt -> rita inte pilen
                        },
                        None => continue, // dubbelpil utan start-item är ogiltig
                    }
                }
            };

            let num_timestamps: BigInt = self.safe_num_timestamps();
            let frame_width = ctx.cfg.canvas_size.x;

            let new_to_x =
                viewport.pixel_from_time(&arrow_annotation.to.time, frame_width, &num_timestamps);

            let new_from_x =
                viewport.pixel_from_time(&arrow_annotation.from.time, frame_width, &num_timestamps);

            let mut new_to = (ctx.to_screen)(new_to_x, to_y);
            let mut new_from = (ctx.to_screen)(new_from_x, from_y);

            // y verkar redan vara i rätt koordinatsystem i din nuvarande kod
            new_to.y = to_y;
            new_from.y = from_y;

            arrow_annotation.to.screen_pos = new_to;
            arrow_annotation.from.screen_pos = new_from;

            let response = ui.add(arrow_annotation);

            if response.clicked() {
                println!("clicked");
            }

            if response.hovered() {
                println!("time: {}", arrow.created_at_string());
                println!("hovered id: {:?},", arrow.id);
            }
        }
    }
    pub fn hide_arrow(arrows: &mut Vec<ArrowAnnotation>, arrow_id: Id) {
        if let Some(arrow) = arrows.iter_mut().find(|a| a.id == arrow_id) {
            arrow.hide();
        }
    }

    pub fn show_arrow(arrows: &mut Vec<ArrowAnnotation>, arrow_id: Id) {
        if let Some(arrow) = arrows.iter_mut().find(|a| a.id == arrow_id) {
            arrow.show();
        }
    }
}
