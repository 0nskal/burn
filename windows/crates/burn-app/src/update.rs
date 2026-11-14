//! Self-update from GitHub releases, like the Mac app's Updater: checks once a day, offers the
//! newer version in the footer, and on request installs it with the per-user NSIS installer
//! (signature checked by the updater plugin), which restarts Burn.

use crate::model::Model;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);

/// Checks now and then once a day. Development builds never update.
pub fn start(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            tauri::async_runtime::block_on(check(&app));
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

async fn check(app: &AppHandle) {
    let model = app.state::<Arc<Model>>().inner().clone();
    let result = match app.updater() {
        Ok(u) => u.check().await,
        Err(e) => {
            crate::log!("updater unavailable: {e}");
            return;
        }
    };
    match result {
        // Cleared again when the release was pulled or this copy got updated some other way.
        Ok(update) => model.set_update(update.map(|u| u.version)),
        Err(e) => crate::log!("update check failed: {e}"),
    }
}

/// Downloads and runs the installer; the installer closes Burn and starts the new one.
pub async fn install(app: AppHandle) {
    let model = app.state::<Arc<Model>>().inner().clone();
    model.set_updating(true);
    let result = async {
        let update = app.updater()?.check().await?;
        match update {
            Some(u) => u.download_and_install(|_, _| {}, || {}).await.map(|_| true),
            None => Ok(false),
        }
    }
    .await;
    match result {
        Ok(true) => app.restart(),
        Ok(false) => model.set_update(None),
        Err(e) => crate::log!("update failed: {e}"),
    }
    model.set_updating(false);
}
