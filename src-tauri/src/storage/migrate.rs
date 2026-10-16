use chrono::NaiveDate;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

use crate::storage::{get_meta, set_meta};

fn valid_day_name(name: &str) -> bool {
    NaiveDate::parse_from_str(name, "%Y-%m-%d").is_ok()
}

pub fn run(
    conn: &Connection,
    app_root: &Path,
    _media_dir: &Path,
) -> anyhow::Result<()> {
    if get_meta(conn, "legacy_import_done").is_some() {
        return Ok(());
    }
    let days_dir = app_root.join("Days");
    if !days_dir.exists() {
        set_meta(conn, "legacy_import_done", "1")?;
        return Ok(());
    }
    let entries = match std::fs::read_dir(&days_dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("migration: cannot list Days dir: {e}");
            set_meta(conn, "legacy_import_done", "1")?;
            return Ok(());
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !valid_day_name(name) {
            continue;
        }
        import_one_day(conn, &path, name);
    }
    set_meta(conn, "legacy_import_done", "1")?;
    Ok(())
}

fn import_one_day(conn: &Connection, folder: &Path, date: &str) {
    let metadata_path = folder.join("metadata.json");
    let (label, created_at, updated_at): (String, String, Option<String>) =
        match std::fs::read_to_string(&metadata_path) {
            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => (
                    v.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                    v.get("created_at")
                        .and_then(|c| c.as_str())
                        .unwrap_or(&format!("{date}T00:00:00.000000"))
                        .to_string(),
                    v.get("updated_at").and_then(|u| u.as_str()).map(|s| s.to_string()),
                ),
                Err(e) => {
                    log::warn!("migration {date}: bad metadata.json: {e}");
                    (String::new(), format!("{date}T00:00:00.000000"), None)
                }
            },
            Err(_) => (String::new(), format!("{date}T00:00:00.000000"), None),
        };

    if let Err(e) = conn.execute(
        "INSERT OR IGNORE INTO days (date, label, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![date, label, created_at, updated_at],
    ) {
        log::warn!("migration {date}: day insert failed: {e}");
        return;
    }
    if !label.is_empty()
        && let Err(e) = conn.execute(
            "UPDATE days SET label = ?1, updated_at = COALESCE(updated_at, ?3) WHERE date = ?2 AND label = ''",
            rusqlite::params![label, date, updated_at],
        )
    {
        log::warn!("migration {date}: label fill failed: {e}");
    }

    let db_path = folder.join("history.db");
    if !db_path.exists() {
        return;
    }
    let Ok(meta_len) = std::fs::metadata(&db_path).map(|m| m.len()) else {
        return;
    };
    if meta_len == 0 {
        log::warn!("migration {date}: 0-byte DB, keeping empty clip list");
        return;
    }

    let legacy = match Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("migration {date}: cannot open history.db read-only: {e}");
            return;
        }
    };

    let mut stmt = match legacy.prepare("SELECT timestamp, content FROM clips ORDER BY timestamp ASC") {
        Ok(s) => s,
        Err(e) => {
            log::warn!("migration {date}: legacy query prepare failed: {e}");
            return;
        }
    };
    let rows: Vec<(String, String)> = match stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("migration {date}: legacy read failed: {e}");
            return;
        }
    };

    for (timestamp, content) in rows {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM clips WHERE date = ?1 AND timestamp = ?2 AND text = ?3)",
                rusqlite::params![date, timestamp, content],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if exists {
            continue;
        }
        if let Err(e) = conn.execute(
            "INSERT INTO clips (date, timestamp, kind, text) VALUES (?1, ?2, 'text', ?3)",
            rusqlite::params![date, timestamp, content],
        ) {
            log::warn!("migration {date}: clip insert failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{get_day_data, init_schema};

    fn legacy_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let days = dir.path().join("Days");
        let day_a = days.join("2026-09-01");
        std::fs::create_dir_all(&day_a).unwrap();
        let conn = Connection::open(day_a.join("history.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE clips (id INTEGER PRIMARY KEY, timestamp TEXT, content TEXT);
             CREATE INDEX idx_timestamp ON clips(timestamp);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clips (timestamp, content) VALUES (?1, ?2)",
            ["2026-09-01T10:00:00.000000", "hello world"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clips (timestamp, content) VALUES (?1, ?2)",
            ["2026-09-01T11:00:00.000000", "second"],
        )
        .unwrap();
        drop(conn);
        std::fs::write(
            day_a.join("metadata.json"),
            serde_json::json!({"date": "2026-09-01", "label": "My Day", "created_at": "2026-09-01T00:00:00"}).to_string(),
        )
        .unwrap();
        let degenerate = days.join("2026-09-02");
        std::fs::create_dir_all(&degenerate).unwrap();
        std::fs::write(degenerate.join("history.db"), b"").unwrap();
        let media = dir.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        (dir, media)
    }

    #[test]
    fn migrates_and_idempotent_with_degenerate() {
        let (dir, media) = legacy_fixture();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        run(&conn, dir.path(), &media).unwrap();
        let (clips, meta) = get_day_data(&conn, "2026-09-01", &media).unwrap();
        assert_eq!(clips.len(), 2);
        assert_eq!(meta.label, "My Day");
        let (deg_clips, deg_meta) = get_day_data(&conn, "2026-09-02", &media).unwrap();
        assert!(deg_clips.is_empty());
        assert_eq!(deg_meta.date, "2026-09-02");

        conn.execute("DELETE FROM meta WHERE key='legacy_import_done'", [])
            .unwrap();
        run(&conn, dir.path(), &media).unwrap();
        let (clips2, _) = get_day_data(&conn, "2026-09-01", &media).unwrap();
        assert_eq!(clips2.len(), 2);
    }

    #[test]
    fn label_fill_keeps_updated_at_clean() {
        let (dir, media) = legacy_fixture();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO days (date, label, created_at) VALUES ('2026-09-01', '', '2026-09-01T00:00:00.000000')",
            [],
        )
        .unwrap();
        run(&conn, dir.path(), &media).unwrap();
        let (label, updated_at): (String, Option<String>) = conn
            .query_row(
                "SELECT label, updated_at FROM days WHERE date = '2026-09-01'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(label, "My Day");
        assert!(updated_at.is_none());
    }
}
