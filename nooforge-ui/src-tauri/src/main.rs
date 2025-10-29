// nooforge-ui/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lib;

use lib::commands::{ingest_file, ingest_text, rag, search};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ingest_text,
            ingest_file,
            rag,
            search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
