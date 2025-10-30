// nooforge-ui/src-tauri/src/dnd.rs
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn set_file_drop_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    // Берём главное окно (или по id, если у тебя другое имя)
    let Some(win) = app.get_webview_window("main") else {
        return Err("main window not found".into());
    };
    // win.set_file_drop_enabled(enabled)
    win.set_enabled(enabled)
        .map_err(|e| format!("set_file_drop_enabled failed: {e}"))?;
    Ok(())
}
