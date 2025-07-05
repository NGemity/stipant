use std::sync::{Arc, Mutex};

use config::AppConfig;
use stipant::{DataHandler, RZData};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

mod config;
mod stipant;

pub struct AppState {
    data_handler: Option<DataHandler>,
    config: Arc<AppConfig>
}

#[tauri::command]
fn select_data_dir(app_handle: tauri::AppHandle) {
    app_handle.dialog().file().pick_folder(move |file_path| {
        if let Some(p) = file_path {
            let state = app_handle.state::<Mutex<AppState>>();
            let mut unlocked = state.lock().unwrap();
            match DataHandler::new(&p.to_string(), unlocked.config.clone()) {
                Ok(mgr) => {
                    app_handle.emit("set_filecount", mgr.len()).unwrap();
                    app_handle
                        .emit("set_data_dir", &p.clone().to_string())
                        .unwrap();
                    unlocked.data_handler = Some(mgr);
                }
                Err(err) => {
                    app_handle
                        .dialog()
                        .message(err.to_string())
                        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                        .show(|_|{})
                }
            }
        };
    });
}

#[tauri::command]
fn select_export_dir(app_handle: tauri::AppHandle) {
    app_handle.dialog().file().pick_folder(move |file_path| {
        if let Some(p) = file_path {
            let state = app_handle.state::<Mutex<AppState>>();
            let mut unlocked = state.lock().unwrap();
            if unlocked.data_handler.is_some() {
                let a = unlocked.data_handler.as_mut().unwrap();
                a.set_export_dir(&p.clone().to_string());
                app_handle
                    .emit("set_export_dir", &p.clone().to_string())
                    .unwrap();
            }
        };
    });
}

#[tauri::command]
fn get_filename(app_handle: tauri::AppHandle, filename: String) {
    let state = app_handle.state::<Mutex<AppState>>();
    let unlocked = state.lock().unwrap();
    if let Some(rz_file) = &unlocked.data_handler {
        if let Some(entry) = rz_file.get_entry_by_name(&filename) {
            app_handle.emit("set_data", entry).unwrap();
            return;
        }
        let empty_display = Arc::new(RZData::default());
        app_handle.emit("set_data", empty_display).unwrap();
    }
}

#[tauri::command]
fn dump_filename(app_handle: tauri::AppHandle, filename: String) {
    let state = app_handle.state::<Mutex<AppState>>();
    let unlocked = state.lock().unwrap();
    if let Some(rz_file) = &unlocked.data_handler {
        if let Err(err) = rz_file.dump_by_filename(&filename) {
            app_handle
                .dialog()
                .message(err.to_string())
                .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                .show(|_|{})
        }
    }
}

#[tauri::command]
fn dump_all(app_handle: tauri::AppHandle) {
    let state = app_handle.state::<Mutex<AppState>>();
    let unlocked = state.lock().unwrap();
    if let Some(rz_file) = &unlocked.data_handler {
        rz_file.dump_all();
        app_handle
            .dialog()
            .message("Done unpacking files!")
            .kind(tauri_plugin_dialog::MessageDialogKind::Info)
            .show(|_|{})
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = Arc::new(AppConfig::new().unwrap_or_default());
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState { data_handler: None, config: cfg }));
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_export_dir,
            select_data_dir,
            get_filename,
            dump_filename,
            dump_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
