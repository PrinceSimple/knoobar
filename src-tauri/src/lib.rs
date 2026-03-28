mod audio;
mod db;
mod library;
mod playback;

use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::Connection;
use tauri::State;

pub struct AppDb(pub Mutex<Connection>);

pub struct Playback(pub Arc<playback::PlaybackEngine>);

fn db_path() -> std::path::PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("knoobar");
    dir.join("library.db")
}

#[tauri::command]
fn library_add_root(
    path: String,
    label: Option<String>,
    db: State<'_, AppDb>,
) -> Result<i64, String> {
    library::add_root(&db.0.lock(), path, label)
}

#[tauri::command]
fn library_list_roots(db: State<'_, AppDb>) -> Result<Vec<serde_json::Value>, String> {
    library::list_roots(&db.0.lock())
}

#[tauri::command]
fn library_remove_root(root_id: i64, db: State<'_, AppDb>) -> Result<(), String> {
    library::remove_root(&db.0.lock(), root_id)
}

#[tauri::command]
fn library_scan_root(root_id: i64, db: State<'_, AppDb>) -> Result<serde_json::Value, String> {
    let (files_seen, tracks_upserted) = library::scan_root(&db.0.lock(), root_id)?;
    Ok(serde_json::json!({
        "files_seen": files_seen,
        "tracks_upserted": tracks_upserted,
    }))
}

#[tauri::command]
fn library_list_tracks(
    filter: Option<String>,
    db: State<'_, AppDb>,
) -> Result<Vec<serde_json::Value>, String> {
    library::list_tracks(&db.0.lock(), filter)
}

#[tauri::command]
fn library_get_track(id: i64, db: State<'_, AppDb>) -> Result<serde_json::Value, String> {
    library::get_track(&db.0.lock(), id)
}

#[tauri::command]
fn library_update_track_tags(
    id: i64,
    patch: library::TagPatch,
    db: State<'_, AppDb>,
) -> Result<(), String> {
    library::update_track_tags(&db.0.lock(), id, patch)
}

#[tauri::command]
fn library_embed_cover(id: i64, png_bytes: Vec<u8>, db: State<'_, AppDb>) -> Result<(), String> {
    library::embed_cover(&db.0.lock(), id, png_bytes)
}

#[tauri::command]
fn playback_load(path: String, playback: State<'_, Playback>) -> Result<(), String> {
    let engine = playback.inner().0.clone();
    playback::PlaybackEngine::load(&engine, path)
}

#[tauri::command]
fn playback_play(playback: State<'_, Playback>) -> Result<(), String> {
    playback::PlaybackEngine::play(&playback.inner().0)
}

#[tauri::command]
fn playback_pause(playback: State<'_, Playback>) {
    playback.inner().0.pause();
}

#[tauri::command]
fn playback_seek(position_ms: u64, playback: State<'_, Playback>) -> Result<(), String> {
    let engine = playback.inner().0.clone();
    playback::PlaybackEngine::seek(&engine, position_ms)
}

#[tauri::command]
fn playback_set_volume(volume: f32, playback: State<'_, Playback>) {
    playback.inner().0.set_volume(volume);
}

#[tauri::command]
fn playback_get_state(playback: State<'_, Playback>) -> playback::PlaybackStatus {
    playback.inner().0.status_snapshot()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open_database(&db_path()).expect("database");
    let engine = Arc::new(playback::PlaybackEngine::new().expect("playback engine"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppDb(Mutex::new(conn)))
        .manage(Playback(engine))
        .invoke_handler(tauri::generate_handler![
            library_add_root,
            library_list_roots,
            library_remove_root,
            library_scan_root,
            library_list_tracks,
            library_get_track,
            library_update_track_tags,
            library_embed_cover,
            playback_load,
            playback_play,
            playback_pause,
            playback_seek,
            playback_set_volume,
            playback_get_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
