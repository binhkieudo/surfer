use egui_remixicon::icons;
use serde::{Deserialize, Serialize};

use crate::{
    BigInt, SystemState,
    annotation::{Annotatable, Annotation},
    view::DrawingContext,
    viewport::Viewport,
    wave_data::WaveData,
};
#[derive(Clone)]
pub struct CommentMessage {
    pub user: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: egui::Id,
    pub rect: egui::Rect,
    pub color: egui::Color32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub time_anchor: BigInt,
    pub x_anchor: f32,
    pub y_anchor: f32,
    pub x_size: f32,
    pub y_size: f32,
    pub annotation_id: egui::Id,
    pub name: String,

    #[serde(skip)]
    pub message_chain: Vec<CommentMessage>,
    pub new_text: String,
}

impl egui::Widget for &mut Comment {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // 1. Setup Layout Area
        // We create a temporary rectangle based on the stored position but with a large
        // height to allow the content to expand downwards without clipping during layout.
        let mut layout_rect = self.rect;
        layout_rect.set_height(2000.0);

        // Scope the UI to this specific rectangle so child elements align correctly
        let inner = ui.scope_builder(egui::UiBuilder::new().max_rect(layout_rect), |ui| {
            let line_start = self.rect.left_top();
            let anchor_pos = egui::pos2(self.x_anchor, self.y_anchor);

            // 2. Connector Line
            // Draw a dashed line from the comment box to the target it's referencing
            ui.painter().add(egui::Shape::dashed_line(
                &[line_start, anchor_pos],
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                4.0,
                2.0,
            ));

            // 3. Background Placeholders
            // We reserve slots in the painter order now, but fill them later after
            // we know the final size of the content.
            let background_fill_shape = ui.painter().add(egui::Shape::Noop);
            let background_stroke_shape = ui.painter().add(egui::Shape::Noop);

            // 4. Collapsible State
            // Load whether this specific comment is open or closed from egui's persistent storage
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.make_persistent_id(self.id),
                false,
            );

            // --- 1. Header Logic ---
            let header = ui.horizontal(|ui| {
                // Custom toggle button using a chat icon instead of the default arrow
                state.show_toggle_button(ui, |ui, _openness, response| {
                    let icon = icons::QUESTION_ANSWER_FILL;
                    ui.painter().text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::proportional(20.0),
                        ui.visuals().text_color(),
                    );
                });

                // Only show the title label if the header is expanded
                if state.is_open() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&self.name).strong());
                }
            });

            let mut text_response_rect = egui::Rect::ZERO;

            // --- 2. Body Logic (Message List & Input) ---
            state.show_body_unindented(ui, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;

                    // Top border line
                    ui.painter().add(egui::Shape::line_segment(
                        [ui.cursor().left_top(), ui.cursor().right_top()],
                        egui::Stroke::new(0.5, egui::Color32::WHITE),
                    ));

                    // Render existing comments in the chain
                    for comment in &self.message_chain {
                        ui.label(egui::RichText::new(&comment.text).size(14.0));
                        ui.painter().add(egui::Shape::line_segment(
                            [ui.cursor().left_top(), ui.cursor().right_top()],
                            egui::Stroke::new(0.5, egui::Color32::WHITE),
                        ));
                    }

                    ui.add_space(4.0);

                    // Text Input Field
                    let text_response = ui.add(
                        egui::TextEdit::multiline(&mut self.new_text)
                            .desired_rows(1)
                            .desired_width(self.x_size)
                            .hint_text("Comment..."),
                    );

                    // Store rect to prevent dragging the whole widget while typing
                    text_response_rect = text_response.rect;

                    // Handle "Enter" key to submit new comment
                    if text_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let clean_text = self.new_text.trim().to_string();
                        if !clean_text.is_empty() {
                            self.message_chain.push(CommentMessage {
                                user: "user".to_string(),
                                text: clean_text,
                            });
                        }
                        self.new_text = String::new(); // Clear input after submission
                    }
                });
            });

            // --- 3. Size Sync ---
            // Calculate how much space the UI actually took up
            let final_rect = ui.min_rect();
            let content_height = final_rect.height();

            // Auto-expand the saved height if the content grows (e.g. adding messages)
            if self.y_size < content_height {
                self.y_size = content_height;
            }

            let background_rect = final_rect.expand(5.0);

            // --- 4. Delayed Background Rendering ---
            // Now that we have the final background_rect size, we fill the Noop shapes from earlier
            ui.painter().set(
                background_fill_shape,
                egui::Shape::rect_filled(background_rect, 4.0, egui::Color32::BLACK),
            );
            ui.painter().set(
                background_stroke_shape,
                egui::Shape::rect_stroke(
                    background_rect,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                    egui::StrokeKind::Middle,
                ),
            );

            // --- 5. Interaction Logic (Resize & Drag) ---

            // Define a small interactive handle in the bottom-right corner for resizing
            let handle_rect = egui::Rect::from_min_max(
                background_rect.max - egui::vec2(15.0, 15.0),
                background_rect.max,
            );

            // Logic for resizing the widget (South-East Corner)
            let se_res = ui.interact(handle_rect, self.id.with("se_res"), egui::Sense::drag());

            if se_res.hovered() || se_res.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }

            if se_res.dragged() {
                let delta = se_res.drag_delta();
                // Update width/height based on mouse movement
                self.x_size += delta.x;
                self.y_size += delta.y;
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }

            // Draw a small icon to indicate the resize handle
            ui.painter().text(
                handle_rect.center(),
                egui::Align2::CENTER_CENTER,
                icons::DRAG_MOVE_2_FILL,
                egui::FontId::proportional(10.0),
                ui.visuals().text_color().linear_multiply(0.4),
            );

            // Logic for moving the entire widget
            // We disable dragging if the user is currently resizing or interacting with the text box
            let can_drag_body = !se_res.dragged()
                && !text_response_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default());

            let body_res = ui.interact(
                background_rect,
                self.id.with("body_res"),
                if can_drag_body {
                    egui::Sense::drag()
                } else {
                    egui::Sense::hover()
                },
            );

            if body_res.dragged() {
                let delta = body_res.drag_delta();
                self.x_offset += delta.x;
                self.y_offset += delta.y;
            }

            // Logic for Header interactions (Click to toggle, Drag to move)
            let header_res = ui.interact(
                header.response.rect,
                self.id.with("head_res"),
                egui::Sense::click_and_drag(),
            );

            if header_res.clicked() {
                state.toggle(ui);
                state.store(ui.ctx());
            } else if header_res.dragged() {
                self.x_offset += header_res.drag_delta().x;
                self.y_offset += header_res.drag_delta().y;
            }

            // Return a combined response so the parent UI knows if any part was touched
            se_res | body_res | header_res
        });

        inner.inner
    }
}

impl WaveData {
    pub fn draw_comments(
        &self,
        ui: &mut egui::Ui,
        viewport: &Viewport,
        ctx: &DrawingContext,
        comments: &mut Vec<Comment>,
        annotation_offset: f32,
    ) {
        for comment in comments {
            let annotation = self
                .annotations
                .iter()
                .find(|a| a.get_id() == comment.annotation_id);
            if !annotation.is_none() {
                if annotation.unwrap().is_visible() {
                    comment.name = annotation.unwrap().get_name();
                    comment.time_anchor = annotation.unwrap().get_time_at_end();
                    let y = annotation.unwrap().get_lowest_y_pos(self);
                    match annotation {
                        Some(Annotation::Arrow(_a)) => {
                            match _a.head_mode {
                                crate::arrow::ArrowHeadMode::End => {}
                                crate::arrow::ArrowHeadMode::Double => {
                                    comment.time_anchor = _a.get_time_at_start()
                                }
                            }
                            let pos = (ctx.to_screen)(0., y);
                            comment.y_anchor = y - (pos.y - y);
                        }
                        Some(Annotation::Rect(_r)) => {
                            comment.y_anchor = y + annotation_offset;
                        }
                        None => todo!(),
                    }
                    let num_timestamps = self.safe_num_timestamps();

                    // x-coordinate
                    comment.rect.min.x = viewport.pixel_from_time(
                        &comment.time_anchor,
                        ctx.cfg.canvas_size.x,
                        &num_timestamps,
                    ) + comment.x_offset;

                    comment.rect.max.x = viewport.pixel_from_time(
                        &comment.time_anchor,
                        ctx.cfg.canvas_size.x,
                        &num_timestamps,
                    ) + comment.x_offset
                        + comment.x_size;

                    // y-coordinate
                    comment.rect.min.y = comment.y_anchor + comment.y_offset;
                    comment.rect.max.y = comment.y_anchor + comment.y_size + comment.y_offset;

                    comment.x_anchor = viewport.pixel_from_time(
                        &comment.time_anchor,
                        ctx.cfg.canvas_width,
                        &num_timestamps,
                    );

                    comment.rect.min = (ctx.to_screen)(comment.rect.min.x, comment.rect.min.y);
                    comment.rect.max = (ctx.to_screen)(comment.rect.max.x, comment.rect.max.y);

                    // for the dotted line
                    let anchor_screen = (ctx.to_screen)(
                        viewport.pixel_from_time(
                            &comment.time_anchor,
                            ctx.cfg.canvas_width,
                            &num_timestamps,
                        ),
                        comment.y_anchor,
                    );

                    // Assign these to the clone so the Widget knows where to draw the line
                    comment.x_anchor = anchor_screen.x;
                    comment.y_anchor = anchor_screen.y;

                    ui.add(comment);
                }
            }
        }
    }
}
