//! Code related to the mouse gesture handling.
use derive_more::Display;
use egui::{Context, Painter, PointerButton, Response, RichText, Sense, Window};
use emath::{Align2, Pos2, Rect, RectTransform, Vec2};
use epaint::{FontId, Stroke};
use num::BigInt;
use serde::Deserialize;

use crate::arrow::{ArrowHeadMode, WavePoint};
use crate::config::{SurferConfig, SurferTheme};
use crate::graphics::{Anchor, GrPoint, GraphicsY};
use crate::time::TimeFormatter;
use crate::view::DrawingContext;
use crate::{Message, SystemState, wave_data::WaveData};

/// Geometric constant: tan(22.5°) used for gesture zone calculations
const TAN_22_5_DEGREES: f32 = 0.41421357;

/// Helper function to create a stroke with appropriate color and width based on mode
fn create_gesture_stroke(config: &SurferConfig, is_measure: bool) -> Stroke {
    let line_style = if is_measure {
        &config.theme.measure
    } else {
        &config.theme.gesture
    };
    Stroke::from(line_style)
}

/// The supported mouse gesture operations.
#[derive(Clone, PartialEq, Copy, Display, Debug, Deserialize)]
enum GestureKind {
    #[display("Zoom to fit")]
    ZoomToFit,
    #[display("Zoom in")]
    ZoomIn,
    #[display("Zoom out")]
    ZoomOut,
    #[display("Go to end")]
    GoToEnd,
    #[display("Go to start")]
    GoToStart,
    Cancel,
}

/// The supported mouse gesture zones.
#[derive(Clone, PartialEq, Copy, Debug, Deserialize)]
pub struct GestureZones {
    north: GestureKind,
    northeast: GestureKind,
    east: GestureKind,
    southeast: GestureKind,
    south: GestureKind,
    southwest: GestureKind,
    west: GestureKind,
    northwest: GestureKind,
}

impl SystemState {
    fn clamp_y(&self, pos: Pos2, max_y: f32) -> Pos2 {
        Pos2 {
            x: pos.x,
            y: pos.y.min(max_y),
        }
    }

    /// Draw the mouse gesture widget, i.e., the line(s) and text showing which gesture is being drawn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_mouse_gesture_widget(
        &self,
        egui_ctx: &Context,
        waves: &WaveData,
        pointer_pos_canvas: Option<Pos2>,
        response: &Response,
        msgs: &mut Vec<Message>,
        ctx: &mut DrawingContext,
        viewport_idx: usize,
        y_offset: f32,
    ) {
        if let Some(mut start_location) = self.gesture_start_location {
            if self.add_rectangle && start_location.y > waves.get_content_height(y_offset) {
                return;
            }
            if let Some(time) = &self.gesture_start_time{
                let x_pixel = waves.viewports[viewport_idx].pixel_from_time(&time, ctx.cfg.canvas_size.x, &waves.safe_num_timestamps());
                start_location.x = x_pixel;
            }
            let modifiers = egui_ctx.input(|i| i.modifiers);
            if response.dragged_by(PointerButton::Middle)
                || modifiers.command && response.dragged_by(PointerButton::Primary)
                || self.add_rectangle && response.dragged_by(PointerButton::Primary)
                || self.add_arrow && response.dragged_by(PointerButton::Primary)
            {
                self.start_dragging(
                    pointer_pos_canvas,
                    start_location,
                    ctx,
                    response,
                    waves,
                    viewport_idx,
                    y_offset,
                );
            }

            if response.drag_stopped_by(PointerButton::Middle)
                || modifiers.command && response.drag_stopped_by(PointerButton::Primary)
                || self.add_rectangle && response.drag_stopped_by(PointerButton::Primary)
                || self.add_arrow && response.drag_stopped_by(PointerButton::Primary)
            {
                let frame_width = response.rect.width();
                self.stop_dragging(
                    pointer_pos_canvas,
                    start_location,
                    msgs,
                    viewport_idx,
                    waves,
                    frame_width,
                    ctx,
                    y_offset,
                );
            }
        }
    }

    fn stop_dragging(
        &self,
        pointer_pos_canvas: Option<Pos2>,
        start_location: Pos2,
        msgs: &mut Vec<Message>,
        viewport_idx: usize,
        waves: &WaveData,
        frame_width: f32,
        ctx: &mut DrawingContext<'_>,
        y_offset: f32,
    ) {
        let num_timestamps = waves.safe_num_timestamps();
        let Some(end_location) = pointer_pos_canvas else {
            return;
        };
        let distance = end_location - start_location;
        if distance.length_sq() >= self.user.config.gesture.deadzone {
            if self.add_rectangle == true {
                let end_location = self.clamp_y(end_location, waves.get_content_height(y_offset));
                let temp_rect = emath::Rect::from_two_pos(start_location, end_location);
                let color = self.user.config.theme.annotation_rectangle.color;
                let width = self.user.config.theme.annotation_rectangle.width;

                let id = egui::Id::new(self.next_id_source);

                let first_time = waves.viewports[viewport_idx].as_time_bigint(
                    start_location.x,
                    frame_width,
                    &num_timestamps,
                );

                let second_time = waves.viewports[viewport_idx].as_time_bigint(
                    end_location.x,
                    frame_width,
                    &num_timestamps,
                );

                let wave_from: Option<GraphicsY> = if let Some(vidx_from) =
                    waves.get_item_at_y(start_location.y - y_offset)
                    && let Some(node) = waves.items_tree.get_visible(vidx_from)
                {
                    let from_item_ref = node.item_ref;
                    let p = waves.get_item_y_scale(from_item_ref, start_location.y - y_offset);
                    println!("p value: {:?}", Some(p));
                    Some(GraphicsY {
                        item: from_item_ref,
                        anchor: Anchor::Percentual,
                        p: p,
                    })
                } else {
                    None
                };
                let wave_to = if let Some(vidx_to) = waves.get_item_at_y(end_location.y - y_offset)
                    && let Some(node) = waves.items_tree.get_visible(vidx_to)
                {
                    let from_item_ref = node.item_ref;
                    let p = waves.get_item_y_scale(from_item_ref, end_location.y - y_offset);

                    Some(GraphicsY {
                        item: from_item_ref,
                        anchor: Anchor::Percentual,
                        p: p,
                    })
                } else {
                    None
                };

                if first_time < second_time {
                    msgs.push(Message::RectangleAdded {
                        id,
                        time_at_start: first_time,
                        time_at_end: second_time,
                        wave_from: wave_from,
                        wave_to: wave_to,
                        rect: temp_rect,
                        color,
                        width,
                    });
                } else {
                    msgs.push(Message::RectangleAdded {
                        id,
                        time_at_end: first_time,
                        time_at_start: second_time,
                        wave_to: wave_from,
                        wave_from: wave_to,
                        rect: temp_rect,
                        color,
                        width,
                    });
                }
            } else if self.add_arrow == true {
                let color = self.user.config.theme.annotation_arrow.color;
                let width = self.user.config.theme.annotation_arrow.width;
                let start_pos = (ctx.to_screen)(start_location.x, start_location.y);
                let end_pos = (ctx.to_screen)(end_location.x, end_location.y);
                println!("end_pos_y: {}", end_pos.y);

                //fix offset, difference between wave data with timeline, pointer_pos_canvas and pointer_pos_mouse_gesture.
                let mut offset = 0.0;
                if self.show_default_timeline() {
                    offset = 20.0;
                }

                let time_from: BigInt = waves.viewports[viewport_idx].as_time_bigint(
                    start_location.x,
                    frame_width,
                    &num_timestamps,
                );

                let snap_pos = Some(Pos2::new(end_location.x, end_location.y - offset));

                let time_to: BigInt = self
                    .snap_to_edge(snap_pos, waves, frame_width, viewport_idx)
                    .unwrap_or_else(|| {
                        waves.viewports[viewport_idx].as_time_bigint(
                            end_location.x,
                            frame_width,
                            &num_timestamps,
                        )
                    });

                let attached_item_to = waves.item_ref_at_canvas_y(end_location.y - offset);
                let attached_item_from = waves.item_ref_at_canvas_y(start_location.y - offset);

                let mut head_mode = ArrowHeadMode::End;

                if self.add_double_headed_arrow == true {
                    head_mode = ArrowHeadMode::Double;
                };

                // let from : GrPoint = ;

                println!("Mode: {:?}", head_mode);

                let wave_point_from = WavePoint {
                    time: time_from.clone(),
                    attached_item: attached_item_from,
                    screen_pos: start_pos,
                };

                let wave_point_to = WavePoint {
                    time: time_to.clone(),
                    attached_item: attached_item_to,
                    screen_pos: end_pos,
                };

                if attached_item_to.is_some() {
                    msgs.push(Message::ArrowAdded {
                        wave_point_from,
                        wave_point_to,
                        color,
                        width,
                        head_mode,
                    });
                };
            } else {
                match gesture_type(self.user.config.gesture.mapping, distance) {
                    GestureKind::ZoomToFit => {
                        msgs.push(Message::ZoomToFit { viewport_idx });
                    }
                    GestureKind::ZoomIn => {
                        let (min_x, max_x) = if end_location.x < start_location.x {
                            (end_location.x, start_location.x)
                        } else {
                            (start_location.x, end_location.x)
                        };
                        msgs.push(Message::ZoomToRange {
                            // FIXME: No need to go via bigint here, this could all be relative
                            start: waves.viewports[viewport_idx].as_time_bigint(
                                min_x,
                                frame_width,
                                &num_timestamps,
                            ),
                            end: waves.viewports[viewport_idx].as_time_bigint(
                                max_x,
                                frame_width,
                                &num_timestamps,
                            ),
                            viewport_idx,
                        });
                    }
                    GestureKind::GoToStart => {
                        msgs.push(Message::GoToStart { viewport_idx });
                    }
                    GestureKind::GoToEnd => {
                        msgs.push(Message::GoToEnd { viewport_idx });
                    }
                    GestureKind::ZoomOut => {
                        msgs.push(Message::CanvasZoom {
                            mouse_ptr: None,
                            delta: 2.0,
                            viewport_idx,
                        });
                    }
                    GestureKind::Cancel => {}
                }
            }
        }
        msgs.push(Message::SetMouseGestureDragStart(None, None));
    }

    fn start_dragging(
        &self,
        pointer_pos_canvas: Option<Pos2>,
        start_location: Pos2,
        ctx: &mut DrawingContext<'_>,
        response: &Response,
        waves: &WaveData,
        viewport_idx: usize,
        y_offset: f32,
    ) {
        let Some(current_location) = pointer_pos_canvas else {
            return;
        };
        let distance = current_location - start_location;
        if distance.length_sq() >= self.user.config.gesture.deadzone {
            if self.add_rectangle == true {
                let current_location =
                    self.clamp_y(current_location, waves.get_content_height(y_offset));
                self.draw_gesture_rectangle(start_location, current_location, ctx);
            } else if self.add_arrow == true {
                self.draw_arrow_line(start_location, current_location, "Add arrow", true, ctx);
            } else {
                match gesture_type(self.user.config.gesture.mapping, distance) {
                    GestureKind::ZoomToFit => self.draw_gesture_line(
                        start_location,
                        current_location,
                        "Zoom to fit",
                        true,
                        ctx,
                    ),
                    GestureKind::ZoomIn => self.draw_zoom_in_gesture(
                        start_location,
                        current_location,
                        response,
                        ctx,
                        waves,
                        viewport_idx,
                        false,
                    ),

                    GestureKind::GoToStart => self.draw_gesture_line(
                        start_location,
                        current_location,
                        "Go to start",
                        true,
                        ctx,
                    ),
                    GestureKind::GoToEnd => {
                        self.draw_gesture_line(
                            start_location,
                            current_location,
                            "Go to end",
                            true,
                            ctx,
                        );
                    }
                    GestureKind::ZoomOut => {
                        self.draw_gesture_line(
                            start_location,
                            current_location,
                            "Zoom out",
                            true,
                            ctx,
                        );
                    }
                    GestureKind::Cancel => {
                        self.draw_gesture_line(
                            start_location,
                            current_location,
                            "Cancel",
                            false,
                            ctx,
                        );
                    }
                }
            }
        } else {
            draw_gesture_help(
                &self.user.config,
                response,
                ctx.painter,
                Some(start_location),
                true,
            );
        }
    }

    fn draw_gesture_rectangle(&self, start: Pos2, end: Pos2, ctx: &mut DrawingContext) {
        let color = self.user.config.theme.annotation_rectangle.color;
        let stroke = Stroke {
            color,
            width: self.user.config.theme.annotation_rectangle.width,
        };

        let start_pos = (ctx.to_screen)(start.x, start.y);
        let end_pos = (ctx.to_screen)(end.x, end.y);

        let temp_rect = emath::Rect::from_two_pos(start_pos, end_pos);

        ctx.painter
            .rect_stroke(temp_rect, 0.0, stroke, egui::StrokeKind::Middle);
    }

    /// Draw the line used by most mouse gestures.
    fn draw_gesture_line(
        &self,
        start: Pos2,
        end: Pos2,
        text: &str,
        active: bool,
        ctx: &mut DrawingContext,
    ) {
        let color = if active {
            self.user.config.theme.gesture.color
        } else {
            self.user.config.theme.gesture.color.gamma_multiply(0.3)
        };
        let stroke = Stroke {
            color,
            width: self.user.config.theme.gesture.width,
        };
        ctx.painter.line_segment(
            [
                (ctx.to_screen)(end.x, end.y),
                (ctx.to_screen)(start.x, start.y),
            ],
            stroke,
        );
        draw_gesture_text(
            ctx,
            (ctx.to_screen)(end.x, end.y),
            text.to_string(),
            &self.user.config.theme,
        );
    }

    /// TODO: fixe code duplication
    fn draw_arrow_line(
        &self,
        start: Pos2,
        end: Pos2,
        text: &str,
        active: bool,
        ctx: &mut DrawingContext,
    ) {
        let color = if active {
            self.user.config.theme.annotation_arrow.color
        } else {
            self.user.config.theme.gesture.color.gamma_multiply(0.3)
        };
        let stroke = Stroke {
            color,
            width: self.user.config.theme.gesture.width,
        };
        ctx.painter.line_segment(
            [
                (ctx.to_screen)(end.x, end.y),
                (ctx.to_screen)(start.x, start.y),
            ],
            stroke,
        );
        draw_gesture_text(
            ctx,
            (ctx.to_screen)(end.x, end.y),
            text.to_string(),
            &self.user.config.theme,
        );
    }

    /// Draw the lines used for the zoom-in gesture.
    #[allow(clippy::too_many_arguments)]
    fn draw_zoom_in_gesture(
        &self,
        start_location: Pos2,
        current_location: Pos2,
        response: &Response,
        ctx: &mut DrawingContext<'_>,
        waves: &WaveData,
        viewport_idx: usize,
        measure: bool,
    ) {
        let stroke = create_gesture_stroke(&self.user.config, measure);
        let height = response.rect.height();
        let width = response.rect.width();
        let segments = [
            ((start_location.x, 0.0), (start_location.x, height)),
            ((current_location.x, 0.0), (current_location.x, height)),
            (
                (start_location.x, start_location.y),
                (current_location.x, start_location.y),
            ),
        ];
        for (start, end) in segments {
            ctx.painter.line_segment(
                [
                    (ctx.to_screen)(start.0, start.1),
                    (ctx.to_screen)(end.0, end.1),
                ],
                stroke,
            );
        }
        let (minx, maxx) = if measure || current_location.x > start_location.x {
            (start_location.x, current_location.x)
        } else {
            (current_location.x, start_location.x)
        };
        let num_timestamps = waves.safe_num_timestamps();
        let start_time = waves.viewports[viewport_idx].as_time_bigint(minx, width, &num_timestamps);
        let end_time = waves.viewports[viewport_idx].as_time_bigint(maxx, width, &num_timestamps);
        let diff_time = &end_time - &start_time;
        let time_formatter = TimeFormatter::new(
            &waves.inner.metadata().timescale,
            &self.user.wanted_timeunit,
            &self.get_time_format(),
        );
        let start_time_str = time_formatter.format(&start_time);
        let end_time_str = time_formatter.format(&end_time);
        let diff_time_str = time_formatter.format(&diff_time);
        draw_gesture_text(
            ctx,
            (ctx.to_screen)(current_location.x, current_location.y),
            if measure {
                format!("{start_time_str} to {end_time_str}\nΔ = {diff_time_str}")
            } else {
                format!("Zoom in: {diff_time_str}\n{start_time_str} to {end_time_str}")
            },
            &self.user.config.theme,
        );
    }

    /// Draw the mouse gesture help window.
    pub(crate) fn mouse_gesture_help(&self, ctx: &Context, msgs: &mut Vec<Message>) {
        let mut open = true;
        Window::new("Mouse gestures")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(
                        "Press middle mouse button (or ctrl+primary mouse button) and drag",
                    ));
                    ui.add_space(20.);
                    let (response, painter) = ui.allocate_painter(
                        Vec2 {
                            x: self.user.config.gesture.size,
                            y: self.user.config.gesture.size,
                        },
                        Sense::empty(),
                    );
                    draw_gesture_help(&self.user.config, &response, &painter, None, false);
                    ui.add_space(10.);
                    ui.separator();
                    if ui.button("Close").clicked() {
                        msgs.push(Message::SetGestureHelpVisible(false));
                    }
                });
            });
        if !open {
            msgs.push(Message::SetGestureHelpVisible(false));
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_measure_widget(
        &self,
        egui_ctx: &Context,
        waves: &WaveData,
        pointer_pos_canvas: Option<Pos2>,
        response: &Response,
        msgs: &mut Vec<Message>,
        ctx: &mut DrawingContext,
        viewport_idx: usize,
    ) {
        if let Some(start_location) = self.measure_start_location {
            let modifiers = egui_ctx.input(|i| i.modifiers);
            if !modifiers.command
                && response.dragged_by(PointerButton::Primary)
                && self.do_measure(&modifiers)
                && let Some(current_location) = pointer_pos_canvas
            {
                self.draw_zoom_in_gesture(
                    start_location,
                    current_location,
                    response,
                    ctx,
                    waves,
                    viewport_idx,
                    true,
                );
            }
            if response.drag_stopped_by(PointerButton::Primary) {
                msgs.push(Message::SetMeasureDragStart(None));
            }
        }
    }
}

/// Draw the "compass" showing the boundaries for different gestures.
fn draw_gesture_help(
    config: &SurferConfig,
    response: &Response,
    painter: &Painter,
    midpoint: Option<Pos2>,
    draw_bg: bool,
) {
    let frame_size = response.rect.size();
    // Compute sizes and coordinates
    let (midx, midy, deltax, deltay) = if let Some(midpoint) = midpoint {
        let halfsize = config.gesture.size * 0.5;
        (midpoint.x, midpoint.y, halfsize, halfsize)
    } else {
        let halfwidth = frame_size.x * 0.5;
        let halfheight = frame_size.y * 0.5;
        (halfwidth, halfheight, halfwidth, halfheight)
    };

    let container_rect = Rect::from_min_size(Pos2::ZERO, frame_size);
    let to_screen = &|x, y| {
        RectTransform::from_to(container_rect, response.rect).transform_pos(Pos2::new(x, y))
    };
    let stroke = Stroke::from(&config.theme.gesture);
    let tan225deltax = TAN_22_5_DEGREES * deltax;
    let tan225deltay = TAN_22_5_DEGREES * deltay;
    let left = midx - deltax;
    let right = midx + deltax;
    let top = midy - deltay;
    let bottom = midy + deltay;
    // Draw background
    if draw_bg {
        let bg_radius = config.gesture.background_radius * deltax;
        painter.circle_filled(
            to_screen(midx, midy),
            bg_radius,
            config
                .theme
                .canvas_colors
                .background
                .gamma_multiply(config.gesture.background_gamma),
        );
    }
    // Draw lines
    let segments = [
        ((left, midy + tan225deltax), (right, midy - tan225deltax)),
        ((left, midy - tan225deltax), (right, midy + tan225deltax)),
        ((midx + tan225deltay, top), (midx - tan225deltay, bottom)),
        ((midx - tan225deltay, top), (midx + tan225deltay, bottom)),
    ];
    for (start, end) in segments {
        painter.line_segment(
            [to_screen(start.0, start.1), to_screen(end.0, end.1)],
            stroke,
        );
    }

    let halfwaytexty_upper = top + (deltay - tan225deltax) * 0.5;
    let halfwaytexty_lower = bottom - (deltay - tan225deltax) * 0.5;

    // Draw commands using a table-driven approach
    let directions = [
        (left, midy, Align2::LEFT_CENTER, config.gesture.mapping.west),
        (
            right,
            midy,
            Align2::RIGHT_CENTER,
            config.gesture.mapping.east,
        ),
        (
            left,
            halfwaytexty_upper,
            Align2::LEFT_CENTER,
            config.gesture.mapping.northwest,
        ),
        (
            right,
            halfwaytexty_upper,
            Align2::RIGHT_CENTER,
            config.gesture.mapping.northeast,
        ),
        (midx, top, Align2::CENTER_TOP, config.gesture.mapping.north),
        (
            left,
            halfwaytexty_lower,
            Align2::LEFT_CENTER,
            config.gesture.mapping.southwest,
        ),
        (
            right,
            halfwaytexty_lower,
            Align2::RIGHT_CENTER,
            config.gesture.mapping.southeast,
        ),
        (
            midx,
            bottom,
            Align2::CENTER_BOTTOM,
            config.gesture.mapping.south,
        ),
    ];

    for (x, y, align, text) in directions {
        painter.text(
            to_screen(x, y),
            align,
            text,
            FontId::default(),
            config.theme.foreground,
        );
    }
}

/// Determine which mouse gesture ([`GestureKind`]) is currently drawn.
fn gesture_type(zones: GestureZones, delta: Vec2) -> GestureKind {
    let tan225x = TAN_22_5_DEGREES * delta.x;
    let tan225y = TAN_22_5_DEGREES * delta.y;
    if delta.x < 0.0 {
        if delta.y.abs() < -tan225x {
            // West
            zones.west
        } else if delta.y < 0.0 && delta.x < tan225y {
            // North west
            zones.northwest
        } else if delta.y > 0.0 && delta.x < -tan225y {
            // South west
            zones.southwest
        } else if delta.y < 0.0 {
            // North
            zones.north
        } else {
            // South
            zones.south
        }
    } else if tan225x > delta.y.abs() {
        // East
        zones.east
    } else if delta.y < 0.0 && delta.x > -tan225y {
        // North east
        zones.northeast
    } else if delta.y > 0.0 && delta.x > tan225y {
        // South east
        zones.southeast
    } else if delta.y < 0.0 {
        // North
        zones.north
    } else {
        // South
        zones.south
    }
}

fn draw_gesture_text(
    ctx: &mut DrawingContext,
    pos: Pos2,
    text: impl ToString,
    theme: &SurferTheme,
) {
    // Translate away from the mouse cursor so the text isn't hidden by it
    let pos = pos + Vec2::new(10.0, -10.0);

    let galley = ctx
        .painter
        .layout_no_wrap(text.to_string(), FontId::default(), theme.foreground);

    ctx.painter.rect(
        galley.rect.translate(pos.to_vec2()).expand(3.0),
        2.0,
        theme.primary_ui_color.background,
        Stroke::default(),
        epaint::StrokeKind::Inside,
    );

    ctx.painter
        .galley(pos, galley, theme.primary_ui_color.foreground);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_zones() -> GestureZones {
        GestureZones {
            north: GestureKind::ZoomToFit,
            northeast: GestureKind::ZoomIn,
            east: GestureKind::GoToEnd,
            southeast: GestureKind::ZoomOut,
            south: GestureKind::Cancel,
            southwest: GestureKind::ZoomOut,
            west: GestureKind::GoToStart,
            northwest: GestureKind::ZoomIn,
        }
    }

    #[test]
    fn gesture_type_cardinal_directions() {
        let zones = default_zones();

        // Pure cardinal directions
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, 0.0)),
            GestureKind::GoToEnd
        ); // East
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, 0.0)),
            GestureKind::GoToStart
        ); // West
        assert_eq!(
            gesture_type(zones, Vec2::new(0.0, -100.0)),
            GestureKind::ZoomToFit
        ); // North
        assert_eq!(
            gesture_type(zones, Vec2::new(0.0, 100.0)),
            GestureKind::Cancel
        ); // South
    }

    #[test]
    fn gesture_type_diagonal_directions() {
        let zones = default_zones();

        // 45-degree diagonals (should be in the diagonal zones)
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, -100.0)),
            GestureKind::ZoomIn
        ); // Northeast
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, 100.0)),
            GestureKind::ZoomOut
        ); // Southeast
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, 100.0)),
            GestureKind::ZoomOut
        ); // Southwest
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, -100.0)),
            GestureKind::ZoomIn
        ); // Northwest
    }

    #[test]
    fn gesture_type_boundary_zones() {
        let zones = default_zones();

        // Test vectors just inside the east zone boundary (tan(22.5°) ≈ 0.414)
        // For east: |y| < tan(22.5°) * x
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, 40.0)),
            GestureKind::GoToEnd
        ); // East
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, -40.0)),
            GestureKind::GoToEnd
        ); // East

        // Test vectors just outside the east zone boundary (should be southeast/northeast)
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, 50.0)),
            GestureKind::ZoomOut
        ); // Southeast
        assert_eq!(
            gesture_type(zones, Vec2::new(100.0, -50.0)),
            GestureKind::ZoomIn
        ); // Northeast
    }

    #[test]
    fn gesture_type_west_boundary_zones() {
        let zones = default_zones();

        // Test vectors just inside the west zone boundary
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, 40.0)),
            GestureKind::GoToStart
        ); // West
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, -40.0)),
            GestureKind::GoToStart
        ); // West

        // Test vectors just outside the west zone boundary
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, 50.0)),
            GestureKind::ZoomOut
        ); // Southwest
        assert_eq!(
            gesture_type(zones, Vec2::new(-100.0, -50.0)),
            GestureKind::ZoomIn
        ); // Northwest
    }
}
