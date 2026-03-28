use rusqlite::Connection;
use std::path::Path;

pub fn open_database(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    migrate(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS library_roots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            label TEXT,
            added_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            root_id INTEGER NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
            path TEXT NOT NULL UNIQUE,
            title TEXT,
            album TEXT,
            artist TEXT,
            album_artist TEXT,
            track_number INTEGER,
            disc_number INTEGER,
            duration_ms INTEGER,
            sample_rate INTEGER,
            channels INTEGER,
            bit_depth INTEGER,
            codec_hint TEXT,
            container TEXT,
            file_mtime INTEGER NOT NULL,
            file_size INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
        "#,
    )?;
    Ok(())
}
