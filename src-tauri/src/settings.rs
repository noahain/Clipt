use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_PERSONA: &str = "You are Clipt, a helpful clipboard history assistant.";
pub const DEFAULT_MODEL: &str = "qwen/qwen3.5-122b-a10b";
pub const DEFAULT_AI_NAME: &str = "Clipt";
pub const MIN_IMAGE_BYTES: u64 = 1024;
pub const DEFAULT_MAX_IMAGE_BYTES: u64 = 26_214_400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_ai_name")]
    pub name: String,
    #[serde(default = "default_persona")]
    pub persona: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSettings {
    #[serde(default = "default_true")]
    pub text: bool,
    #[serde(default = "default_true")]
    pub images: bool,
    #[serde(default = "default_true")]
    pub files: bool,
    #[serde(default = "default_max_image_bytes")]
    pub max_image_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSettings {
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
    #[serde(default = "default_cta_style")]
    pub cta_style: String,
    #[serde(default = "default_bg_type")]
    pub bg_type: String,
    #[serde(default = "default_bg_color")]
    pub bg_color: String,
    #[serde(default)]
    pub bg_gradient: String,
    #[serde(default = "default_bg_transparency")]
    pub bg_transparency: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub capture: CaptureSettings,
    #[serde(default)]
    pub theme: ThemeSettings,
    #[serde(default)]
    pub launch_at_login: bool,
}

fn default_true() -> bool {
    true
}
fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}
fn default_ai_name() -> String {
    DEFAULT_AI_NAME.to_string()
}
fn default_persona() -> String {
    DEFAULT_PERSONA.to_string()
}
fn default_max_image_bytes() -> u64 {
    DEFAULT_MAX_IMAGE_BYTES
}
fn default_theme_mode() -> String {
    "dark".to_string()
}
fn default_cta_style() -> String {
    "blur".to_string()
}
fn default_bg_type() -> String {
    "none".to_string()
}
fn default_bg_color() -> String {
    "#0a0a0a".to_string()
}
fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()))
}
fn default_bg_transparency() -> u8 {
    75
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_model(),
            name: default_ai_name(),
            persona: default_persona(),
        }
    }
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            text: true,
            images: true,
            files: true,
            max_image_bytes: default_max_image_bytes(),
        }
    }
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            theme_mode: default_theme_mode(),
            cta_style: default_cta_style(),
            bg_type: default_bg_type(),
            bg_color: default_bg_color(),
            bg_gradient: String::new(),
            bg_transparency: default_bg_transparency(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    ai: Option<AiPatch>,
    capture: Option<CapturePatch>,
    theme: Option<ThemePatch>,
    #[serde(alias = "launch_at_login")]
    launch_at_login: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiPatch {
    #[serde(alias = "api_key")]
    api_key: Option<String>,
    model: Option<String>,
    name: Option<String>,
    persona: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapturePatch {
    text: Option<bool>,
    images: Option<bool>,
    files: Option<bool>,
    #[serde(alias = "max_image_bytes")]
    max_image_bytes: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemePatch {
    theme_mode: Option<String>,
    cta_style: Option<String>,
    bg_type: Option<String>,
    bg_color: Option<String>,
    bg_gradient: Option<String>,
    bg_transparency: Option<u64>,
}

impl Settings {
    pub fn clamp(&mut self) {
        if self.capture.max_image_bytes < MIN_IMAGE_BYTES {
            self.capture.max_image_bytes = MIN_IMAGE_BYTES;
        }
        if !["dark", "light"].contains(&self.theme.theme_mode.as_str()) {
            self.theme.theme_mode = default_theme_mode();
        }
        if !["blur", "glass", "solid", "outline"].contains(&self.theme.cta_style.as_str()) {
            self.theme.cta_style = default_cta_style();
        }
        if !["none", "color", "gradient"].contains(&self.theme.bg_type.as_str()) {
            self.theme.bg_type = default_bg_type();
        }
        if !is_hex_color(&self.theme.bg_color) {
            self.theme.bg_color = default_bg_color();
        }
        if self.theme.bg_gradient.len() > 256 {
            self.theme.bg_gradient.clear();
        }
        if self.theme.bg_transparency > 100 {
            self.theme.bg_transparency = 100;
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                log::warn!("settings read failed ({}), using defaults", e);
                Self::default()
            }
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(mut s) => {
                    s.clamp();
                    s
                }
                Err(e) => {
                    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
                    let backup = path.with_extension(format!("json.bak-{ts}"));
                    let _ = std::fs::rename(path, &backup);
                    log::warn!("corrupt settings.json backed up, using defaults: {e}");
                    Self::default()
                }
            },
        }
    }

    pub fn save_to_path(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn apply_patch(&mut self, patch: &serde_json::Value) {
        let Ok(patch) = serde_json::from_value::<SettingsPatch>(patch.clone()) else {
            return;
        };
        if let Some(ai) = patch.ai {
            if let Some(v) = ai.api_key {
                self.ai.api_key = v;
            }
            if let Some(v) = ai.model
                && !v.trim().is_empty()
            {
                self.ai.model = v;
            }
            if let Some(v) = ai.name {
                self.ai.name = v;
            }
            if let Some(v) = ai.persona {
                self.ai.persona = v;
            }
        }
        if let Some(cap) = patch.capture {
            if let Some(v) = cap.text {
                self.capture.text = v;
            }
            if let Some(v) = cap.images {
                self.capture.images = v;
            }
            if let Some(v) = cap.files {
                self.capture.files = v;
            }
            if let Some(v) = cap.max_image_bytes {
                self.capture.max_image_bytes = v;
            }
        }
        if let Some(v) = patch.launch_at_login {
            self.launch_at_login = v;
        }
        if let Some(theme) = patch.theme {
            if let Some(v) = theme.theme_mode {
                self.theme.theme_mode = v;
            }
            if let Some(v) = theme.cta_style {
                self.theme.cta_style = v;
            }
            if let Some(v) = theme.bg_type {
                self.theme.bg_type = v;
            }
            if let Some(v) = theme.bg_color {
                self.theme.bg_color = v;
            }
            if let Some(v) = theme.bg_gradient {
                self.theme.bg_gradient = v;
            }
            if let Some(v) = theme.bg_transparency {
                self.theme.bg_transparency = v.min(100) as u8;
            }
        }
        self.clamp();
    }
}

pub fn parse_dotenv_file(path: &Path) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim().to_string();
        let mut v = v.trim().to_string();
        if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
            || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
        {
            v = v[1..v.len() - 1].to_string();
        }
        map.insert(k, v);
    }
    map
}

pub fn apply_dotenv_once(settings: &mut Settings, env_path: &Path, already_imported: bool) -> bool {
    if already_imported {
        return false;
    }
    if !env_path.exists() {
        return false;
    }
    let map = parse_dotenv_file(env_path);
    if map.is_empty() {
        return false;
    }
    let mut changed = false;
    if let Some(v) = map.get("NVIDIA_API_KEY") {
        settings.ai.api_key = v.clone();
        changed = true;
    }
    if let Some(v) = map.get("AI_MODEL")
        && !v.trim().is_empty()
    {
        settings.ai.model = v.clone();
        changed = true;
    }
    if let Some(v) = map.get("AI_NAME") {
        settings.ai.name = v.clone();
        changed = true;
    }
    if let Some(v) = map.get("AI_PERSONA") {
        settings.ai.persona = v.clone();
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_file_recovers_with_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ not json").unwrap();
        let s = Settings::load_from_path(&path);
        assert_eq!(s, Settings::default());
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.iter().any(|n| n.starts_with("settings.json.bak-")));
    }

    #[test]
    fn dotenv_import_verbatim_persona() {
        let dir = tempfile::tempdir().unwrap();
        let env = dir.path().join(".env");
        let persona = "Line1 with `code` and \"quotes\" and triple-backtick blocks";
        std::fs::write(
            &env,
            format!(
                "NVIDIA_API_KEY=abc123\nAI_MODEL=custom/model\nAI_NAME=Custom\nAI_PERSONA={persona}"
            ),
        )
        .unwrap();
        let mut s = Settings::default();
        let changed = apply_dotenv_once(&mut s, &env, false);
        assert!(changed);
        assert_eq!(s.ai.api_key, "abc123");
        assert_eq!(s.ai.model, "custom/model");
        assert_eq!(s.ai.persona, persona);
        assert!(!apply_dotenv_once(&mut s, &env, true));
    }

    #[test]
    fn theme_patch_validates() {
        let mut s = Settings::default();
        assert_eq!(s.theme.theme_mode, "dark");
        s.apply_patch(&serde_json::json!({
            "theme": {
                "themeMode": "light",
                "ctaStyle": "nope",
                "bgType": "gradient",
                "bgGradient": "linear-gradient(#000, #111)",
                "bgTransparency": 250
            }
        }));
        assert_eq!(s.theme.theme_mode, "light");
        assert_eq!(s.theme.cta_style, "blur");
        assert_eq!(s.theme.bg_type, "gradient");
        assert_eq!(s.theme.bg_gradient, "linear-gradient(#000, #111)");
        assert_eq!(s.theme.bg_transparency, 100);
    }

    #[test]
    fn patch_merges_and_clamps() {
        let mut s = Settings::default();
        let patch = serde_json::json!({
            "capture": { "maxImageBytes": 10 },
            "launchAtLogin": true,
            "ai": { "model": "x/y" }
        });
        s.apply_patch(&patch);
        assert_eq!(s.capture.max_image_bytes, MIN_IMAGE_BYTES);
        assert!(s.launch_at_login);
        assert_eq!(s.ai.model, "x/y");
        assert_eq!(s.ai.api_key, "");
    }

    #[test]
    fn legacy_snake_case_keys_still_apply() {
        let mut s = Settings::default();
        s.apply_patch(&serde_json::json!({
            "ai": { "api_key": "k" },
            "capture": { "max_image_bytes": 4096 },
            "launch_at_login": true
        }));
        assert_eq!(s.ai.api_key, "k");
        assert_eq!(s.capture.max_image_bytes, 4096);
        assert!(s.launch_at_login);
    }

    #[test]
    fn malformed_patch_field_rejects_patch_atomically() {
        let mut s = Settings::default();
        s.apply_patch(&serde_json::json!({
            "theme": { "themeMode": "light" },
            "capture": { "text": "not-a-bool" }
        }));
        assert_eq!(s.theme.theme_mode, "dark");
    }
}
