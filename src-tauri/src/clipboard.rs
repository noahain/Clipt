use tauri::{AppHandle, Emitter, Manager};

use crate::settings::CaptureSettings;
use crate::state::{AppState, ClipFingerprint};
use crate::storage::{self, media};

#[derive(Debug, Clone, PartialEq)]
pub struct ImageInput {
    pub png_bytes: Vec<u8>,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedClip {
    pub kind: String,
    pub text: Option<String>,
    pub image: Option<ImageInput>,
    pub files: Option<Vec<String>>,
}

impl ClassifiedClip {
    pub fn fingerprint(&self) -> ClipFingerprint {
        match self.kind.as_str() {
            "image" => ClipFingerprint::Image(self.image.as_ref().unwrap().hash.clone()),
            "files" => ClipFingerprint::Files(normalize_file_set(
                self.files.as_ref().unwrap(),
            )),
            _ => ClipFingerprint::Text(self.text.clone().unwrap_or_default()),
        }
    }
}

pub fn normalize_file_set(paths: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for p in paths {
        let mut s = p.trim().to_string();
        while s.ends_with('\\') || s.ends_with('/') {
            s.pop();
        }
        let key = s.to_lowercase();
        if key.is_empty() {
            continue;
        }
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out.sort();
    out
}

pub fn classify(
    files: Option<Vec<String>>,
    image: Option<ImageInput>,
    text: Option<String>,
    capture: &CaptureSettings,
) -> Option<ClassifiedClip> {
    if capture.files
        && let Some(list) = files
    {
        let cleaned: Vec<String> = list
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !cleaned.is_empty() {
            return Some(ClassifiedClip {
                kind: "files".to_string(),
                text: None,
                image: None,
                files: Some(cleaned),
            });
        }
    }
    if capture.images
        && let Some(img) = image
    {
        if (img.png_bytes.len() as u64) <= capture.max_image_bytes {
            return Some(ClassifiedClip {
                kind: "image".to_string(),
                text: None,
                image: Some(img),
                files: None,
            });
        } else {
            log::warn!(
                "clipboard image skipped: {} bytes over limit {}",
                img.png_bytes.len(),
                capture.max_image_bytes
            );
        }
    }
    if capture.text
        && let Some(t) = text
    {
        if t.trim().is_empty() {
            return None;
        }
        return Some(ClassifiedClip {
            kind: "text".to_string(),
            text: Some(t),
            image: None,
            files: None,
        });
    }
    None
}

pub fn is_duplicate_text(new_text: &str, last: Option<&crate::storage::Clip>) -> bool {
    match last {
        Some(c) if c.kind == "text" => c.text.as_deref() == Some(new_text),
        Some(_) => false,
        None => false,
    }
}

pub fn is_duplicate_image(new_hash: &str, last: Option<&crate::storage::Clip>) -> bool {
    let Some(c) = last else { return false };
    if c.kind != "image" {
        return false;
    }
    let Some(mp) = c.media_path.as_deref() else {
        return false;
    };
    std::path::Path::new(mp).file_stem().and_then(|s| s.to_str()) == Some(new_hash)
}

pub fn is_duplicate_files(new_files: &[String], last: Option<&crate::storage::Clip>) -> bool {
    let Some(c) = last else { return false };
    if c.kind != "files" {
        return false;
    }
    let Some(existing) = c.file_paths.as_ref() else {
        return false;
    };
    normalize_file_set(new_files) == normalize_file_set(existing)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveOutcome {
    Saved,
    Duplicate,
    Error,
}

pub fn next_baseline(
    prev: Option<ClipFingerprint>,
    outcome: SaveOutcome,
    attempted: ClipFingerprint,
) -> Option<ClipFingerprint> {
    match outcome {
        SaveOutcome::Saved => Some(attempted),
        SaveOutcome::Duplicate | SaveOutcome::Error => prev,
    }
}

type RawClipboard = (Option<Vec<String>>, Option<ImageInput>, Option<String>);

fn read_clipboard_raw() -> anyhow::Result<RawClipboard> {
    let files = read_file_list().unwrap_or(None);
    let (image, text) = read_arboard()?;
    Ok((files, image, text))
}

fn read_file_list() -> anyhow::Result<Option<Vec<String>>> {
    #[cfg(windows)]
    {
        let _clip = clipboard_win::Clipboard::new_attempts(10)
            .map_err(|e| anyhow::anyhow!("clipboard open failed: {e:?}"))?;
        let mut out: Vec<String> = Vec::new();
        match clipboard_win::raw::get_file_list(&mut out) {
            Ok(_) => {
                if out.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(out))
                }
            }
            Err(_) => Ok(None),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

fn read_arboard() -> anyhow::Result<(Option<ImageInput>, Option<String>)> {
    let mut cb = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("arboard open: {e}"))?;
    let image = match cb.get_image() {
        Ok(img) => {
            let rgba: Vec<u8> = img.bytes.to_vec();
            let w = img.width as u32;
            let h = img.height as u32;
            match image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(w, h, rgba) {
                Some(buf) => {
                    let mut png = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut png);
                    match buf.write_to(&mut cursor, image::ImageFormat::Png) {
                        Ok(()) => {
                            let hash = media::hash_bytes(&png);
                            Some(ImageInput { png_bytes: png, hash })
                        }
                        Err(e) => {
                            log::warn!("png encode failed: {e}");
                            None
                        }
                    }
                }
                None => None,
            }
        }
        Err(_) => None,
    };
    let text = cb.get_text().ok();
    Ok((image, text))
}

fn read_with_retries() -> anyhow::Result<RawClipboard> {
    let mut last_err = None;
    for _ in 0..3 {
        match read_clipboard_raw() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("clipboard read failed")))
}

pub fn read_fingerprint_now(capture: &CaptureSettings) -> Option<ClipFingerprint> {
    let (files, image, text) = read_clipboard_raw().ok()?;
    classify(files, image, text, capture).map(|c| c.fingerprint())
}

pub fn handle_clipboard_event(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.paused.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(100));

    let (capture, media_dir_path) = {
        let s = state.settings.lock().unwrap();
        (s.capture.clone(), crate::paths::media_dir())
    };

    let raw = match read_with_retries() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("clipboard read failed after retries: {e:#}");
            let mut baseline = state.baseline.lock().unwrap();
            *baseline = Some(ClipFingerprint::Text("__read_error__".to_string()));
            return;
        }
    };

    let Some(classified) = classify(raw.0, raw.1, raw.2, &capture) else {
        return;
    };
    let fp = classified.fingerprint();

    {
        let baseline = state.baseline.lock().unwrap();
        if baseline.as_ref() == Some(&fp) {
            return;
        }
    }

    let (outcome, saved_clip) = {
        let db = state.db.lock().unwrap();
        let last = storage::last_clip(&db, &media_dir_path).ok().flatten();
        let dup = match classified.kind.as_str() {
            "text" => is_duplicate_text(classified.text.as_deref().unwrap_or(""), last.as_ref()),
            "image" => is_duplicate_image(
                &classified.image.as_ref().unwrap().hash,
                last.as_ref(),
            ),
            "files" => is_duplicate_files(classified.files.as_ref().unwrap(), last.as_ref()),
            _ => false,
        };
        if dup {
            (SaveOutcome::Duplicate, None)
        } else {
            let ts = storage::timestamp_now();
            let res: rusqlite::Result<storage::Clip> = match classified.kind.as_str() {
                "text" => storage::save_text_clip(
                    &db,
                    &ts,
                    classified.text.as_deref().unwrap_or(""),
                    &media_dir_path,
                ),
                "image" => {
                    let img = classified.image.as_ref().unwrap();
                    match media::store_png(&media_dir_path, &img.png_bytes) {
                        Ok(hash) => storage::save_image_clip(&db, &ts, &hash, &media_dir_path),
                        Err(e) => {
                            log::warn!("media store failed: {e:#}");
                            Err(rusqlite::Error::InvalidQuery)
                        }
                    }
                }
                "files" => storage::save_files_clip(
                    &db,
                    &ts,
                    classified.files.as_ref().unwrap(),
                    &media_dir_path,
                ),
                _ => Err(rusqlite::Error::InvalidQuery),
            };
            match res {
                Ok(clip) => (SaveOutcome::Saved, Some(clip)),
                Err(e) => {
                    log::warn!("clip save failed: {e}");
                    (SaveOutcome::Error, None)
                }
            }
        }
    };

    {
        let mut baseline = state.baseline.lock().unwrap();
        *baseline = next_baseline(baseline.clone(), outcome, fp);
    }
    if let Some(clip) = saved_clip {
        let _ = app.emit("clip-added", serde_json::json!({ "clip": clip }));
    }
}

static APP_CONTEXT: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

#[cfg(windows)]
unsafe extern "system" fn wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wp: windows::Win32::Foundation::WPARAM,
    lp: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WM_CLIPBOARDUPDATE};
    use windows::Win32::Foundation::LRESULT;
    if msg == WM_CLIPBOARDUPDATE {
        if let Some(ctx) = APP_CONTEXT.get() {
            let app = ctx.clone();
            std::thread::spawn(move || handle_clipboard_event(&app));
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    // wrapped: caller is unsafe fn; edition 2024 requires inner unsafe block
}

pub fn note_self_written_fingerprint(app: &AppHandle, fp: ClipFingerprint) {
    let state = app.state::<AppState>();
    *state.baseline.lock().unwrap() = Some(fp);
}

#[cfg(windows)]
pub fn spawn_listener(app: AppHandle) {
    let _ = APP_CONTEXT.set(app.clone());
    std::thread::spawn(move || {
        use windows::{
            core::w,
            Win32::System::DataExchange::AddClipboardFormatListener,
            Win32::System::LibraryLoader::GetModuleHandleW,
            Win32::UI::WindowsAndMessaging::*,
        };

        unsafe {
            let hinst = match GetModuleHandleW(None) {
                Ok(h) => h,
                Err(e) => {
                    log::warn!("clipboard listener: GetModuleHandle failed: {e}");
                    return;
                }
            };
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                lpszClassName: w!("CliptClipboardListener"),
                hInstance: hinst.into(),
                ..Default::default()
            };
            RegisterClassW(&class);
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("CliptClipboardListener"),
                w!(""),
                WINDOW_STYLE(0),
                0, 0, 0, 0,
                Some(HWND_MESSAGE),
                None,
                Some(hinst.into()),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    log::warn!("clipboard listener: CreateWindow failed: {e}");
                    return;
                }
            };
            if let Err(e) = AddClipboardFormatListener(hwnd) {
                log::warn!("clipboard listener: AddClipboardFormatListener failed: {e}");
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn spawn_listener(_app: AppHandle) {
    log::info!("clipboard listener stub (non-Windows)");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture_all() -> CaptureSettings {
        CaptureSettings { text: true, images: true, files: true, max_image_bytes: 25_000_000 }
    }

    #[test]
    fn priority_files_over_image_over_text() {
        let cap = capture_all();
        let img = ImageInput { png_bytes: vec![1, 2, 3], hash: "abc".into() };
        let c = classify(Some(vec!["C:\\a.txt".into()]), Some(img), Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "files");
        let img = ImageInput { png_bytes: vec![1], hash: "h".into() };
        let c = classify(None, Some(img), Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "image");
        let c = classify(None, None, Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "text");
    }

    #[test]
    fn fall_through_when_top_disabled_or_absent() {
        let mut cap = capture_all();
        cap.files = false;
        let img = ImageInput { png_bytes: vec![1], hash: "h".into() };
        let c = classify(Some(vec!["C:\\a.txt".into()]), Some(img), Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "image");
        cap.images = false;
        let c = classify(Some(vec!["C:\\a.txt".into()]), None, Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "text");
        cap.text = false;
        assert!(classify(Some(vec!["x".into()]), None, Some("hi".into()), &cap).is_none());
    }

    #[test]
    fn oversize_image_falls_through_to_text() {
        let mut cap = capture_all();
        cap.max_image_bytes = 2;
        let img = ImageInput { png_bytes: vec![1, 2, 3], hash: "abc".into() };
        let c = classify(None, Some(img), Some("hi".into()), &cap).unwrap();
        assert_eq!(c.kind, "text");
        let img = ImageInput { png_bytes: vec![1, 2, 3], hash: "abc".into() };
        assert!(classify(None, Some(img), None, &cap).is_none());
    }

    #[test]
    fn whitespace_and_empty_skipped() {
        let cap = capture_all();
        assert!(classify(None, None, Some("   ".into()), &cap).is_none());
        assert!(classify(Some(vec![]), None, None, &cap).is_none());
    }

    #[test]
    fn file_set_order_case_insensitive() {
        let a = vec!["C:\\A.txt".to_string(), "d:\\b.txt".to_string()];
        let b = vec!["D:\\B.TXT".to_string(), "c:\\a.txt\\".to_string()];
        assert_eq!(normalize_file_set(&a), normalize_file_set(&b));
    }

    #[test]
    fn baseline_moves_only_on_save() {
        let prev = Some(ClipFingerprint::Text("x".into()));
        let attempted = ClipFingerprint::Text("y".into());
        assert_eq!(
            next_baseline(prev.clone(), SaveOutcome::Saved, attempted.clone()),
            Some(attempted.clone())
        );
        assert_eq!(
            next_baseline(prev.clone(), SaveOutcome::Duplicate, attempted.clone()),
            prev
        );
        assert_eq!(next_baseline(prev.clone(), SaveOutcome::Error, attempted), prev);
    }

    #[test]
    fn dedupe_cross_midnight_text() {
        let dir = tempfile::tempdir().unwrap();
        fn open_test_db() -> rusqlite::Connection {
            let c = rusqlite::Connection::open_in_memory().unwrap();
            crate::storage::init_schema(&c).unwrap();
            c
        }
        let conn = open_test_db();
        let media = dir.path();
        crate::storage::save_text_clip(&conn, "2026-01-01T23:59:59.999999", "same", media).unwrap();
        let last = crate::storage::last_clip(&conn, media).unwrap();
        assert!(is_duplicate_text("same", last.as_ref()));
        assert!(!is_duplicate_text("different", last.as_ref()));
    }

    #[test]
    fn image_hash_dedupe() {
        let dir = tempfile::tempdir().unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::init_schema(&conn).unwrap();
        let media = dir.path();
        let h = crate::storage::media::store_png(media, b"img").unwrap();
        crate::storage::save_image_clip(&conn, "2026-01-01T10:00:00.000001", &h, media).unwrap();
        let last = crate::storage::last_clip(&conn, media).unwrap();
        assert!(is_duplicate_image(&h, last.as_ref()));
        assert!(!is_duplicate_image("other", last.as_ref()));
    }
}
