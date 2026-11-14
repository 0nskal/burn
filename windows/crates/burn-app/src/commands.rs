//! What the panel's page can ask for.

use crate::model::{Model, Snapshot};
use crate::panel::Panel;
use burn_core::Period;
use burn_core::settings::Settings;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_state(model: State<'_, Arc<Model>>) -> Snapshot {
    model.snapshot()
}

#[tauri::command]
pub fn set_period(model: State<'_, Arc<Model>>, period: Period) {
    model.set_period(period);
}

/// The settings as the panel edited them.
#[tauri::command]
pub fn set_settings(app: AppHandle, model: State<'_, Arc<Model>>, settings: Settings) {
    #[cfg(windows)]
    {
        let tray_only = settings.tray_only;
        let _ = app.run_on_main_thread(move || burn_taskbar::win::set_tray_only(tray_only));
    }
    #[cfg(not(windows))]
    let _ = app;
    model.set_settings(settings);
}

/// Turns launch at login on or off; the panel gets the actual state back in the next snapshot.
#[tauri::command]
pub fn set_launch_at_login(model: State<'_, Arc<Model>>, on: bool) {
    #[cfg(windows)]
    {
        if let Err(e) = burn_taskbar::win::autostart::set_enabled(on) {
            crate::log!("launch at login: {e}");
        }
        model.set_launch_at_login(burn_taskbar::win::autostart::is_enabled());
    }
    #[cfg(not(windows))]
    model.set_launch_at_login(on);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    crate::log!("quit");
    app.exit(0);
}

/// Whether the panel is open; the page asks once it listens, as it may have missed `panel-shown`.
#[tauri::command]
pub fn panel_visible(panel: State<'_, Panel>) -> bool {
    panel.is_visible()
}

#[tauri::command]
pub fn hide_panel(app: AppHandle, panel: State<'_, Panel>) {
    panel.hide(&app);
}

/// The page's content size in CSS pixels; the window follows it.
#[tauri::command]
pub fn content_resized(app: AppHandle, panel: State<'_, Panel>, width: f64, height: f64) {
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        panel.content_resized(&app, width, height);
    }
}
