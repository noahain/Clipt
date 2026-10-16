pub mod media;
pub mod migrate;

use chrono::NaiveDate;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

pub const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS days (
  date       TEXT PRIMARY KEY,
  label      TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT
);
CREATE TABLE IF NOT EXISTS clips (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  date       TEXT NOT NULL REFERENCES days(date),
  timestamp  TEXT NOT NULL,
  kind       TEXT NOT NULL CHECK (kind IN ('text','image','files')),
  text       TEXT,
  blob_hash  TEXT,
  file_paths TEXT
);
CREATE INDEX IF NOT EXISTS idx_clips_date_ts ON clips(date, timestamp);
CREATE TABLE IF NOT EXISTS pins (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  kind       TEXT NOT NULL CHECK (kind IN ('clip','day')),
  ref        TEXT NOT NULL,
  note       TEXT NOT NULL DEFAULT '',
  preview    TEXT NOT NULL DEFAULT '',
  date       TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  UNIQUE(kind, ref)
);
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DayInfo {
    pub date: String,
    pub label: String,
    pub clip_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Clip {
    pub id: i64,
    pub date: String,
    pub timestamp: String,
    pub kind: String,
    pub text: Option<String>,
    pub media_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub id: i64,
    pub kind: String,
    pub r#ref: String,
    pub note: String,
    pub preview: String,
    pub date: String,
    pub created_at: String,
}

pub fn timestamp_now() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string()
}

pub fn date_from_timestamp(ts: &str) -> String {
    if ts.len() >= 10 {
        ts[..10].to_string()
    } else {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
}

pub fn valid_date(date: &str) -> bool {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '1')",
        [],
    )?;
    Ok(())
}

pub fn open_or_create(db_path: &Path) -> anyhow::Result<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let attempt = open_with_pragmas(db_path);
    match attempt {
        Ok(conn) => match init_schema(&conn) {
            Ok(()) => Ok(conn),
            Err(e) => {
                drop(conn);
                quarantine(db_path)?;
                log::warn!("clipt.db schema init failed, recreated fresh: {e}");
                let fresh = open_with_pragmas(db_path)?;
                init_schema(&fresh)?;
                Ok(fresh)
            }
        },
        Err(e) => {
            quarantine(db_path)?;
            log::warn!("clipt.db unopenable, recreated fresh: {e}");
            let fresh = open_with_pragmas(db_path)?;
            init_schema(&fresh)?;
            Ok(fresh)
        }
    }
}

fn open_with_pragmas(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
    Ok(conn)
}

fn quarantine(db_path: &Path) -> anyhow::Result<()> {
    if db_path.exists() {
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let corrupt = db_path.with_extension(format!("db.corrupt-{ts}"));
        let _ = std::fs::rename(db_path, &corrupt);
        for suffix in ["-wal", "-shm", "-journal"] {
            let sidecar_str = format!("{}{suffix}", db_path.display());
            let sidecar = Path::new(&sidecar_str);
            let _ = std::fs::remove_file(sidecar);
        }
    }
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| {
        r.get(0)
    })
    .ok()
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        (key, value),
    )?;
    Ok(())
}

fn ensure_day(conn: &Connection, date: &str, created_at: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO days (date, label, created_at) VALUES (?1, '', ?2)",
        (date, created_at),
    )?;
    Ok(())
}

const CLIP_COLUMNS: &str = "id, date, timestamp, kind, text, blob_hash, file_paths";

struct ClipRow {
    id: i64,
    date: String,
    timestamp: String,
    kind: String,
    text: Option<String>,
    blob_hash: Option<String>,
    file_paths_json: Option<String>,
}

fn clip_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClipRow> {
    Ok(ClipRow {
        id: r.get(0)?,
        date: r.get(1)?,
        timestamp: r.get(2)?,
        kind: r.get(3)?,
        text: r.get(4)?,
        blob_hash: r.get(5)?,
        file_paths_json: r.get(6)?,
    })
}

impl ClipRow {
    fn into_clip(self, media_dir: &Path) -> Clip {
        let media_path = self
            .blob_hash
            .map(|h| media::media_path_for(media_dir, &h).to_string_lossy().to_string());
        let file_paths = self
            .file_paths_json
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
        Clip {
            id: self.id,
            date: self.date,
            timestamp: self.timestamp,
            kind: self.kind,
            text: self.text,
            media_path,
            file_paths,
        }
    }
}

pub fn save_text_clip(
    conn: &Connection,
    timestamp: &str,
    text: &str,
    media_dir: &Path,
) -> rusqlite::Result<Clip> {
    let date = date_from_timestamp(timestamp);
    ensure_day(conn, &date, timestamp)?;
    conn.execute(
        "INSERT INTO clips (date, timestamp, kind, text) VALUES (?1, ?2, 'text', ?3)",
        (date.clone(), timestamp, text),
    )?;
    Ok(ClipRow {
        id: conn.last_insert_rowid(),
        date,
        timestamp: timestamp.to_string(),
        kind: "text".to_string(),
        text: Some(text.to_string()),
        blob_hash: None,
        file_paths_json: None,
    }
    .into_clip(media_dir))
}

pub fn save_image_clip(
    conn: &Connection,
    timestamp: &str,
    blob_hash: &str,
    media_dir: &Path,
) -> rusqlite::Result<Clip> {
    let date = date_from_timestamp(timestamp);
    ensure_day(conn, &date, timestamp)?;
    conn.execute(
        "INSERT INTO clips (date, timestamp, kind, blob_hash) VALUES (?1, ?2, 'image', ?3)",
        (date.clone(), timestamp, blob_hash),
    )?;
    Ok(ClipRow {
        id: conn.last_insert_rowid(),
        date,
        timestamp: timestamp.to_string(),
        kind: "image".to_string(),
        text: None,
        blob_hash: Some(blob_hash.to_string()),
        file_paths_json: None,
    }
    .into_clip(media_dir))
}

pub fn save_files_clip(
    conn: &Connection,
    timestamp: &str,
    file_paths: &[String],
    media_dir: &Path,
) -> rusqlite::Result<Clip> {
    let date = date_from_timestamp(timestamp);
    ensure_day(conn, &date, timestamp)?;
    let json = serde_json::to_string(file_paths).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO clips (date, timestamp, kind, file_paths) VALUES (?1, ?2, 'files', ?3)",
        (date.clone(), timestamp, json.clone()),
    )?;
    Ok(ClipRow {
        id: conn.last_insert_rowid(),
        date,
        timestamp: timestamp.to_string(),
        kind: "files".to_string(),
        text: None,
        blob_hash: None,
        file_paths_json: Some(json),
    }
    .into_clip(media_dir))
}

pub fn get_days(conn: &Connection) -> rusqlite::Result<Vec<DayInfo>> {
    let mut stmt = conn.prepare(
        "SELECT d.date, d.label, d.created_at, d.updated_at, COUNT(c.id)
         FROM days d LEFT JOIN clips c ON c.date = d.date
         GROUP BY d.date ORDER BY d.date DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(DayInfo {
            date: r.get(0)?,
            label: r.get(1)?,
            created_at: r.get(2)?,
            updated_at: r.get(3)?,
            clip_count: r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_day_data(
    conn: &Connection,
    date: &str,
    media_dir: &Path,
) -> rusqlite::Result<(Vec<Clip>, DayInfo)> {
    let meta: Option<DayInfo> = conn
        .query_row(
            "SELECT d.date, d.label, d.created_at, d.updated_at, COUNT(c.id)
             FROM days d LEFT JOIN clips c ON c.date = d.date WHERE d.date = ?1 GROUP BY d.date",
            [date],
            |r| {
                Ok(DayInfo {
                    date: r.get(0)?,
                    label: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                    clip_count: r.get(4)?,
                })
            },
        )
        .ok();
    let meta = meta.unwrap_or(DayInfo {
        date: date.to_string(),
        label: String::new(),
        clip_count: 0,
        created_at: None,
        updated_at: None,
    });
    let mut stmt = conn.prepare(&format!(
        "SELECT {CLIP_COLUMNS} FROM clips WHERE date = ?1 ORDER BY timestamp ASC, id ASC"
    ))?;
    let rows = stmt.query_map([date], clip_row)?;
    let mut clips = Vec::new();
    for row in rows {
        clips.push(row?.into_clip(media_dir));
    }
    Ok((clips, meta))
}

pub fn update_day_label(conn: &Connection, date: &str, label: &str) -> bool {
    if !valid_date(date) {
        return false;
    }
    let trimmed = label.trim();
    let truncated: String = trimmed.chars().take(200).collect();
    let now = timestamp_now();
    let day_midnight = format!("{date}T00:00:00.000000");
    let res = (|| -> rusqlite::Result<()> {
        ensure_day(conn, date, &day_midnight)?;
        conn.execute(
            "UPDATE days SET label = ?1, updated_at = ?2 WHERE date = ?3",
            (&truncated, &now, date),
        )?;
        Ok(())
    })();
    res.is_ok()
}

fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '%' || c == '_' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub fn search_clips(
    conn: &Connection,
    query: &str,
    date: Option<&str>,
    limit: Option<i64>,
    media_dir: &Path,
) -> rusqlite::Result<Vec<Clip>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let lim = match limit {
        None => 100,
        Some(n) if n <= 0 => 100,
        Some(n) => n.min(500),
    };
    let pattern = format!("%{}%", escape_like(query));
    let mut stmt = conn.prepare(&format!(
        "SELECT {CLIP_COLUMNS}
         FROM clips WHERE kind = 'text' AND (?1 IS NULL OR date = ?1) AND text LIKE ?2 ESCAPE '\\'
         ORDER BY timestamp DESC, id DESC LIMIT ?3"
    ))?;
    let rows = stmt.query_map(rusqlite::params![date, pattern, lim], clip_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?.into_clip(media_dir));
    }
    Ok(out)
}

pub fn get_clip_by_id(
    conn: &Connection,
    id: i64,
    media_dir: &Path,
) -> rusqlite::Result<Option<Clip>> {
    let row = conn
        .query_row(
            &format!("SELECT {CLIP_COLUMNS} FROM clips WHERE id = ?1"),
            [id],
            clip_row,
        )
        .ok();
    Ok(row.map(|r| r.into_clip(media_dir)))
}

pub fn last_clip(conn: &Connection, media_dir: &Path) -> rusqlite::Result<Option<Clip>> {
    let row = conn
        .query_row(
            &format!("SELECT {CLIP_COLUMNS} FROM clips ORDER BY id DESC LIMIT 1"),
            [],
            clip_row,
        )
        .ok();
    Ok(row.map(|r| r.into_clip(media_dir)))
}

pub fn delete_clip(
    conn: &Connection,
    id: i64,
    media_dir: &Path,
) -> rusqlite::Result<bool> {
    let blob: Option<String> = conn
        .query_row("SELECT blob_hash FROM clips WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .ok()
        .flatten();
    let n = conn.execute("DELETE FROM clips WHERE id = ?1", [id])?;
    if n == 0 {
        return Ok(false);
    }
    if let Some(hash) = blob {
        let _ = media::gc_if_unreferenced(conn, media_dir, &hash);
    }
    conn.execute(
        "DELETE FROM pins WHERE kind = 'clip' AND ref = ?1",
        [id.to_string()],
    )?;
    Ok(true)
}

pub fn get_pins(conn: &Connection) -> rusqlite::Result<Vec<Pin>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, ref, note, preview, date, created_at FROM pins
         ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Pin {
            id: r.get(0)?,
            kind: r.get(1)?,
            r#ref: r.get(2)?,
            note: r.get(3)?,
            preview: r.get(4)?,
            date: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn set_pin(
    conn: &Connection,
    kind: &str,
    ref_id: &str,
    note: &str,
    preview: &str,
    date: &str,
) -> rusqlite::Result<Pin> {
    let note: String = note.chars().take(200).collect();
    let preview: String = preview.chars().take(120).collect();
    conn.execute(
        "INSERT INTO pins (kind, ref, note, preview, date, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(kind, ref) DO UPDATE SET note = excluded.note, preview = excluded.preview",
        (kind, ref_id, &note, &preview, date, timestamp_now()),
    )?;
    conn.query_row(
        "SELECT id, kind, ref, note, preview, date, created_at FROM pins WHERE kind = ?1 AND ref = ?2",
        (kind, ref_id),
        |r| {
            Ok(Pin {
                id: r.get(0)?,
                kind: r.get(1)?,
                r#ref: r.get(2)?,
                note: r.get(3)?,
                preview: r.get(4)?,
                date: r.get(5)?,
                created_at: r.get(6)?,
            })
        },
    )
}

pub fn remove_pin(conn: &Connection, kind: &str, ref_id: &str) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "DELETE FROM pins WHERE kind = ?1 AND ref = ?2",
        (kind, ref_id),
    )?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        (conn, dir)
    }

    #[test]
    fn crud_and_day_derivation() {
        let (conn, dir) = test_conn();
        let media = dir.path();
        let ts = "2026-10-15T23:59:59.123456";
        save_text_clip(&conn, ts, "hello", media).unwrap();
        let days = get_days(&conn).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].date, "2026-10-15");
        assert_eq!(days[0].clip_count, 1);
        let (clips, meta) = get_day_data(&conn, "2026-10-15", media).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(meta.clip_count, 1);
        let (empty, missing) = get_day_data(&conn, "2000-01-01", media).unwrap();
        assert!(empty.is_empty());
        assert_eq!(missing.clip_count, 0);
        assert!(missing.created_at.is_none());
    }

    #[test]
    fn timestamp_format_always_microseconds() {
        let ts = timestamp_now();
        assert!(ts.len() >= 26);
        assert!(ts.contains('T'));
        let frac = ts.rsplit('.').next().unwrap();
        assert_eq!(frac.len(), 6);
        assert_eq!(date_from_timestamp(&ts).len(), 10);
    }

    #[test]
    fn label_validation_truncation() {
        let (conn, _dir) = test_conn();
        assert!(!update_day_label(&conn, "not-a-date", "x"));
        assert!(!update_day_label(&conn, "2026-13-99", "x"));
        let long = "a".repeat(500);
        assert!(update_day_label(&conn, "2026-01-02", &long));
        let days = get_days(&conn).unwrap();
        assert_eq!(days[0].label.chars().count(), 200);
        assert!(update_day_label(&conn, "2026-01-03", "  hi  "));
        let days = get_days(&conn).unwrap();
        let row = days.iter().find(|d| d.date == "2026-01-03").unwrap();
        assert_eq!(row.label, "hi");
    }

    #[test]
    fn search_escaping_limit_ordering() {
        let (conn, dir) = test_conn();
        let media = dir.path();
        save_text_clip(&conn, "2026-01-01T10:00:00.000001", "100% legit", media).unwrap();
        save_text_clip(&conn, "2026-01-01T11:00:00.000002", "under_score", media).unwrap();
        save_text_clip(&conn, "2026-01-01T12:00:00.000003", "back\\slash", media).unwrap();
        save_text_clip(&conn, "2026-01-01T13:00:00.000004", "other", media).unwrap();
        save_text_clip(&conn, "2026-01-02T09:00:00.000005", "second day o", media).unwrap();
        let r = search_clips(&conn, "100%", None, None, media).unwrap();
        assert_eq!(r.len(), 1);
        let r = search_clips(&conn, "_", None, None, media).unwrap();
        assert_eq!(r.len(), 1);
        let r = search_clips(&conn, "\\", None, None, media).unwrap();
        assert_eq!(r.len(), 1);
        assert!(search_clips(&conn, "   ", None, None, media).unwrap().is_empty());
        let r = search_clips(&conn, "o", None, Some(1), media).unwrap();
        assert_eq!(r.len(), 1);
        let r = search_clips(&conn, "o", None, Some(9999), media).unwrap();
        assert!(r.len() <= 500);
        let r = search_clips(&conn, "o", Some("2026-01-01"), None, media).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|c| c.date == "2026-01-01"));
        let r = search_clips(&conn, "o", None, None, media).unwrap();
        assert_eq!(r.len(), 3);
        if r.len() >= 2 {
            assert!(r[0].timestamp >= r[1].timestamp);
        }
    }

    #[test]
    fn pins_crud_upsert_and_clip_cleanup() {
        let (conn, dir) = test_conn();
        let media = dir.path();
        save_text_clip(&conn, "2026-01-01T10:00:00.000001", "hello pin", media).unwrap();
        let p = set_pin(&conn, "clip", "1", "my note", "hello pin", "2026-01-01").unwrap();
        assert_eq!(p.note, "my note");
        set_pin(&conn, "day", "2026-01-01", "", "Jan 1", "2026-01-01").unwrap();
        assert_eq!(get_pins(&conn).unwrap().len(), 2);
        let p = set_pin(&conn, "clip", "1", "renamed", "hello pin", "2026-01-01").unwrap();
        assert_eq!(p.note, "renamed");
        assert_eq!(get_pins(&conn).unwrap().len(), 2);
        assert!(delete_clip(&conn, 1, media).unwrap());
        let pins = get_pins(&conn).unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].kind, "day");
        assert!(remove_pin(&conn, "day", "2026-01-01").unwrap());
        assert!(get_pins(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_removes_blob_when_unreferenced() {
        let (conn, dir) = test_conn();
        let media = dir.path();
        let h = media::store_png(media, b"pngdata").unwrap();
        save_image_clip(&conn, "2026-01-01T10:00:00.000001", &h, media).unwrap();
        save_image_clip(&conn, "2026-01-01T10:01:00.000002", &h, media).unwrap();
        assert!(media::media_path_for(media, &h).exists());
        assert!(delete_clip(&conn, 1, media).unwrap());
        assert!(media::media_path_for(media, &h).exists());
        assert!(delete_clip(&conn, 2, media).unwrap());
        assert!(!media::media_path_for(media, &h).exists());
        assert!(!delete_clip(&conn, 999, media).unwrap());
    }
}
