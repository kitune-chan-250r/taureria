// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
// #[tauri::command]
// fn greet(name: &str) -> String {
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }
mod feature;
use feature::{library, viewer};
use s_zip::StreamingZipReader;
// use std::{collections::HashMap, sync::Mutex};
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct ZipArchive {
    reader: StreamingZipReader,
    entries: Vec<String>, // ファイル名順にソート済み
}

pub struct AppState(Mutex<HashMap<String, ZipArchive>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(HashMap::new()))) // initialize empty state
        .invoke_handler(tauri::generate_handler![
            // viewer::open_zip,
            viewer::get_image,
            viewer::open_viewer_window,
            library::create,
            library::update,
            library::list,
            library::contents_list,
            library::delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
