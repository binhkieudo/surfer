use std::ops::Shl;

use egui::{Context, Layout, Window};
use emath::Align;
use num::BigUint;
use surfer_translation_types::{VariableEncoding, VariableMeta as TyVariableMeta, VariableValue};

use crate::displayed_item::{DisplayedItem, DisplayedItemRef};
use crate::displayed_item_tree::VisibleItemIndex;
use crate::message::Message;
use crate::translation::TranslatorList;
use crate::wave_container::{ScopeId, VarId, VariableMeta, WaveContainer};
use crate::wave_data::{WaveData, variable_translator};

// ── Dialog state ─────────────────────────────────────────────────────────────

/// Selection mode for the Split Field extraction dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMode {
    /// Specify start bit (LSB) and end bit (MSB).
    StartEnd,
    /// Specify start bit (LSB) and width (number of bits).
    StartWidth,
    /// Specify end bit (MSB) and width (number of bits).
    EndWidth,
}

impl SplitMode {
    fn label(self) -> &'static str {
        match self {
            SplitMode::StartEnd => "Start Bit & End Bit",
            SplitMode::StartWidth => "Start Bit & Width",
            SplitMode::EndWidth => "End Bit & Width",
        }
    }
}

/// State for the Split Field extraction dialog.
#[derive(Debug, Clone)]
pub struct SplitFieldDialogState {
    pub source_vidx: VisibleItemIndex,
    pub source_name: String,
    pub total_bits: u32,
    pub mode: SplitMode,
    pub start_bit_str: String,
    pub end_bit_str: String,
    pub width_str: String,
    pub error: Option<String>,
}

impl SplitFieldDialogState {
    pub fn new(source_vidx: VisibleItemIndex, source_name: String, total_bits: u32) -> Self {
        let default_width = total_bits.to_string();
        let default_end = if total_bits > 0 {
            (total_bits - 1).to_string()
        } else {
            "0".to_string()
        };
        Self {
            source_vidx,
            source_name,
            total_bits,
            mode: SplitMode::StartEnd,
            start_bit_str: "0".to_string(),
            end_bit_str: default_end,
            width_str: default_width,
            error: None,
        }
    }
}

// ── Dialog rendering ─────────────────────────────────────────────────────────

pub fn draw_split_field_dialog(
    state: &mut SplitFieldDialogState,
    ctx: &Context,
    msgs: &mut Vec<Message>,
) {
    let mut open = true;
    Window::new("Split Field")
        .collapsible(false)
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(format!(
                "Signal: {} ({} bits, index 0 = LSB)",
                state.source_name, state.total_bits
            ));
            ui.add_space(6.0);

            // Mode selection
            ui.horizontal(|ui| {
                for mode in [SplitMode::StartEnd, SplitMode::StartWidth, SplitMode::EndWidth] {
                    ui.radio_value(&mut state.mode, mode, mode.label());
                }
            });
            ui.add_space(6.0);

            // Mode-specific fields
            egui::Grid::new("split_field_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| match state.mode {
                    SplitMode::StartEnd => {
                        ui.label("End Bit (MSB):");
                        ui.text_edit_singleline(&mut state.end_bit_str);
                        ui.end_row();
                        ui.label("Start Bit (LSB):");
                        ui.text_edit_singleline(&mut state.start_bit_str);
                        ui.end_row();
                    }
                    SplitMode::StartWidth => {
                        ui.label("Start Bit (LSB):");
                        ui.text_edit_singleline(&mut state.start_bit_str);
                        ui.end_row();
                        ui.label("Width (bits):");
                        ui.text_edit_singleline(&mut state.width_str);
                        ui.end_row();
                    }
                    SplitMode::EndWidth => {
                        ui.label("End Bit (MSB):");
                        ui.text_edit_singleline(&mut state.end_bit_str);
                        ui.end_row();
                        ui.label("Width (bits):");
                        ui.text_edit_singleline(&mut state.width_str);
                        ui.end_row();
                    }
                });

            // Live preview of the resulting range
            if let Ok((start, end)) = compute_range(state) {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("→ bits [{end}:{start}], width = {}", end - start + 1))
                        .color(egui::Color32::GRAY),
                );
            }

            if let Some(err) = &state.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::RED, err.clone());
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Extract").clicked() {
                    match compute_range(state).and_then(|(s, e)| validate_range(state, s, e)) {
                        Ok((start, end)) => {
                            msgs.push(Message::ExtractSplitField {
                                source_vidx: state.source_vidx,
                                start_bit: start,
                                end_bit: end,
                            });
                            msgs.push(Message::CloseSplitFieldDialog);
                        }
                        Err(e) => state.error = Some(e),
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        msgs.push(Message::CloseSplitFieldDialog);
                    }
                });
            });
        });

    if !open {
        msgs.push(Message::CloseSplitFieldDialog);
    }
}

/// Compute (start_bit, end_bit) from the dialog state based on the selected mode.
fn compute_range(state: &SplitFieldDialogState) -> Result<(u32, u32), String> {
    match state.mode {
        SplitMode::StartEnd => {
            let start = state
                .start_bit_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "Start bit must be a non-negative integer".to_string())?;
            let end = state
                .end_bit_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "End bit must be a non-negative integer".to_string())?;
            Ok((start, end))
        }
        SplitMode::StartWidth => {
            let start = state
                .start_bit_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "Start bit must be a non-negative integer".to_string())?;
            let width = state
                .width_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "Width must be a positive integer".to_string())?;
            if width == 0 {
                return Err("Width must be at least 1".to_string());
            }
            Ok((start, start + width - 1))
        }
        SplitMode::EndWidth => {
            let end = state
                .end_bit_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "End bit must be a non-negative integer".to_string())?;
            let width = state
                .width_str
                .trim()
                .parse::<u32>()
                .map_err(|_| "Width must be a positive integer".to_string())?;
            if width == 0 {
                return Err("Width must be at least 1".to_string());
            }
            if width > end + 1 {
                return Err(format!(
                    "Width ({width}) exceeds end bit + 1 ({})",
                    end + 1
                ));
            }
            Ok((end + 1 - width, end))
        }
    }
}

fn validate_range(
    state: &SplitFieldDialogState,
    start: u32,
    end: u32,
) -> Result<(u32, u32), String> {
    if start > end {
        return Err(format!("Start bit ({start}) must be ≤ end bit ({end})"));
    }
    if state.total_bits > 0 && end >= state.total_bits {
        return Err(format!(
            "End bit ({end}) must be < total bits ({})",
            state.total_bits
        ));
    }
    Ok((start, end))
}

// ── Shared value computation ──────────────────────────────────────────────────

/// Get the raw value and bit width for any displayed item at `time`.
///
/// Returns `Some((value, total_bits))` or `None` if the value is not available.
/// Handles Variable, Bus, and SplitField sources recursively.
pub fn compute_source_value_at(
    waves: &WaveData,
    wave_container: &WaveContainer,
    source_ref: DisplayedItemRef,
    time: &BigUint,
    translators: &TranslatorList,
) -> Option<(VariableValue, u32)> {
    let item = waves.displayed_items.get(&source_ref)?;
    match item {
        DisplayedItem::Variable(var) => {
            let meta = wave_container.variable_meta(&var.variable_ref).ok()?;
            let num_bits = meta.num_bits.unwrap_or(1);
            let signal_id = wave_container.signal_id(&var.variable_ref).ok()?;
            if !wave_container.is_signal_loaded(&signal_id) {
                return None;
            }
            let result = wave_container
                .query_variable(&var.variable_ref, time)
                .ok()??;
            let (_, val) = result.current?;
            Some((val, num_bits))
        }

        DisplayedItem::Bus(bus) => {
            let mut concat_value: Option<VariableValue> = None;
            let mut bit_offset: u32 = 0;
            let mut total_bits: u32 = 0;

            for src_ref in &bus.sources {
                let Some(DisplayedItem::Variable(src_var)) =
                    waves.displayed_items.get(src_ref)
                else {
                    continue;
                };
                let Ok(meta) = wave_container.variable_meta(&src_var.variable_ref) else {
                    continue;
                };
                let src_bits = meta.num_bits.unwrap_or(1);
                total_bits += src_bits;

                let query = match wave_container
                    .query_variable(&src_var.variable_ref, time)
                {
                    Ok(Some(q)) => q,
                    _ => {
                        let x_bits = "x".repeat(src_bits as usize);
                        concat_value = Some(concat_with_x(
                            concat_value.take(),
                            x_bits,
                            bit_offset,
                        ));
                        bit_offset += src_bits;
                        continue;
                    }
                };

                if let Some((_, src_val)) = query.current {
                    concat_value =
                        Some(concat_values(concat_value.take(), src_val, src_bits, bit_offset));
                }
                bit_offset += src_bits;
            }

            Some((concat_value?, total_bits))
        }

        DisplayedItem::SplitField(sf) => {
            let (src_val, src_bits) = compute_source_value_at(
                waves,
                wave_container,
                sf.source,
                time,
                translators,
            )?;
            let extracted = extract_bits(src_val, src_bits, sf.start_bit, sf.end_bit);
            Some((extracted, sf.end_bit - sf.start_bit + 1))
        }

        _ => None,
    }
}

/// Concatenate a new source value on top of the accumulator (acc = lower bits already done).
/// `src_val` goes at bit positions `[bit_offset .. bit_offset + src_bits)`.
pub fn concat_values(
    acc: Option<VariableValue>,
    src_val: VariableValue,
    src_bits: u32,
    bit_offset: u32,
) -> VariableValue {
    match (acc, src_val) {
        (None, v) => match v {
            VariableValue::BigUint(u) => VariableValue::BigUint(u),
            VariableValue::String(s) => {
                VariableValue::String(surfer_translation_types::extend_string(&s, src_bits))
            }
        },
        (Some(VariableValue::BigUint(acc_u)), VariableValue::BigUint(v)) => {
            VariableValue::BigUint(acc_u | v.shl(bit_offset as usize))
        }
        (Some(VariableValue::String(acc_s)), VariableValue::BigUint(v)) => {
            let v_bits = format!("{v:0>width$b}", width = src_bits as usize);
            VariableValue::String(v_bits + &acc_s)
        }
        (Some(VariableValue::BigUint(acc_u)), VariableValue::String(s)) => {
            let acc_bits = format!("{acc_u:0>width$b}", width = bit_offset as usize);
            let padded = surfer_translation_types::extend_string(&s, src_bits);
            VariableValue::String(padded + &acc_bits)
        }
        (Some(VariableValue::String(acc_s)), VariableValue::String(s)) => {
            let padded = surfer_translation_types::extend_string(&s, src_bits);
            VariableValue::String(padded + &acc_s)
        }
    }
}

/// Public alias for use from lib.rs
pub fn concat_with_x_pub(
    acc: Option<VariableValue>,
    x_bits: String,
    bit_offset: u32,
) -> VariableValue {
    concat_with_x(acc, x_bits, bit_offset)
}

fn concat_with_x(acc: Option<VariableValue>, x_bits: String, bit_offset: u32) -> VariableValue {
    match acc {
        None => VariableValue::String(x_bits),
        Some(VariableValue::String(s)) => VariableValue::String(x_bits + &s),
        Some(VariableValue::BigUint(u)) => {
            let existing = format!("{u:0>width$b}", width = bit_offset as usize);
            VariableValue::String(x_bits + &existing)
        }
    }
}

/// Extract bits `[start_bit, end_bit]` (inclusive, 0 = LSB) from `value`.
/// `total_source_bits` is the full width of `value`.
pub fn extract_bits(
    value: VariableValue,
    total_source_bits: u32,
    start_bit: u32,
    end_bit: u32,
) -> VariableValue {
    let width = end_bit - start_bit + 1;
    match value {
        VariableValue::BigUint(u) => {
            let mask = (BigUint::from(1u32) << width as usize) - 1u32;
            VariableValue::BigUint((u >> start_bit) & mask)
        }
        VariableValue::String(s) => {
            // String is MSB-first. Bit i (0=LSB) is at position (total_source_bits - 1 - i).
            // Slice [start_bit, end_bit] → string positions [(total_source_bits - 1 - end_bit),
            //                                                  (total_source_bits - start_bit)]
            let padded = surfer_translation_types::extend_string(&s, total_source_bits);
            let str_start = (total_source_bits.saturating_sub(end_bit + 1)) as usize;
            let str_end = (total_source_bits - start_bit) as usize;
            let slice = if str_end <= padded.len() && str_start <= str_end {
                padded[str_start..str_end].to_string()
            } else {
                "x".repeat(width as usize)
            };
            VariableValue::String(slice)
        }
    }
}

/// Recursively collect all signal change timestamps reachable from `source_ref`.
/// Handles Variable, Bus (iterates all sources), and SplitField (recursion through source).
pub fn collect_change_times(
    waves: &WaveData,
    wave_container: &WaveContainer,
    source_ref: DisplayedItemRef,
    times: &mut std::collections::BTreeSet<u64>,
) {
    let Some(item) = waves.displayed_items.get(&source_ref) else {
        return;
    };
    match item {
        DisplayedItem::Variable(var) => {
            let Ok(signal_id) = wave_container.signal_id(&var.variable_ref) else {
                return;
            };
            if !wave_container.is_signal_loaded(&signal_id) {
                return;
            }
            let Ok(accessor) = wave_container.signal_accessor(signal_id) else {
                return;
            };
            for (t, _) in accessor.iter_changes() {
                times.insert(t);
            }
        }
        DisplayedItem::Bus(bus) => {
            let sources = bus.sources.clone();
            for src in sources {
                collect_change_times(waves, wave_container, src, times);
            }
        }
        DisplayedItem::SplitField(sf) => {
            let src = sf.source;
            collect_change_times(waves, wave_container, src, times);
        }
        _ => {}
    }
}

/// Build a synthetic `VariableMeta` for a virtual N-bit vector (used by bus/split-field translator).
pub fn make_virtual_meta(name: &str, num_bits: u32) -> VariableMeta {
    use surfer_translation_types::{ScopeRef as TyScopeRef, VariableRef as TyVariableRef};
    let dummy_scope: TyScopeRef<ScopeId> = TyScopeRef {
        strs: vec![],
        id: ScopeId::None,
    };
    TyVariableMeta {
        var: TyVariableRef {
            path: dummy_scope,
            name: name.to_string(),
            id: VarId::None,
            index: None,
        },
        num_bits: Some(num_bits),
        variable_type: None,
        variable_type_name: None,
        index: None,
        direction: None,
        enum_map: Default::default(),
        encoding: VariableEncoding::BitVector,
    }
}

/// Translate a value using a virtual meta and return the root field string.
pub fn translate_virtual_value(
    name: &str,
    num_bits: u32,
    value: &VariableValue,
    format: Option<&String>,
    translators: &TranslatorList,
) -> Option<String> {
    let meta = make_virtual_meta(name, num_bits);
    let translator = variable_translator(format, &[], translators, || Ok(meta.clone()));
    let translated = translator.translate(&meta, value).ok()?;
    use crate::translation::TranslationResultExt;
    let fields = translated.format_flat(&format.cloned(), &[], translators);
    let root = fields.iter().find(|r| r.names.is_empty())?;
    root.value.as_ref().map(|tv| tv.value.clone())
}

/// Compute total bit width for any item that can be a split-field source.
pub fn get_item_total_bits(
    waves: &WaveData,
    wave_container: &WaveContainer,
    source_ref: DisplayedItemRef,
) -> Option<u32> {
    let item = waves.displayed_items.get(&source_ref)?;
    match item {
        DisplayedItem::Variable(var) => {
            wave_container
                .variable_meta(&var.variable_ref)
                .ok()
                .map(|m| m.num_bits.unwrap_or(1))
        }
        DisplayedItem::Bus(bus) => {
            let total: u32 = bus
                .sources
                .iter()
                .filter_map(|src| {
                    if let Some(DisplayedItem::Variable(v)) = waves.displayed_items.get(src) {
                        wave_container
                            .variable_meta(&v.variable_ref)
                            .ok()
                            .map(|m| m.num_bits.unwrap_or(1))
                    } else {
                        None
                    }
                })
                .sum();
            Some(total)
        }
        DisplayedItem::SplitField(sf) => Some(sf.end_bit - sf.start_bit + 1),
        _ => None,
    }
}
