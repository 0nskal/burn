//! Burn's taskbar widget for Windows: a per-pixel-alpha Direct2D window embedded in Explorer's
//! taskbar next to the notification area, with a tray icon fallback. The placement maths and the
//! widget's model are platform independent so they are tested anywhere.
// Off Windows only the shared modules build, for their tests.
#![cfg_attr(not(windows), allow(dead_code))]

pub mod layout;
pub mod model;
#[cfg(windows)]
pub mod win;
