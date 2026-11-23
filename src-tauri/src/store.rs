use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub volume: f32,
    pub fade: bool,
    pub muted: bool,
    pub sound_id: Option<String>,
    pub always_on_top: bool,
    pub autostart: bool,
    pub accent: String,
    pub work_min: u32,
    pub break_min: u32,
    pub long_break_min: u32,
    pub rounds: u32,
    pub auto_start: bool,
    pub mode: String,
    pub fs_style: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.7,
            fade: true,
            muted: false,
            sound_id: Some("builtin-chime".to_string()),
            always_on_top: false,
            autostart: false,
            accent: "pink".to_string(),
            work_min: 25,
            break_min: 5,
            long_break_min: 15,
            rounds: 4,
            auto_start: true,
            mode: "normal".to_string(),
            fs_style: "digital".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub ms: u64,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SoundMeta {
    pub id: String,
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub builtin: bool,
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

pub fn presets_path(dir: &Path) -> PathBuf {
    dir.join("presets.json")
}

pub fn sounds_path(dir: &Path) -> PathBuf {
    dir.join("sounds.json")
}

pub fn sounds_dir(dir: &Path) -> PathBuf {
    dir.join("sounds")
}
