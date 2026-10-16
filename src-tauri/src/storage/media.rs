use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn hash_bytes(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex_encode(h.finalize())
}

fn hex_encode(digest: impl AsRef<[u8]>) -> String {
    let mut s = String::with_capacity(64);
    for b in digest.as_ref() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn media_path_for(media_dir: &Path, hash: &str) -> PathBuf {
    media_dir.join(format!("{hash}.png"))
}

pub fn store_png(media_dir: &Path, png_bytes: &[u8]) -> anyhow::Result<String> {
    std::fs::create_dir_all(media_dir)?;
    let hash = hash_bytes(png_bytes);
    let path = media_path_for(media_dir, &hash);
    if !path.exists() {
        std::fs::write(&path, png_bytes)?;
    }
    Ok(hash)
}

pub fn refcount(conn: &Connection, hash: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM clips WHERE blob_hash = ?1",
        [hash],
        |r| r.get(0),
    )
}

pub fn gc_if_unreferenced(
    conn: &Connection,
    media_dir: &Path,
    hash: &str,
) -> anyhow::Result<()> {
    let n: i64 = match refcount(conn, hash) {
        Ok(n) => n,
        Err(e) => {
            log::warn!("media GC skipped for {hash}: refcount query failed: {e}");
            return Ok(());
        }
    };
    if n == 0 {
        let path = media_path_for(media_dir, hash);
        if path.exists()
            && let Err(e) = std::fs::remove_file(&path)
        {
            log::warn!("media GC failed to delete {}: {e}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE clips (id INTEGER PRIMARY KEY, blob_hash TEXT);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn content_addressed_once() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"fake-png-bytes";
        let h1 = store_png(dir.path(), bytes).unwrap();
        let h2 = store_png(dir.path(), bytes).unwrap();
        assert_eq!(h1, h2);
        assert!(media_path_for(dir.path(), &h1).exists());
    }

    #[test]
    fn gc_skips_delete_when_refcount_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let h = store_png(dir.path(), b"data").unwrap();
        gc_if_unreferenced(&conn, dir.path(), &h).unwrap();
        assert!(media_path_for(dir.path(), &h).exists());
    }

    #[test]
    fn gc_orphan_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_mem();
        gc_if_unreferenced(&conn, dir.path(), "nonexistent").unwrap();
        let h = store_png(dir.path(), b"data").unwrap();
        conn.execute("INSERT INTO clips (blob_hash) VALUES (?1)", [&h])
            .unwrap();
        gc_if_unreferenced(&conn, dir.path(), &h).unwrap();
        assert!(media_path_for(dir.path(), &h).exists());
        conn.execute("DELETE FROM clips", []).unwrap();
        gc_if_unreferenced(&conn, dir.path(), &h).unwrap();
        assert!(!media_path_for(dir.path(), &h).exists());
    }
}
