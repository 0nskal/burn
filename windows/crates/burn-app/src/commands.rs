//! What the panel's page can ask for.

use crate::model::{Model, Snapshot};
use crate::panel::Panel;
use burn_core::Period;
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
