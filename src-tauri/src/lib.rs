use std::sync::Mutex;

use stripant::DataHandler;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

mod stripant;

pub struct AppState {
    data_handler: Option<DataHandler>
}

#[tauri::command]
fn select_data_dir(app_handle: tauri::AppHandle) {
    app_handle.dialog().file().add_filter("data.000", &["000"]).pick_file(move |file_path| {
        if let Some(p) = file_path {
            app_handle.emit("set_data_dir", &p.clone().to_string()).unwrap();
            let state = app_handle.state::<Mutex<AppState>>();
            let mut unlocked = state.lock().unwrap();
            match DataHandler::new(&p.to_string()) {
                Ok(mgr) => { 
                    app_handle.emit("set_filecount", mgr.len()).unwrap();
                    unlocked.data_handler = Some(mgr); 
                }
                Err(err) => { }
            }
        };
    });
}

#[tauri::command]
fn select_export_dir(app_handle: tauri::AppHandle) {
    app_handle.dialog().file().pick_folder(move |file_path| {
        if let Some(p) = file_path { 
            app_handle.emit("set_export_dir", &p.clone().to_string()).unwrap();
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
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {  
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState
                { 
                    data_handler: None 
                }));
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![select_export_dir, select_data_dir, get_filename])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
