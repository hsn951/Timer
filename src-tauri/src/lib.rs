mod audio;
mod commands;
mod engine;
mod store;
mod tray;

use engine::{Engine, Kind};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use store::{Preset, Settings, SoundMeta};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

pub struct AppState {
    pub engine: Mutex<Engine>,
    pub settings: Mutex<Settings>,
    pub presets: Mutex<Vec<Preset>>,
    pub sounds: Mutex<Vec<SoundMeta>>,
    pub audio: audio::AudioHandle,
    pub data_dir: PathBuf,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let toggle =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
                    let show = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT);
                    let show_alt =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH);
                    if shortcut == &toggle {
                        crate::toggle_active(app);
                    } else if shortcut == &show || shortcut == &show_alt {
                        crate::show_main(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = match app.path().app_data_dir() {
                Ok(dir) => dir,
                Err(err) => return Err(format!("无法定位数据目录: {err}").into()),
            };
            std::fs::create_dir_all(&data_dir)?;

            let settings: Settings = store::load_json(&store::settings_path(&data_dir));
            let presets: Vec<Preset> = store::load_json(&store::presets_path(&data_dir));
            let mut sounds: Vec<SoundMeta> = store::load_json(&store::sounds_path(&data_dir));

            audio::ensure_builtins(&data_dir, &mut sounds);
            let _ = store::save_json(&store::sounds_path(&data_dir), &sounds);

            let engine = Engine::new(&settings);
            let data_dir_for_log = data_dir.clone();
            let state = AppState {
                engine: Mutex::new(engine),
                settings: Mutex::new(settings),
                presets: Mutex::new(presets),
                sounds: Mutex::new(sounds),
                audio: audio::spawn(),
                data_dir,
            };
            app.manage(state);

            match tray::create(&handle) {
                Ok(_) => {
                    let _ = std::fs::write(data_dir_for_log.join("tray.log"), "tray: ok");
                }
                Err(err) => {
                    let _ = std::fs::write(data_dir_for_log.join("tray.log"), format!("tray: {err}"));
                }
            }
            commands::register_shortcuts(&handle);

            let tick_handle = handle.clone();
            std::thread::Builder::new()
                .name("timer-tick".into())
                .spawn(move || tick_loop(tick_handle))
                .ok();

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::start,
            commands::pause,
            commands::reset,
            commands::lap,
            commands::switch_kind,
            commands::set_countdown,
            commands::add_time,
            commands::skip_phase,
            commands::start_preset,
            commands::add_preset,
            commands::remove_preset,
            commands::update_settings,
            commands::set_mode,
            commands::set_fullscreen,
            commands::import_sound,
            commands::remove_sound,
            commands::preview_sound,
            commands::stop_sound,
            commands::hide_window,
            commands::quit,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}

fn tick_loop(app: AppHandle) {
    let mut last_wall = SystemTime::now();
    let mut last_instant = Instant::now();

    loop {
        std::thread::sleep(Duration::from_millis(50));

        let now = Instant::now();
        let wall = SystemTime::now();
        let d_wall = wall.duration_since(last_wall).unwrap_or_default();
        let d_mono = now.saturating_duration_since(last_instant);
        last_wall = wall;
        last_instant = now;

        let state = app.state::<AppState>();
        let mut just_finished: Option<String> = None;
        let mut advanced = false;

        let snapshot = {
            let mut engine = match state.engine.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };

            if d_wall > d_mono + Duration::from_millis(2000) {
                let extra = d_wall.saturating_sub(d_mono).as_millis() as u64;
                engine.compensate(extra, now);
            }

            if !engine.running() {
                continue;
            }

            let settings = match state.settings.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            let mut snap = engine.snapshot(now, settings.rounds);

            if engine.kind != Kind::Stopwatch && snap.remaining_ms == 0 {
                engine.finish();
                let label = engine.label.clone();
                if engine.kind == Kind::Pomodoro && settings.auto_start {
                    engine.advance(&settings);
                    engine.start(&settings);
                    advanced = true;
                }
                just_finished = Some(label);
                snap = engine.snapshot(now, settings.rounds);
            }
            snap
        };

        let _ = app.emit("timer://update", snapshot.clone());

        if let Some(label) = just_finished {
            let _ = app.emit("timer://finished", serde_json::json!({ "label": label }));
            on_finish(&app, &label);

            let latest = {
                let engine = match state.engine.lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let settings = match state.settings.lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                engine.snapshot(Instant::now(), settings.rounds)
            };
            let _ = app.emit("timer://state", latest);

            if advanced {
                // 自动进入下一阶段后，立即刷新一次界面
                let _ = app.emit("timer://update", snapshot);
            }
        }
    }
}

fn on_finish(app: &AppHandle, label: &str) {
    let state = app.state::<AppState>();
    let (muted, volume, fade, sound_id) = {
        let settings = match state.settings.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        (
            settings.muted,
            settings.volume,
            settings.fade,
            settings.sound_id.clone(),
        )
    };

    if !muted {
        if let Some(id) = sound_id {
            let sounds = match state.sounds.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(meta) = sounds.iter().find(|s| s.id == id) {
                let path = store::sounds_dir(&state.data_dir).join(&meta.file);
                state.audio.play(path, volume, fade);
            }
        }
    }

    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("计时结束")
        .body(label.to_string())
        .show();
}

pub fn emit_state(app: &AppHandle) {
    let state = app.state::<AppState>();
    let snapshot = {
        let engine = match state.engine.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let settings = match state.settings.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        engine.snapshot(Instant::now(), settings.rounds)
    };
    let _ = app.emit("timer://state", snapshot);
}

pub fn toggle_active(app: &AppHandle) {
    // 窗口被收进托盘时，这个快捷键优先把窗口叫回来
    if let Some(window) = app.get_webview_window("main") {
        if !window.is_visible().unwrap_or(true) {
            show_main(app);
            return;
        }
    }

    let state = app.state::<AppState>();
    {
        let mut engine = match state.engine.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if engine.running() {
            engine.pause(Instant::now());
        } else {
            let settings = match state.settings.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            engine.start(&settings);
        }
    }
    emit_state(app);
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
