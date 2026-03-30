use crate::{
    Message, SystemState, annotation::Annotatable, annotation_list, graphics::GraphicsY, view::DrawingContext, viewport::Viewport, wave_data::WaveData
};
use egui::{Align, Color32, Context, Key, Layout, Ui};
use egui_remixicon::icons;

#[derive(Clone)]
pub struct AnnotationList {}

impl Default for AnnotationList {
    fn default() -> Self {
        Self {}
    }
}

pub const DEFAULT_GROUP_NAME: &str = "Ungrouped";

impl AnnotationList {}

use std::collections::BTreeSet;

impl WaveData {
    pub fn draw_annotation_list(
        &self,
        ui: &mut egui::Ui,
        waves: &WaveData,
        msgs: &mut Vec<Message>,
    ) {
        ui.style_mut()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width = 0.5;

        ui.horizontal(|ui| {
            ui.allocate_space(egui::vec2(ui.available_width() - 30.0, 0.0));
            if ui.button(icons::CLOSE_LARGE_LINE).clicked() {
                msgs.push(Message::SetAnnotationlistVisible());
            }
        });

        ui.vertical_centered(|ui| {
            ui.heading("Annotation List");
            ui.label("Your annotations will be displayed here.");
        });

        ui.add_space(8.0);
        ui.separator();

       

            // --- Create Group UI (Using egui Temp Memory) --- TEMPORATY FIX
            ui.label(egui::RichText::new("Manage Groups").small().strong());
            ui.horizontal(|ui| {
                let input_id = ui.make_persistent_id("group_input_buffer");
                let mut buffer =
                    ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

                let text_edit_res = ui.add(
                    egui::TextEdit::singleline(&mut buffer)
                        .hint_text("Type group name...")
                        .desired_width(ui.available_width() - 160.0),
                );

                // handle focusing of the text area when user clicks elsewhere, enables shortcuts.
                let focus_id = ui.make_persistent_id("group_input_focus_init");
                let has_focused =
                    ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(false));

                if !has_focused {
                    text_edit_res.request_focus();
                    ui.data_mut(|d| d.insert_temp(focus_id, true));
                }

                ui.data_mut(|d| d.insert_temp(input_id, buffer.clone()));

                // create group when user press enter
                if text_edit_res.ctx.input(|i| i.key_pressed(Key::Enter)) {
                    if !buffer.is_empty() {
                        //TODO: Code duplication, move to new func.
                        let group_name = buffer.trim().to_string();
                        if !waves.available_groups.contains(&group_name) {
                            msgs.push(Message::CreateAnnotationGroup(group_name));
                            ui.data_mut(|d| d.insert_temp(input_id, String::new()));
                        }
                        ui.data_mut(|d| d.insert_temp(input_id, String::new()));
                        // Keep focus here so users can type the next group immediately
                        text_edit_res.request_focus();
                    }
            // delete group when user press plus button
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
        }});

            ui.add_space(4.0);
            ui.separator();

            // --- Scrollable List ---
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut all_groups = BTreeSet::new();
                    // for annotation in &waves.annotations {
                    //     if let Some(ref g) = annotation.group_name() {
                    //         all_groups.insert(g.clone());
                    //     }
                    // }

                    // Include empty groups from the state's list
                    for g in &self.available_groups {
                        all_groups.insert(g.clone());
                    }

                    for group_name in all_groups {
                        self.render_group_section(
                            ui,
                            Some(group_name.to_string()),
                            msgs,
                            waves,
                        );
                    }

                    self.render_group_section(ui, None, msgs, waves);
                });
        }

    fn render_group_section(
        &self,
        ui: &mut Ui,
        group_filter: Option<String>,
        msgs: &mut Vec<Message>,
        waves: &WaveData
    ) {
        let display_name = group_filter
            .clone()
            .unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
        let items: Vec<_> = waves
            .annotations
            .iter()
            .filter(|r| r.get_group_name() == group_filter)
            .collect();

        if group_filter.is_none() && items.is_empty() {
            return;
        }

        // Determine if the group is "mostly visible" or "mostly hidden" to pick the icon
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

                for (annotation) in items {
                    let annotation_id = annotation.get_id();
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);

                        /* // show what type of annotering
                        if ui.button(icons::DELETE_BIN_LINE).clicked() {

                            }
                        */

                        // Editable Name Logic
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

                        // Group Selector
                        let mut current_group = annotation.get_group_name();

                        // Assign a unique ID for this specific dropdown
                        let combo_id = ui.make_persistent_id(("move_group", annotation_id));

                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text("") // Keep it slim
                            .width(20.0) // Small width just for the arrow/icon
                            .show_ui(ui, |ui| {
                                // Option: Ungrouped
                                if ui
                                    .selectable_value(
                                        &mut current_group,
                                        None,
                                        format!("None ({})", DEFAULT_GROUP_NAME),
                                    )
                                    .clicked()
                                {
                                    msgs.push(Message::UpdateAnnotationGroup(annotation_id, None));
                                }

                                // Options: Existing groups
                                for group in &self.available_groups {
                                    if ui
                                        .selectable_value(
                                            &mut current_group,
                                            Some(group.clone()),
                                            group,
                                        )
                                        .clicked()
                                    {
                                        msgs.push(Message::UpdateAnnotationGroup(
                                            annotation_id,
                                            Some(group.clone()),
                                        ));
                                    }
                                }
                            })
                            .response
                            .on_hover_text("Change Group");

                        // Buttons on the right
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(icons::DELETE_BIN_LINE).clicked() {
                                msgs.push(Message::RemoveAnnotation(annotation_id));
                            }

                            let vis_icon = if !annotation.is_visible() {
                                icons::EYE_OFF_LINE
                            } else {
                                icons::EYE_LINE
                            };
                            if ui.button(vis_icon).clicked() {
                                msgs.push(Message::ToggleAnnotationVisiblility(annotation_id));
                            }

                            if ui.button(icons::SEARCH_LINE).clicked() {
                                msgs.push(Message::GoToAnnotationPosition(
                                    annotation_id,
                                    self.last_active_viewport_idx,
                                ));
                            }
                        });
                    });
                }
            });
        ui.separator();
        ui.add_space(4.0);
    }
}
