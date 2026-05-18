use ecolor::Color32;
use egui::{Context, Layout, Window};
use emath::Align;
use num::BigInt;

use crate::{displayed_item_tree::VisibleItemIndex, message::Message};

/// State for the Find Value dialog
#[derive(Debug, Clone)]
pub struct FindValueState {
    pub vidx: VisibleItemIndex,
    pub search_input: String,
    pub occurrences: Vec<BigInt>,
    pub current_idx: Option<usize>,
    pub not_found: bool,
}

impl FindValueState {
    pub fn new(vidx: VisibleItemIndex) -> Self {
        Self {
            vidx,
            search_input: String::new(),
            occurrences: vec![],
            current_idx: None,
            not_found: false,
        }
    }
}

pub fn draw_find_value_dialog(
    state: &mut FindValueState,
    ctx: &Context,
    msgs: &mut Vec<Message>,
) {
    let mut open = true;
    Window::new("Find Value")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Value:");
                let response = ui.text_edit_singleline(&mut state.search_input);

                let pressed_enter = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if ui.button("Find").clicked() || pressed_enter {
                    msgs.push(Message::FindValueSearch(state.search_input.clone()));
                }
            });

            if state.not_found {
                ui.add_space(4.0);
                ui.colored_label(Color32::RED, "Not found value");
            }

            if let Some(idx) = state.current_idx {
                let total = state.occurrences.len();
                ui.add_space(4.0);
                ui.label(format!("{} / {} occurrences", idx + 1, total));
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let has_results = !state.occurrences.is_empty();

                if ui
                    .add_enabled(has_results, egui::Button::new("Previous"))
                    .clicked()
                {
                    msgs.push(Message::FindValuePrevious);
                }
                if ui
                    .add_enabled(has_results, egui::Button::new("Next"))
                    .clicked()
                {
                    msgs.push(Message::FindValueNext);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Exit").clicked() {
                        msgs.push(Message::CloseFindValueDialog);
                    }
                });
            });
        });

    if !open {
        msgs.push(Message::CloseFindValueDialog);
    }
}
