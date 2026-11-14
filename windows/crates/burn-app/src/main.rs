//! Burn for Windows: live limits of Claude Code, Codex and Cursor in the taskbar, and what the
//! usage would cost at API prices in a panel above it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod model;
mod panel;
#[cfg(windows)]
mod shell;
mod update;

use burn_core::Paths;
use model::Model;
use panel::Panel;
#[cfg(windows)]
use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};

/// Appends a line to Burn's log (`%LOCALAPPDATA%\Burn\burn.log` on Windows, stderr elsewhere).
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::write_log(&format!($($t)*)) };
}

pub fn write_log(line: &str) {
    #[cfg(windows)]
    burn_taskbar::win::write_log(line);
    #[cfg(not(windows))]
    eprintln!("{line}");
}

fn main() {
    #[cfg(windows)]
    {
        burn_taskbar::win::init_log();
        log!("Burn {} on {}", env!("CARGO_PKG_VERSION"), burn_taskbar::win::os_description());
    }
    let paths = Paths::detect();
    let model = Model::new(paths);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch opens the panel of the running one.
            open_panel(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(model.clone())
        .manage(Panel::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_period,
            commands::hide_panel,
            commands::panel_visible,
            commands::set_settings,
            commands::set_launch_at_login,
            commands::quit,
            commands::shelf_copy,
            commands::share_copy,
            commands::install_update,
            commands::shelf_delete,
            commands::content_resized,
        ])
        .setup(move |app| {
            // Shelf previews are shown straight from disk, and only from there.
            let _ = app.asset_protocol_scope().allow_directory(model.shelf_dir(), true);
            let handle = app.handle().clone();
            model.set_listener(Box::new(move |snap| {
                let _ = handle.emit_to(panel::LABEL, "state", snap);
                push_widget(&handle);
            }));
            #[cfg(windows)]
            {
                model.set_launch_at_login(burn_taskbar::win::autostart::is_enabled());
                shell::start(app.handle())?;
            }
            #[cfg(not(windows))]
            {
                // Development off Windows: there is no taskbar, so the panel stays open.
                app.state::<Panel>().toggle(app.handle(), None);
            }
            model.start();
            update::start(app.handle());
            // Reset countdowns in the widget age between refreshes.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    push_widget(&handle);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) if cfg!(windows) => {
                window.app_handle().state::<Panel>().hide(window.app_handle());
            }
            WindowEvent::CloseRequested { api, .. } => {
                // Alt+F4 closes the panel, not the app.
                api.prevent_close();
                window.app_handle().state::<Panel>().hide(window.app_handle());
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("build Burn")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                // On the UI thread, before the process ends: no ghost tray icon, no orphaned widget.
                #[cfg(windows)]
                burn_taskbar::win::shutdown();
            }
        });
}

fn open_panel(app: &tauri::AppHandle) {
    let panel = app.state::<Panel>();
    if panel.is_visible() {
        return;
    }
    #[cfg(windows)]
    panel.toggle(app, burn_taskbar::win::current_anchor().map(shell::placement));
    #[cfg(not(windows))]
    panel.toggle(app, None);
}

/// Sends the widget its entries, on the UI thread that owns it.
fn push_widget(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let entries = app.state::<Arc<Model>>().widget_entries();
        let _ = app.run_on_main_thread(move || shell::set_entries(&entries));
    }
    #[cfg(not(windows))]
    let _ = app;
}
