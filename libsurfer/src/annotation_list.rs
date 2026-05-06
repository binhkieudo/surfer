use crate::{Message, annotation::Annotatable, time::TimeFormatter, wave_data::WaveData};
use egui::{Align, Color32, Key, Layout, Ui};
use egui_remixicon::icons;

#[derive(Clone)]
pub struct AnnotationList {}

impl Default for AnnotationList {
    fn default() -> Self {
        Self {}
    }
}

const DEFAULT_GROUP_NAME: &str = "Ungrouped";
const TIME_FONT_SIZE: f32 = 11.;
const DEFAULT_SPACE: f32 = 4.;
const WIDTH_CONSTRAINT: f32 = 30.;

impl AnnotationList {}

use std::collections::BTreeSet;

impl WaveData {
    pub fn draw_annotation_list(
        &self,
        ui: &mut Ui,
        msgs: &mut Vec<Message>,
        time_formatter: &TimeFormatter,
    ) {
        ui.style_mut()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width = 0.5;

        ui.horizontal(|ui| {
            ui.allocate_space(egui::vec2(ui.available_width() - WIDTH_CONSTRAINT, 0.0));
            if ui.button(icons::CLOSE_LARGE_LINE).clicked() {
                msgs.push(Message::ToggleAnnotationlistVisibility());
            }
        });

        ui.vertical_centered(|ui| {
            ui.heading("Annotation List");
            ui.label("Your annotations will be displayed here.");
        });

        ui.add_space(DEFAULT_SPACE * 2.);
        ui.separator();

        // Create Group UI (Using egui Temp Memory)
        ui.horizontal(|ui| {
            ui.add_space(DEFAULT_SPACE * 2.);
            ui.label(egui::RichText::new("Manage Groups").small().strong());
        });
        ui.horizontal(|ui| {
            ui.add_space(DEFAULT_SPACE * 2.);
            let input_id = ui.make_persistent_id("group_input_buffer");
            let mut buffer = ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

            let text_edit_res = ui.add(
                egui::TextEdit::singleline(&mut buffer)
                    .hint_text("Type group name...")
                    .desired_width(ui.available_width() - 160.0),
            );

            // Handle focusing of the text area when user clicks elsewhere, enables shortcuts.
            let focus_id = ui.make_persistent_id("group_input_focus_init");
            let has_focused = ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(false));

            if !has_focused {
                text_edit_res.request_focus();
                ui.data_mut(|d| d.insert_temp(focus_id, true));
            }

            ui.data_mut(|d| d.insert_temp(input_id, buffer.clone()));

            // Create group when user press enter
            if text_edit_res.ctx.input(|i| i.key_pressed(Key::Enter)) {
                if !buffer.is_empty() {
                    msgs.push(Message::CreateAnnotationGroup(buffer.trim().to_string()));
                    ui.data_mut(|d| d.insert_temp(input_id, String::new()));
                    // Keep focus here so users can type the next group immediately
                    text_edit_res.request_focus();
                }
            }

            // Create group when user press plus button
            if ui
                .button(icons::ADD_LINE)
                .on_hover_text("Create Group")
                .clicked()
            {
                if !buffer.is_empty() {
                    msgs.push(Message::CreateAnnotationGroup(buffer.trim().to_string()));
                    ui.data_mut(|d| d.insert_temp(input_id, String::new()));
                }
            }

            // Delete group when user press plus button
            if ui
                .button(icons::DELETE_BIN_LINE)
                .on_hover_text("Delete Group")
                .clicked()
            {
                if !buffer.is_empty() {
                    msgs.push(Message::DeleteAnnotationGroup(buffer.trim().to_string()));
                    ui.data_mut(|d| d.insert_temp(input_id, String::new()));
                }
            }
        });

        ui.add_space(DEFAULT_SPACE);
        ui.separator();

        // Scrollable List
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut all_groups = BTreeSet::new();

                // include empty groups from the state's list
                for g in &self.annotation_groups {
                    all_groups.insert(g.clone());
                }

                for group_name in all_groups {
                    self.render_group_section(
                        ui,
                        Some(group_name.to_string()),
                        msgs,
                        time_formatter,
                    );
                }

                self.render_group_section(ui, None, msgs, time_formatter);
            });
    }

    fn render_group_section(
        &self,
        ui: &mut Ui,
        group_filter: Option<String>,
        msgs: &mut Vec<Message>,
        time_formatter: &TimeFormatter,
    ) {
        let display_name = group_filter
            .clone()
            .unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
        let items: Vec<_> = self
            .annotations
            .iter()
            .filter(|r| r.get_group_name() == group_filter)
            .collect();

        if group_filter.is_none() && items.is_empty() {
            return;
        }

        // Determine if the group is has any visible annotations to pick the icon
        let any_visible = items.iter().any(|r| r.is_visible());
        let group_icon = if any_visible {
            icons::EYE_LINE
        } else {
            icons::EYE_OFF_LINE
        };

        // Create the header manually to inject the button
        let id = ui.make_persistent_id(&display_name);
        let state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);

        state
            .show_header(ui, |ui| {
                ui.label(format!("{} ({})", display_name, items.len()));

                // Push everything else to the right
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if display_name != DEFAULT_GROUP_NAME.to_string() {
                        if ui
                            .button(icons::DELETE_BIN_LINE)
                            .on_hover_text("Delete all annotations in this group")
                            .clicked()
                        {
                            msgs.push(Message::DeleteAllAnnotationInGroup(
                                group_filter.clone().unwrap_or("".to_string()),
                            ));
                        }
                    }
                    if ui
                        .button(group_icon)
                        .on_hover_text("Toggle visibility for all in this group")
                        .clicked()
                    {
                        msgs.push(Message::SetGroupVisibility(
                            group_filter.clone(),
                            !any_visible,
                        ));
                    }
                });
            })
            .body(|ui| {
                if items.is_empty() {
                    ui.weak("  No items");
                }

                for annotation in items {
                    let annotation_id = annotation.get_id();
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);

                        // Editable name logic
                        let editing_id = ui.make_persistent_id(("editing_name", annotation_id));
                        let is_editing =
                            ui.data(|d| d.get_temp::<bool>(editing_id).unwrap_or(false));

                        let current_name = annotation.get_name();

                        if is_editing {
                            let mut buffer = ui.data_mut(|d| {
                                d.get_temp::<String>(editing_id)
                                    .unwrap_or_else(|| current_name.clone())
                            });

                            let res = ui
                                .add(egui::TextEdit::singleline(&mut buffer).desired_width(120.0));

                            if res.has_focus() {
                                ui.data_mut(|d| d.insert_temp(editing_id, buffer.clone()));
                            }

                            // Save on Enter or if focus is lost
                            if res.lost_focus()
                                || (res.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                            {
                                msgs.push(Message::UpdateAnnotationName(
                                    annotation_id,
                                    buffer.trim().to_string(),
                                ));
                                ui.data_mut(|d| d.insert_temp(editing_id, false));
                            }

                            // Request focus once when we start editing
                            if ui.data(|d| {
                                d.get_temp::<bool>(
                                    ui.make_persistent_id(("focus_req", annotation_id)),
                                )
                                .unwrap_or(true)
                            }) {
                                res.request_focus();
                                ui.data_mut(|d| {
                                    d.insert_temp(
                                        ui.make_persistent_id(("focus_req", annotation_id)),
                                        false,
                                    )
                                });
                            }
                        } else {
                            // Display the name as a clickable label
                            let response = ui.add(
                                egui::Label::new(egui::RichText::new(&current_name).strong())
                                    .sense(egui::Sense::click()),
                            );
                            if response.clicked() {
                                ui.data_mut(|d| d.insert_temp(editing_id, true));
                                ui.data_mut(|d| {
                                    d.insert_temp(
                                        ui.make_persistent_id(("focus_req", annotation_id)),
                                        true,
                                    )
                                });
                            }
                            response.on_hover_text("Click to rename");
                        }

                        let show_comment_icon = if annotation.show_comments() {
                            icons::ARROW_DOWN_S_LINE
                        } else {
                            icons::ARROW_RIGHT_S_LINE
                        };

                        if ui
                            .button(show_comment_icon)
                            .on_hover_text("Show comments")
                            .clicked()
                        {
                            msgs.push(Message::ToggleAnnotationListShowComments(annotation_id));
                        }

                        // Group Selector
                        let current_group = annotation.get_group_name();

                        ui.menu_button(icons::FOLDER_TRANSFER_LINE, |ui| {
                            if ui
                                .selectable_label(current_group.is_none(), DEFAULT_GROUP_NAME)
                                .clicked()
                            {
                                msgs.push(Message::UpdateAnnotationGroup(annotation_id, None));
                                ui.close();
                            }

                            for group in &self.annotation_groups {
                                if ui
                                    .selectable_label(current_group.as_ref() == Some(group), group)
                                    .clicked()
                                {
                                    msgs.push(Message::UpdateAnnotationGroup(
                                        annotation_id,
                                        Some(group.clone()),
                                    ));
                                    ui.close();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Change Group");

                        // Buttons on the right of individual annotations
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(icons::DELETE_BIN_LINE)
                                .on_hover_text("Delete annotation")
                                .clicked()
                            {
                                msgs.push(Message::RemoveAnnotation(annotation_id));
                            }

                            let vis_icon = if !annotation.is_visible() {
                                icons::EYE_OFF_LINE
                            } else {
                                icons::EYE_LINE
                            };
                            if ui
                                .button(vis_icon)
                                .on_hover_text("Toggle visibility")
                                .clicked()
                            {
                                msgs.push(Message::ToggleAnnotationVisiblility(annotation_id));
                            }

                            if ui
                                .button(icons::SEARCH_LINE)
                                .on_hover_text("Go to annotation")
                                .clicked()
                            {
                                msgs.push(Message::GoToAnnotationPosition(
                                    annotation_id,
                                    self.last_active_viewport_idx,
                                ));
                            }
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(annotation.get_time_info(time_formatter))
                                .size(TIME_FONT_SIZE)
                                .color(Color32::LIGHT_GRAY),
                        )
                    });

                    // Show comments for this annotation
                    if annotation.show_comments() {
                        let messages = annotation.get_messages();
                        for c in messages {
                            let mut line_left = ui.cursor().left_top();
                            line_left.x += 16.;
                            ui.painter().add(egui::Shape::line_segment(
                                [line_left, ui.cursor().right_top()],
                                egui::Stroke::new(0.5, egui::Color32::WHITE),
                            ));
                            ui.horizontal(|ui| {
                                ui.add_space(18.0); // Indent comments
                                ui.vertical(|ui| {
                                    ui.add_space(DEFAULT_SPACE / 2.);
                                    ui.set_max_width(ui.available_width() - WIDTH_CONSTRAINT);
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                                    ui.add(egui::Label::new(c.text.as_str()).wrap());
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let response = ui.add_sized(
                                        egui::Vec2::new(10.0, 10.0),
                                        egui::Button::new(icons::DELETE_BIN_LINE),
                                    );

                                    if response.on_hover_text("Delete message").clicked() {
                                        msgs.push(Message::RemoveCommentMessage(annotation.get_id(), c.id));
                                    }
                                });
                            });
                        }
                    }
                }
            });
        ui.separator();
        ui.add_space(DEFAULT_SPACE);
    }
}
