use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示 / 隐藏", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "开始 / 暂停", true, None::<&str>)?;
    let mute = MenuItem::with_id(app, "mute", "静音 / 取消静音", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &toggle, &mute, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("马卡龙计时器")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => toggle_window(app),
            "toggle" => crate::toggle_active(app),
            "mute" => toggle_mute(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            crate::show_main(app);
        }
    }
}

fn toggle_mute(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    {
        let mut settings = state.settings.lock().unwrap();
        settings.muted = !settings.muted;
        let _ = crate::store::save_json(
            &crate::store::settings_path(&state.data_dir),
            &*settings,
        );
    }
    let _ = app.emit("settings://changed", ());
}
