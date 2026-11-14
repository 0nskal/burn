//! The Windows clipboard for the shelf: notices copied images and puts saved ones back.
//!
//! Skips what shouldn't be kept, like the Mac app skips concealed and transient pasteboard types:
//! content marked `ExcludeClipboardContentFromMonitorProcessing` or `Clipboard Viewer Ignore`
//! (password managers), anything that also carries text or files (an image rendition of a
//! document or a file copy), and what Burn put there itself (its own marker format).

use std::path::PathBuf;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND, POINT};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_DIBV5, CF_HDROP, CF_UNICODETEXT};
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::Shell::DROPFILES;
use windows::core::{BOOL, w};

use super::util::log;

/// An image someone copied, as the clipboard had it.
pub enum ClipboardImage {
    Png(Vec<u8>),
    /// A packed DIB (header and pixels), to be converted to PNG off the UI thread.
    Dib(Vec<u8>),
}

struct Formats {
    png: u32,
    own: u32,
    exclude: u32,
    ignore: u32,
}

fn formats() -> Formats {
    unsafe {
        Formats {
            png: RegisterClipboardFormatW(w!("PNG")),
            own: RegisterClipboardFormatW(w!("Burn.Shelf")),
            exclude: RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
            ignore: RegisterClipboardFormatW(w!("Clipboard Viewer Ignore")),
        }
    }
}

/// Starts sending `WM_CLIPBOARDUPDATE` to `hwnd`.
pub fn listen(hwnd: HWND) {
    if let Err(e) = unsafe { AddClipboardFormatListener(hwnd) } {
        log!("clipboard listener failed: {e:?} (the shelf won't capture)");
    }
}

/// Opens the clipboard, retrying briefly: another app may hold it for a moment.
fn open(hwnd: HWND) -> bool {
    for _ in 0..5 {
        if unsafe { OpenClipboard(Some(hwnd)) }.is_ok() {
            return true;
        }
        unsafe { Sleep(10) };
    }
    false
}

struct Opened;

impl Drop for Opened {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn available(format: u32) -> bool {
    format != 0 && unsafe { IsClipboardFormatAvailable(format) }.is_ok()
}

/// Copies the bytes of one clipboard format.
fn read(format: u32) -> Option<Vec<u8>> {
    unsafe {
        let handle = GetClipboardData(format).ok()?;
        let global = HGLOBAL(handle.0);
        let size = GlobalSize(global);
        let ptr = GlobalLock(global) as *const u8;
        if ptr.is_null() || size == 0 {
            return None;
        }
        let data = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(global);
        Some(data)
    }
}

/// The copied image, if there is one the shelf should keep. Call on `WM_CLIPBOARDUPDATE`.
pub fn read_image(hwnd: HWND) -> Option<ClipboardImage> {
    let f = formats();
    if !open(hwnd) {
        log!("clipboard busy, a copied image may be missed");
        return None;
    }
    let _closer = Opened;
    let skip = [f.own, f.exclude, f.ignore, u32::from(CF_UNICODETEXT.0), u32::from(CF_HDROP.0)];
    if skip.into_iter().any(available) {
        return None;
    }
    if available(f.png)
        && let Some(png) = read(f.png)
    {
        return Some(ClipboardImage::Png(png));
    }
    for format in [u32::from(CF_DIBV5.0), u32::from(CF_DIB.0)] {
        if available(format)
            && let Some(dib) = read(format)
        {
            return Some(ClipboardImage::Dib(dib));
        }
    }
    None
}

/// Moveable global memory holding `bytes`, as SetClipboardData wants it.
fn global(bytes: &[u8]) -> Option<HGLOBAL> {
    unsafe {
        let g = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).ok()?;
        let ptr = GlobalLock(g) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(g));
            return None;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(g);
        Some(g)
    }
}

fn set(format: u32, bytes: &[u8]) -> bool {
    let Some(g) = global(bytes) else { return false };
    unsafe {
        if SetClipboardData(format, Some(HANDLE(g.0))).is_ok() {
            true
        } else {
            // Still ours when the clipboard didn't take it.
            let _ = GlobalFree(Some(g));
            false
        }
    }
}

/// CF_HDROP data: a DROPFILES header, then the wide paths, each NUL-terminated, then one more NUL.
fn drop_files(paths: &[PathBuf]) -> Vec<u8> {
    let header =
        DROPFILES { pFiles: size_of::<DROPFILES>() as u32, pt: POINT::default(), fNC: BOOL(0), fWide: BOOL(1) };
    let mut out = unsafe {
        std::slice::from_raw_parts(&header as *const DROPFILES as *const u8, size_of::<DROPFILES>()).to_vec()
    };
    for p in paths {
        for unit in p.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// One image: PNG, DIB and the file, so image and file targets both accept it.
pub struct ImageFile {
    pub png: Vec<u8>,
    pub dib: Option<Vec<u8>>,
    pub path: Option<PathBuf>,
}

/// Puts images on the clipboard, marked as Burn's own so the shelf doesn't keep them again.
/// One image goes as PNG, DIB and file; several go as files only (an image format holds one).
pub fn write_images(hwnd: HWND, images: &[ImageFile]) -> bool {
    if images.is_empty() {
        return false;
    }
    let f = formats();
    if !open(hwnd) {
        log!("clipboard busy, copy failed");
        return false;
    }
    let _closer = Opened;
    if unsafe { EmptyClipboard() }.is_err() {
        return false;
    }
    let mut ok = set(f.own, &[1]);
    if let [one] = images {
        ok &= set(f.png, &one.png);
        if let Some(dib) = &one.dib {
            ok &= set(u32::from(CF_DIB.0), dib);
        }
    }
    let paths: Vec<PathBuf> = images.iter().filter_map(|i| i.path.clone()).collect();
    if !paths.is_empty() {
        ok &= set(u32::from(CF_HDROP.0), &drop_files(&paths));
    }
    ok
}
