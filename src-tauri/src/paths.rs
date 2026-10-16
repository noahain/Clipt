use std::path::PathBuf;

fn roaming_base() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA")
        && !appdata.trim().is_empty()
    {
        return PathBuf::from(appdata);
    }
    dirs_fallback()
}

fn dirs_fallback() -> PathBuf {
    if let Ok(home) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(home)
            .join("AppData")
            .join("Roaming");
        return p;
    }
    PathBuf::from(".")
}

pub fn app_root() -> PathBuf {
    roaming_base().join("Clipt")
}

pub fn db_path() -> PathBuf {
    app_root().join("clipt.db")
}

pub fn settings_path() -> PathBuf {
    app_root().join("settings.json")
}

pub fn media_dir() -> PathBuf {
    app_root().join("media")
}

pub fn logs_dir() -> PathBuf {
    app_root().join("logs")
}

pub fn legacy_env_path() -> PathBuf {
    app_root().join(".env")
}

pub fn ensure_dirs() -> anyhow::Result<()> {
    for dir in [app_root(), media_dir(), logs_dir()] {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_under_appdata_env() {
        let tmp = tempfile::tempdir().unwrap();
        let fake_appdata = tmp.path().join("Roaming");
        std::fs::create_dir_all(&fake_appdata).unwrap();
        let prev = std::env::var("APPDATA").ok();
        unsafe { std::env::set_var("APPDATA", &fake_appdata) };
        let root = app_root();
        assert!(root.to_string_lossy().contains("Clipt"));
        assert!(root.starts_with(&fake_appdata));
        if let Some(v) = prev {
            unsafe { std::env::set_var("APPDATA", v) };
        } else {
            unsafe { std::env::remove_var("APPDATA") };
        }
    }
}
