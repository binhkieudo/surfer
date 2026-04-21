use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use camino::Utf8PathBuf;
use rfd::{AsyncFileDialog, FileHandle};
use serde::Deserialize;
#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
use wasm_bindgen::prelude::*;

use crate::SystemState;
use crate::async_util::perform_async_work;
use crate::channels::checked_send_many;
use crate::message::Message;
use crate::transactions::TRANSACTIONS_FILE_EXTENSION;
use crate::wave_source::LoadOptions;

// JS entry points that must be provided by the VS Code extension's webview setup
// (e.g. inside the SURFER_SETUP_HOOKS block or integration.js).
//
// `vscode_show_open_dialog(kind, filters_json)` – asks the extension host to show
//   a native open-file picker.  `kind` is an opaque tag the host echoes back in
//   the inject_message it fires once the user confirms:
//
//   | kind                       | injected Message                               |
//   |----------------------------|------------------------------------------------|
//   | `"waveform_clear"`         | `LoadWaveformFileFromUrl(url, Clear)`          |
//   | `"waveform_keep_available"`| `LoadWaveformFileFromUrl(url, KeepAvailable)`  |
//   | `"waveform_keep_all"`      | `LoadWaveformFileFromUrl(url, KeepAll)`        |
//   | `"command_file"`           | `LoadCommandFileFromUrl(url)`                  |
//
//   `filters_json` is a JSON array of `{"name":str,"extensions":[str]}` objects.
//
// `vscode_show_save_dialog(kind, filters_json)` – same shape, for save pickers.
//   The extension host is responsible for writing the file and sending back any
//   confirmation messages.
#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
#[wasm_bindgen]
extern "C" {
    fn vscode_show_open_dialog(kind: &str, filters_json: &str);
    fn vscode_show_save_dialog(kind: &str, filters_json: &str);
}

#[derive(Debug, Deserialize)]
pub enum OpenMode {
    Open,
    Switch,
}

impl SystemState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn file_dialog_open<F>(
        &mut self,
        title: &'static str,
        filter: (String, Vec<String>),
        messages: F,
    ) where
        F: FnOnce(PathBuf) -> Vec<Message> + Send + 'static,
    {
        let sender = self.channels.msg_sender.clone();

        perform_async_work(async move {
            if let Some(file) = create_file_dialog(filter, title).pick_file().await {
                checked_send_many(&sender, messages(file.path().to_path_buf()));
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn file_dialog_open<F>(
        &mut self,
        title: &'static str,
        filter: (String, Vec<String>),
        messages: F,
    ) where
        F: FnOnce(Vec<u8>) -> Vec<Message> + 'static,
    {
        let sender = self.channels.msg_sender.clone();

        perform_async_work(async move {
            if let Some(file) = create_file_dialog(filter, title).pick_file().await {
                checked_send_many(&sender, messages(file.read().await));
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn file_dialog_save<F, Fut>(
        &mut self,
        title: &'static str,
        filter: (String, Vec<String>),
        messages: F,
    ) where
        F: FnOnce(FileHandle) -> Fut + Send + 'static,
        Fut: Future<Output = Vec<Message>> + Send + 'static,
    {
        let sender = self.channels.msg_sender.clone();

        perform_async_work(async move {
            if let Some(file) = create_file_dialog(filter, title).save_file().await {
                checked_send_many(&sender, messages(file).await);
            }
        });
    }

    #[cfg(all(target_arch = "wasm32", not(feature = "vscode")))]
    pub(crate) fn file_dialog_save<F, Fut>(
        &mut self,
        title: &'static str,
        filter: (String, Vec<String>),
        messages: F,
    ) where
        F: FnOnce(FileHandle) -> Fut + 'static,
        Fut: Future<Output = Vec<Message>> + 'static,
    {
        let sender = self.channels.msg_sender.clone();

        perform_async_work(async move {
            if let Some(file) = create_file_dialog(filter, title).save_file().await {
                checked_send_many(&sender, messages(file).await);
            }
        });
    }

    // For VS Code, `file_dialog_save` delegates to `file_dialog_save_vscode`.
    // Callers in state_file_io.rs that already hold the data to be written should
    // be updated to call `file_dialog_save_vscode` directly (with an appropriate
    // `kind`) so the extension host can persist the file via its own file-system
    // access rights.
    #[cfg(all(target_arch = "wasm32", feature = "vscode"))]
    pub(crate) fn file_dialog_save<F, Fut>(
        &mut self,
        title: &'static str,
        filter: (String, Vec<String>),
        _messages: F,
    ) where
        F: FnOnce(FileHandle) -> Fut + 'static,
        Fut: Future<Output = Vec<Message>> + 'static,
    {
        let _ = title;
        let filters_json = filters_to_json(&filter);
        vscode_show_save_dialog("state_file", &filters_json);
    }

    pub(crate) fn open_file_dialog(&mut self, mode: OpenMode) {
        let load_options: LoadOptions = (mode, self.user.config.behavior.keep_during_reload).into();

        let filter = (
            "Waveform/Transaction-files (*.vcd, *.fst, *.ghw, *.ftr)".to_string(),
            vec![
                "vcd".to_string(),
                "fst".to_string(),
                "ghw".to_string(),
                TRANSACTIONS_FILE_EXTENSION.to_string(),
            ],
        );

        #[cfg(all(target_arch = "wasm32", feature = "vscode"))]
        {
            let kind = match load_options {
                LoadOptions::Clear => "waveform_clear",
                LoadOptions::KeepAvailable => "waveform_keep_available",
                LoadOptions::KeepAll => "waveform_keep_all",
            };
            let filters_json = filters_to_json(&filter);
            vscode_show_open_dialog(kind, &filters_json);
        }

        #[cfg(not(target_arch = "wasm32"))]
        let message = move |file: PathBuf| match Utf8PathBuf::from_path_buf(file.clone()) {
            Ok(utf8_path) => vec![Message::LoadFile(utf8_path, load_options)],
            Err(_) => {
                vec![Message::Error(eyre::eyre!(
                    "File path '{}' contains invalid UTF-8",
                    file.display()
                ))]
            }
        };

        #[cfg(all(target_arch = "wasm32", not(feature = "vscode")))]
        let message = move |file: Vec<u8>| vec![Message::LoadFromData(file, load_options)];

        #[cfg(not(all(target_arch = "wasm32", feature = "vscode")))]
        self.file_dialog_open("Open waveform file", filter, message);
    }

    pub(crate) fn open_command_file_dialog(&mut self) {
        let filter = (
            "Command-file (*.sucl)".to_string(),
            vec!["sucl".to_string()],
        );

        #[cfg(all(target_arch = "wasm32", feature = "vscode"))]
        {
            let filters_json = filters_to_json(&filter);
            vscode_show_open_dialog("command_file", &filters_json);
        }

        #[cfg(not(target_arch = "wasm32"))]
        let message = move |file: PathBuf| match Utf8PathBuf::from_path_buf(file.clone()) {
            Ok(utf8_path) => vec![Message::LoadCommandFile(utf8_path)],
            Err(_) => {
                vec![Message::Error(eyre::eyre!(
                    "File path '{}' contains invalid UTF-8",
                    file.display()
                ))]
            }
        };

        #[cfg(all(target_arch = "wasm32", not(feature = "vscode")))]
        let message = move |file: Vec<u8>| vec![Message::LoadCommandFromData(file)];

        #[cfg(not(all(target_arch = "wasm32", feature = "vscode")))]
        self.file_dialog_open("Open command file", filter, message);
    }

    #[cfg(feature = "python")]
    pub(crate) fn open_python_file_dialog(&mut self) {
        self.file_dialog_open(
            "Open Python translator file",
            ("Python files (*.py)".to_string(), vec!["py".to_string()]),
            |file| match Utf8PathBuf::from_path_buf(file.clone()) {
                Ok(utf8_path) => vec![Message::LoadPythonTranslator(utf8_path)],
                Err(_) => {
                    vec![Message::Error(eyre::eyre!(
                        "File path '{}' contains invalid UTF-8",
                        file.display()
                    ))]
                }
            },
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn create_file_dialog(filter: (String, Vec<String>), title: &'static str) -> AsyncFileDialog {
    AsyncFileDialog::new()
        .set_title(title)
        .add_filter(filter.0, &filter.1)
        .add_filter("All files", &["*"])
}

#[cfg(target_os = "macos")]
fn create_file_dialog(filter: (String, Vec<String>), title: &'static str) -> AsyncFileDialog {
    AsyncFileDialog::new()
        .set_title(title)
        .add_filter(filter.0, &filter.1)
}

/// Serialise a `(name, extensions)` filter pair into the JSON array expected by
/// `vscode_show_open_dialog` / `vscode_show_save_dialog`.
///
/// Example output: `[{"name":"Waveform files","extensions":["vcd","fst"]}]`
#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
fn filters_to_json(filter: &(String, Vec<String>)) -> String {
    let exts = filter
        .1
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{{\"name\":{:?},\"extensions\":[{exts}]}}]", filter.0)
}
