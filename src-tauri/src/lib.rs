mod core;
mod algorithms;
mod detection;
mod api;

use tauri::command;
use std::path::PathBuf;

// 重新导出API函数
pub use api::{get_image_paths, find_duplicates, get_supported_algorithms, get_detection_stats, get_folder_stats};
pub use core::types::{HashAlgorithm, DuplicateGroup, DuplicateDetectionRequest};

/// 应用入口函数
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_image_paths,
            find_duplicates,
            get_supported_algorithms,
            get_detection_stats,
            get_folder_stats
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("运行应用时出错");
}