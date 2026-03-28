use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::read_from_path;
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagItem};
use rusqlite::{params, Connection};
use walkdir::WalkDir;

use crate::audio::probe;

const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "wav", "wave", "ogg", "oga", "opus", "m4a", "mp4", "aac", "alac", "mka",
    "webm", "wv", "wma", "asf", "dsf", "dff", "dsdiff", "mpc",
];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            AUDIO_EXTENSIONS.iter().any(|&x| x == e)
        })
        .unwrap_or(false)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn add_root(conn: &Connection, path: String, label: Option<String>) -> Result<i64, String> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("empty path".into());
    }
    conn.execute(
        "INSERT INTO library_roots (path, label, added_at) VALUES (?1, ?2, ?3)",
        params![path, label, now_ms()],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_roots(conn: &Connection) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare("SELECT id, path, label, added_at FROM library_roots ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "path": r.get::<_, String>(1)?,
                "label": r.get::<_, Option<String>>(2)?,
                "added_at": r.get::<_, i64>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn remove_root(conn: &Connection, root_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM tracks WHERE root_id = ?1", params![root_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM library_roots WHERE id = ?1", params![root_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn scan_root(conn: &Connection, root_id: i64) -> Result<(usize, usize), String> {
    let root_path: String = conn
        .query_row(
            "SELECT path FROM library_roots WHERE id = ?1",
            params![root_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let root = PathBuf::from(root_path);
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }

    let mut files_seen = 0usize;
    let mut upserted = 0usize;

    for entry in WalkDir::new(&root).follow_links(true).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !is_audio_file(path) {
            continue;
        }
        files_seen += 1;
        if index_track(conn, root_id, path).is_ok() {
            upserted += 1;
        }
    }
    Ok((files_seen, upserted))
}

fn index_track(conn: &Connection, root_id: i64, path: &Path) -> Result<(), String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let size = meta.len() as i64;
    let path_str = path.to_string_lossy().to_string();

    let (title, album, artist, album_artist, track_number, disc_number) = read_lofty_tags(path);

    let probed = probe::probe_file(path).unwrap_or(probe::ProbedInfo {
        sample_rate: None,
        channels: None,
        duration_ms: None,
        codec_hint: None,
        container: None,
    });
    let mut duration_ms = probed.duration_ms.map(|d| d as i64);
    if duration_ms.is_none() || duration_ms == Some(0) {
        if let Some(d) = crate::audio::ffmpeg_cli::ffprobe_duration_ms(path) {
            duration_ms = Some(d as i64);
        }
    }

    let sample_rate = probed.sample_rate.map(|v| v as i64);
    let channels = probed.channels.map(|v| v as i64);
    let codec_hint = probed.codec_hint.clone();
    let container = probed.container.clone();

    conn.execute(
        r#"INSERT INTO tracks (
            root_id, path, title, album, artist, album_artist, track_number, disc_number,
            duration_ms, sample_rate, channels, bit_depth, codec_hint, container, file_mtime, file_size
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(path) DO UPDATE SET
            root_id = excluded.root_id,
            title = excluded.title,
            album = excluded.album,
            artist = excluded.artist,
            album_artist = excluded.album_artist,
            track_number = excluded.track_number,
            disc_number = excluded.disc_number,
            duration_ms = excluded.duration_ms,
            sample_rate = excluded.sample_rate,
            channels = excluded.channels,
            bit_depth = excluded.bit_depth,
            codec_hint = excluded.codec_hint,
            container = excluded.container,
            file_mtime = excluded.file_mtime,
            file_size = excluded.file_size"#,
        params![
            root_id,
            path_str,
            title,
            album,
            artist,
            album_artist,
            track_number,
            disc_number,
            duration_ms,
            sample_rate,
            channels,
            None::<i64>,
            codec_hint,
            container,
            mtime,
            size,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_lofty_tags(path: &Path) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
) {
    let Ok(probe) = Probe::open(path) else {
        return (None, None, None, None, None, None);
    };
    let Ok(file) = probe.read() else {
        return (None, None, None, None, None, None);
    };
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let Some(tag) = tag else {
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string());
        return (name, None, None, None, None, None);
    };

    let title = tag
        .get_string(&ItemKey::TrackTitle)
        .map(str::to_string)
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()));
    let album = tag.get_string(&ItemKey::AlbumTitle).map(str::to_string);
    let artist = tag
        .get_string(&ItemKey::TrackArtist)
        .map(str::to_string)
        .or_else(|| tag.get_string(&ItemKey::AlbumArtist).map(str::to_string));
    let album_artist = tag.get_string(&ItemKey::AlbumArtist).map(str::to_string);
    let track_number = tag.track().map(|n| n as i64);
    let disc_number = tag.disk().map(|n| n as i64);
    (title, album, artist, album_artist, track_number, disc_number)
}

pub fn list_tracks(conn: &Connection, filter: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let filter = filter.unwrap_or_default().trim().to_lowercase();

    let map_row = |r: &rusqlite::Row<'_>| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "root_id": r.get::<_, i64>(1)?,
            "path": r.get::<_, String>(2)?,
            "title": r.get::<_, Option<String>>(3)?,
            "album": r.get::<_, Option<String>>(4)?,
            "artist": r.get::<_, Option<String>>(5)?,
            "album_artist": r.get::<_, Option<String>>(6)?,
            "track_number": r.get::<_, Option<i64>>(7)?,
            "disc_number": r.get::<_, Option<i64>>(8)?,
            "duration_ms": r.get::<_, Option<i64>>(9)?,
            "sample_rate": r.get::<_, Option<i64>>(10)?,
            "channels": r.get::<_, Option<i64>>(11)?,
            "bit_depth": r.get::<_, Option<i64>>(12)?,
            "codec_hint": r.get::<_, Option<String>>(13)?,
            "container": r.get::<_, Option<String>>(14)?,
            "file_mtime": r.get::<_, i64>(15)?,
            "file_size": r.get::<_, i64>(16)?,
        }))
    };

    let mut out = Vec::new();
    if filter.is_empty() {
        let mut stmt = conn
            .prepare(
                r#"SELECT id, root_id, path, title, album, artist, album_artist, track_number, disc_number,
                duration_ms, sample_rate, channels, bit_depth, codec_hint, container, file_mtime, file_size
                FROM tracks ORDER BY album COLLATE NOCASE NULLS LAST, disc_number NULLS LAST, track_number NULLS LAST, path"#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let like = format!("%{}%", filter);
        let mut stmt = conn
            .prepare(
                r#"SELECT id, root_id, path, title, album, artist, album_artist, track_number, disc_number,
                duration_ms, sample_rate, channels, bit_depth, codec_hint, container, file_mtime, file_size
                FROM tracks WHERE
                lower(ifnull(title,'')) LIKE ?1 OR lower(ifnull(album,'')) LIKE ?1
                OR lower(ifnull(artist,'')) LIKE ?1 OR lower(ifnull(album_artist,'')) LIKE ?1
                ORDER BY album COLLATE NOCASE NULLS LAST, disc_number NULLS LAST, track_number NULLS LAST, path"#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![like], map_row).map_err(|e| e.to_string())?;
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
    }
    Ok(out)
}

pub fn get_track(conn: &Connection, id: i64) -> Result<serde_json::Value, String> {
    conn.query_row(
        r#"SELECT id, root_id, path, title, album, artist, album_artist, track_number, disc_number,
        duration_ms, sample_rate, channels, bit_depth, codec_hint, container, file_mtime, file_size
        FROM tracks WHERE id = ?1"#,
        params![id],
        |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "root_id": r.get::<_, i64>(1)?,
                "path": r.get::<_, String>(2)?,
                "title": r.get::<_, Option<String>>(3)?,
                "album": r.get::<_, Option<String>>(4)?,
                "artist": r.get::<_, Option<String>>(5)?,
                "album_artist": r.get::<_, Option<String>>(6)?,
                "track_number": r.get::<_, Option<i64>>(7)?,
                "disc_number": r.get::<_, Option<i64>>(8)?,
                "duration_ms": r.get::<_, Option<i64>>(9)?,
                "sample_rate": r.get::<_, Option<i64>>(10)?,
                "channels": r.get::<_, Option<i64>>(11)?,
                "bit_depth": r.get::<_, Option<i64>>(12)?,
                "codec_hint": r.get::<_, Option<String>>(13)?,
                "container": r.get::<_, Option<String>>(14)?,
                "file_mtime": r.get::<_, i64>(15)?,
                "file_size": r.get::<_, i64>(16)?,
            }))
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct TagPatch {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_artist: String,
    #[serde(default)]
    pub track_number: Option<u32>,
    #[serde(default)]
    pub disc_number: Option<u32>,
}

fn ensure_primary_tag(tagged: &mut TaggedFile, path: &Path) -> Result<(), String> {
    if tagged.primary_tag().is_some() {
        return Ok(());
    }
    let ft = FileType::from_path(path).ok_or_else(|| "could not guess tag type for file".to_string())?;
    tagged.insert_tag(Tag::new(ft.primary_tag_type()));
    Ok(())
}

pub fn update_track_tags(conn: &Connection, id: i64, patch: TagPatch) -> Result<(), String> {
    let path: String = conn
        .query_row("SELECT path FROM tracks WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let path = PathBuf::from(path);

    let mut tagged = read_from_path(&path).map_err(|e| e.to_string())?;
    ensure_primary_tag(&mut tagged, &path)?;
    let tag = tagged.primary_tag_mut().ok_or_else(|| "no tag".to_string())?;

    tag.set_title(patch.title);
    tag.set_album(patch.album);
    tag.set_artist(patch.artist);
    tag.push(TagItem::new(
        ItemKey::AlbumArtist,
        ItemValue::Text(patch.album_artist),
    ));
    if let Some(n) = patch.track_number {
        tag.set_track(n);
    }
    if let Some(n) = patch.disc_number {
        tag.set_disk(n);
    }

    tagged
        .save_to_path(&path, WriteOptions::default())
        .map_err(|e| e.to_string())?;

    let root_id: i64 = conn
        .query_row("SELECT root_id FROM tracks WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    index_track(conn, root_id, &path)?;
    Ok(())
}

pub fn embed_cover(conn: &Connection, id: i64, png_bytes: Vec<u8>) -> Result<(), String> {
    let path: String = conn
        .query_row("SELECT path FROM tracks WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let path = PathBuf::from(path);

    let mime = if png_bytes.len() >= 2 && png_bytes[0] == 0xff && png_bytes[1] == 0xd8 {
        MimeType::Jpeg
    } else {
        MimeType::Png
    };
    let picture = Picture::new_unchecked(PictureType::CoverFront, Some(mime), None, png_bytes);

    let mut tagged = read_from_path(&path).map_err(|e| e.to_string())?;
    ensure_primary_tag(&mut tagged, &path)?;
    let tag = tagged.primary_tag_mut().ok_or_else(|| "no tag".to_string())?;

    tag.push_picture(picture);
    tagged
        .save_to_path(&path, WriteOptions::default())
        .map_err(|e| e.to_string())?;

    let root_id: i64 = conn
        .query_row("SELECT root_id FROM tracks WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    index_track(conn, root_id, &path)?;
    Ok(())
}

