use crate::engine::Kind;
use crate::store::{self, Preset, Settings, SoundMeta};
use crate::{emit_state, AppState};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
pub struct Bootstrap {
    pub timer: crate::engine::Snapshot,
    pub settings: Settings,
    pub presets: Vec<Preset>,
    pub sounds: Vec<SoundMeta>,
    pub version: String,
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Bootstrap {
    let engine = state.engine.lock().unwrap();
    let settings = state.settings.lock().unwrap();
    let presets = state.presets.lock().unwrap().clone();
    let sounds = state.sounds.lock().unwrap().clone();
    Bootstrap {
        timer: engine.snapshot(std::time::Instant::now(), settings.rounds),
        settings: settings.clone(),
        presets,
        sounds,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub fn start(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        if !engine.running() {
            let settings = state.settings.lock().unwrap();
            engine.start(&settings);
        }
    }
    emit_state(&app);
}

#[tauri::command]
pub fn pause(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        engine.pause(std::time::Instant::now());
    }
    emit_state(&app);
}

#[tauri::command]
pub fn reset(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        engine.reset(&settings);
    }
    emit_state(&app);
}

#[tauri::command]
pub fn lap(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        engine.lap(std::time::Instant::now());
    }
    emit_state(&app);
}

#[tauri::command]
pub fn switch_kind(app: AppHandle, kind: Kind, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        engine.switch_kind(kind, &settings);
    }
    emit_state(&app);
}

#[tauri::command]
pub fn set_countdown(app: AppHandle, ms: u64, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        engine.set_target(ms);
        engine.label = "倒计时".to_string();
    }
    emit_state(&app);
}

#[tauri::command]
pub fn add_time(app: AppHandle, ms: u64, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        engine.add_time(ms);
    }
    emit_state(&app);
}

#[tauri::command]
pub fn skip_phase(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut engine = state.engine.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        engine.skip_phase(&settings);
    }
    emit_state(&app);
}

#[tauri::command]
pub fn start_preset(app: AppHandle, id: String, state: State<'_, AppState>) {
    let preset = {
        let presets = state.presets.lock().unwrap();
        presets.iter().find(|p| p.id == id).cloned()
    };
    if let Some(p) = preset {
        {
            let mut engine = state.engine.lock().unwrap();
            engine.kind = Kind::Countdown;
            engine.target_ms = p.ms.max(1000);
            engine.accumulated_ms = 0;
            engine.finished = false;
            engine.laps.clear();
            engine.label = p.name.clone();
            engine.preset_id = Some(p.id.clone());
            engine.phase = crate::engine::Phase::Work;
            let settings = state.settings.lock().unwrap();
            engine.start(&settings);
        }
        emit_state(&app);
    }
}

#[tauri::command]
pub fn add_preset(
    app: AppHandle,
    name: String,
    ms: u64,
    color: String,
    state: State<'_, AppState>,
) -> Result<Preset, String> {
    let preset = Preset {
        id: format!(
            "p{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ),
        name: if name.trim().is_empty() {
            "计时器".to_string()
        } else {
            name.trim().to_string()
        },
        ms: ms.max(1000),
        color,
    };
    {
        let mut presets = state.presets.lock().unwrap();
        presets.push(preset.clone());
        store::save_json(&store::presets_path(&state.data_dir), &*presets)?;
    }
    let _ = app.emit("presets://changed", ());
    Ok(preset)
}

#[tauri::command]
pub fn remove_preset(app: AppHandle, id: String, state: State<'_, AppState>) {
    {
        let mut presets = state.presets.lock().unwrap();
        presets.retain(|p| p.id != id);
        let _ = store::save_json(&store::presets_path(&state.data_dir), &*presets);
    }
    let _ = app.emit("presets://changed", ());
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    patch: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    let previous = { state.settings.lock().unwrap().clone() };
    let mut value = serde_json::to_value(&previous).map_err(|e| e.to_string())?;
    merge(&mut value, patch);
    let next: Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;

    store::save_json(&store::settings_path(&state.data_dir), &next)?;
    *state.settings.lock().unwrap() = next.clone();

    if next.always_on_top != previous.always_on_top {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_always_on_top(next.always_on_top);
        }
    }

    if next.autostart != previous.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let _ = if next.autostart {
            manager.enable()
        } else {
            manager.disable()
        };
    }

    Ok(next)
}

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: String, state: State<'_, AppState>) {
    let mini = mode == "mini";
    if let Some(window) = app.get_webview_window("main") {
        let size = if mini {
            tauri::LogicalSize::new(300.0, 96.0)
        } else {
            tauri::LogicalSize::new(380.0, 620.0)
        };
        let _ = window.set_size(size);
        let _ = window.set_skip_taskbar(mini);
        let always_on_top = if mini {
            true
        } else {
            state.settings.lock().unwrap().always_on_top
        };
        let _ = window.set_always_on_top(always_on_top);

        if let Ok(mut settings) = state.settings.lock() {
            settings.mode = mode;
            let _ = store::save_json(&store::settings_path(&state.data_dir), &*settings);
        }
    }
}

#[tauri::command]
pub fn import_sound(app: AppHandle) {
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("音频文件", &["mp3", "wav", "ogg", "flac"])
        .pick_file(move |picked| {
            let Some(file_path) = picked else {
                return;
            };
            let Some(path) = file_path.as_path().map(|p| p.to_path_buf()) else {
                return;
            };
            match do_import(&handle, &path) {
                Ok(meta) => {
                    let _ = handle.emit("sound://imported", meta);
                }
                Err(err) => {
                    let _ = handle.emit("toast", serde_json::json!({ "text": err }));
                }
            }
        });
}

fn do_import(app: &AppHandle, source: &std::path::Path) -> Result<SoundMeta, String> {
    let state = app.state::<AppState>();
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["mp3", "wav", "ogg", "flac"].contains(&ext.as_str()) {
        return Err("只支持 wav / mp3 / ogg / flac 格式".to_string());
    }

    let id = format!(
        "s{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let file_name = format!("{id}.{ext}");
    let dir = store::sounds_dir(&state.data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&file_name);
    std::fs::copy(source, &dest).map_err(|e| format!("复制文件失败: {e}"))?;

    // 立即试解码，坏文件当场拒绝
    let file = std::fs::File::open(&dest).map_err(|e| e.to_string())?;
    if rodio::Decoder::new(std::io::BufReader::new(file)).is_err() {
        let _ = std::fs::remove_file(&dest);
        return Err("无法解码这个音频文件，换一个试试".to_string());
    }

    let name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("我的铃声")
        .to_string();

    let meta = SoundMeta {
        id: id.clone(),
        name,
        file: file_name,
        builtin: false,
    };

    {
        let mut sounds = state.sounds.lock().unwrap();
        sounds.push(meta.clone());
        store::save_json(&store::sounds_path(&state.data_dir), &*sounds)?;
    }
    Ok(meta)
}

#[tauri::command]
pub fn remove_sound(app: AppHandle, id: String, state: State<'_, AppState>) {
    let file = {
        let mut sounds = state.sounds.lock().unwrap();
        let found = sounds.iter().find(|s| s.id == id).cloned();
        sounds.retain(|s| s.id != id);
        let _ = store::save_json(&store::sounds_path(&state.data_dir), &*sounds);
        found.map(|s| s.file)
    };
    if let Some(file) = file {
        let _ = std::fs::remove_file(store::sounds_dir(&state.data_dir).join(file));
    }
    {
        let mut settings = state.settings.lock().unwrap();
        if settings.sound_id.as_deref() == Some(id.as_str()) {
            settings.sound_id = state.sounds.lock().unwrap().first().map(|s| s.id.clone());
            let _ = store::save_json(&store::settings_path(&state.data_dir), &*settings);
        }
    }
    let _ = app.emit("sounds://changed", ());
}

#[tauri::command]
pub fn preview_sound(id: String, state: State<'_, AppState>) {
    let (path, volume) = {
        let sounds = state.sounds.lock().unwrap();
        let settings = state.settings.lock().unwrap();
        let Some(meta) = sounds.iter().find(|s| s.id == id) else {
            return;
        };
        (
            store::sounds_dir(&state.data_dir).join(&meta.file),
            settings.volume,
        )
    };
    state.audio.play(path, volume, false);
}

#[tauri::command]
pub fn stop_sound(state: State<'_, AppState>) {
    state.audio.stop();
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

pub fn register_shortcuts(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let show_t = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT);
    let show_h = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH);

    let mut log = String::new();
    match app.global_shortcut().register(toggle) {
        Ok(_) => log.push_str("ctrl+alt+space: ok\n"),
        Err(err) => log.push_str(&format!("ctrl+alt+space: {err}\n")),
    }
    match app.global_shortcut().register(show_t) {
        Ok(_) => log.push_str("ctrl+alt+t: ok\n"),
        Err(err) => log.push_str(&format!("ctrl+alt+t: {err}\n")),
    }
    match app.global_shortcut().register(show_h) {
        Ok(_) => log.push_str("ctrl+alt+h: ok\n"),
        Err(err) => log.push_str(&format!("ctrl+alt+h: {err}\n")),
    }

    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("shortcut.log"), log);
    }
}

fn merge(base: &mut serde_json::Value, patch: serde_json::Value) {
    if let (Some(base_map), Some(patch_map)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_map {
            match base_map.get_mut(key) {
                Some(slot) if slot.is_object() && value.is_object() => merge(slot, value.clone()),
                _ => {
                    base_map.insert(key.clone(), value.clone());
                }
            }
        }
    }
}
