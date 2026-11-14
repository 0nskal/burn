fn main() {
    // Our own manifest: Windows 8+ support is required for the layered child window the widget
    // lives in (parented into the taskbar), plus per-monitor DPI and common controls v6.
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("burn.exe.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build");
}
