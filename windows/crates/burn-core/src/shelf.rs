//! The screenshot shelf, a port of `ShelfStore.swift`: every image copied to the clipboard is
//! kept as a PNG in Burn's data dir (`Shelf`), newest first, with small previews for the grid.
//! Platform independent; the clipboard itself lives in burn-taskbar.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tiny_skia::{FilterQuality, IntSize, Pixmap, PixmapPaint, Transform};

pub const MAX_ITEMS: usize = 100;
/// Longest side of a preview, in pixels.
const THUMB_PX: u32 = 640;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub path: PathBuf,
    /// Preview to show in the grid (created on demand).
    pub thumb: PathBuf,
    /// Epoch ms the screenshot was taken.
    pub date: i64,
    pub width: u32,
    pub height: u32,
}

pub struct Shelf {
    dir: PathBuf,
    /// Hash of the newest capture, to skip the same image copied twice in a row.
    newest: Option<(PathBuf, String)>,
}

impl Shelf {
    pub fn new(data_dir: &Path) -> Shelf {
        Shelf { dir: data_dir.join("Shelf"), newest: None }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn thumbs(&self) -> PathBuf {
        self.dir.join(".thumbs")
    }

    fn thumb_path(&self, path: &Path) -> PathBuf {
        self.thumbs().join(path.file_name().unwrap_or_default())
    }

    /// Saved screenshots, newest first.
    pub fn items(&self) -> Vec<ShelfItem> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        let mut items: Vec<ShelfItem> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
            .filter_map(|path| {
                let meta = fs::metadata(&path).ok()?;
                let date = meta.created().or_else(|_| meta.modified()).ok()?;
                let (width, height) = png_size(&path).unwrap_or((0, 0));
                Some(ShelfItem { thumb: self.thumb_path(&path), path, date: epoch_ms(date), width, height })
            })
            .collect();
        items.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.path.cmp(&a.path)));
        items
    }

    /// Stores a copied image (PNG bytes). Returns the new item, or None for a repeat of the newest one.
    pub fn save(&mut self, png: &[u8], now: SystemTime) -> std::io::Result<Option<ShelfItem>> {
        let hash = crate::util::hex(&Sha256::digest(png));
        if let Some((path, h)) = &self.newest
            && *h == hash
            && path.exists()
        {
            return Ok(None);
        }
        let pixmap = Pixmap::decode_png(png).map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::create_dir_all(&self.dir)?;
        let path = self.free_name(now);
        fs::write(&path, png)?;
        self.newest = Some((path.clone(), hash));
        let _ = self.write_thumb(&path, &pixmap);
        Ok(Some(ShelfItem {
            thumb: self.thumb_path(&path),
            date: epoch_ms(now),
            width: pixmap.width(),
            height: pixmap.height(),
            path,
        }))
    }

    /// "Screenshot 2026-11-14 at 17.02.33.png", with " (2)" and up when taken.
    fn free_name(&self, now: SystemTime) -> PathBuf {
        let local: chrono::DateTime<chrono::Local> = now.into();
        let base = format!("Screenshot {}", local.format("%Y-%m-%d at %H.%M.%S"));
        let mut path = self.dir.join(format!("{base}.png"));
        let mut n = 2;
        while path.exists() {
            path = self.dir.join(format!("{base} ({n}).png"));
            n += 1;
        }
        path
    }

    /// Makes sure every item has a preview; returns the items whose preview was created now.
    pub fn ensure_thumbs(&self) -> usize {
        let mut made = 0;
        for item in self.items() {
            if item.thumb.exists() {
                continue;
            }
            if let Ok(data) = fs::read(&item.path)
                && let Ok(pixmap) = Pixmap::decode_png(&data)
                && self.write_thumb(&item.path, &pixmap).is_ok()
            {
                made += 1;
            }
        }
        made
    }

    fn write_thumb(&self, path: &Path, pixmap: &Pixmap) -> std::io::Result<()> {
        fs::create_dir_all(self.thumbs())?;
        let thumb = downscale(pixmap, THUMB_PX);
        let png = thumb.encode_png().map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::write(self.thumb_path(path), png)
    }

    /// Deletes screenshots (and their previews). Only files inside the shelf are touched.
    pub fn delete(&mut self, paths: &[PathBuf]) {
        for p in paths {
            if p.parent() != Some(self.dir.as_path()) {
                continue;
            }
            let _ = fs::remove_file(p);
            let _ = fs::remove_file(self.thumb_path(p));
        }
    }

    pub fn clear(&mut self) {
        let all: Vec<PathBuf> = self.items().into_iter().map(|i| i.path).collect();
        self.delete(&all);
    }

    /// Drops screenshots past the retention period (0 keeps them) and beyond `MAX_ITEMS`.
    /// Returns what's left, newest first.
    pub fn prune(&mut self, retention_days: u32, now: SystemTime) -> Vec<ShelfItem> {
        let cutoff = if retention_days > 0 {
            epoch_ms(now - Duration::from_secs(u64::from(retention_days) * 86_400))
        } else {
            i64::MIN
        };
        let (keep, doomed): (Vec<_>, Vec<_>) =
            self.items().into_iter().enumerate().partition(|(i, item)| *i < MAX_ITEMS && item.date >= cutoff);
        let doomed: Vec<PathBuf> = doomed.into_iter().map(|(_, item)| item.path).collect();
        if !doomed.is_empty() {
            self.delete(&doomed);
        }
        keep.into_iter().map(|(_, item)| item).collect()
    }
}

fn epoch_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Width and height from the PNG header, without decoding the image.
fn png_size(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut head = [0u8; 24];
    fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    if &head[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(head[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(head[20..24].try_into().ok()?);
    Some((w, h))
}

/// Scaled down so the longest side is at most `max` pixels (never up).
pub fn downscale(src: &Pixmap, max: u32) -> Pixmap {
    let longest = src.width().max(src.height());
    if longest <= max {
        return src.clone();
    }
    let s = max as f32 / longest as f32;
    let (w, h) = (((src.width() as f32 * s).round() as u32).max(1), ((src.height() as f32 * s).round() as u32).max(1));
    let mut out = Pixmap::new(w, h).expect("non-empty thumbnail");
    out.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint { quality: FilterQuality::Bicubic, ..Default::default() },
        Transform::from_scale(s, s),
        None,
    );
    out
}

// ---- Device-independent bitmaps (the Windows clipboard's CF_DIB / CF_DIBV5) -------------

/// PNG bytes from a packed DIB (BITMAPINFOHEADER or V5 header, then pixels). 24 and 32 bits
/// per pixel, bottom-up or top-down, BI_RGB or BI_BITFIELDS with the usual BGRA masks.
pub fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    let u32_at = |o: usize| dib.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let i32_at = |o: usize| u32_at(o).map(|v| v as i32);
    let header = u32_at(0)? as usize;
    let width = i32_at(4)?;
    let height = i32_at(8)?;
    let bits = dib.get(14..16).map(|b| u16::from_le_bytes([b[0], b[1]]))?;
    let compression = u32_at(16)?;
    if width <= 0 || height == 0 || !(bits == 24 || bits == 32) || !(compression == 0 || compression == 3) {
        return None;
    }
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    // BI_BITFIELDS with a plain BITMAPINFOHEADER keeps its three masks right after it.
    let masks = if compression == 3 && header == 40 { 12 } else { 0 };
    let stride = (w * bits as usize / 8).div_ceil(4) * 4;
    let pixels = dib.get(header + masks..)?;
    if pixels.len() < stride * h {
        return None;
    }
    // Screenshots often leave the alpha byte at 0: treat an all-zero alpha channel as opaque.
    let has_alpha = bits == 32 && (0..h).any(|y| (0..w).any(|x| pixels[y * stride + x * 4 + 3] != 0));
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        let y = if height > 0 { h - 1 - row } else { row };
        let line = &pixels[y * stride..];
        for x in 0..w {
            let p = &line[x * bits as usize / 8..];
            let a = if has_alpha { p[3] } else { 255 };
            rgba.extend_from_slice(&[p[2], p[1], p[0], a]);
        }
    }
    let size = IntSize::from_wh(w as u32, h as u32)?;
    // tiny-skia stores premultiplied pixels.
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(rgba, size)?.encode_png().ok()
}

/// A packed 32-bit bottom-up DIB (BITMAPINFOHEADER + BGRA) of a PNG, for apps that only paste CF_DIB.
pub fn png_to_dib(png: &[u8]) -> Option<Vec<u8>> {
    let pixmap = Pixmap::decode_png(png).ok()?;
    let (w, h) = (pixmap.width() as usize, pixmap.height() as usize);
    let mut out = Vec::with_capacity(40 + w * h * 4);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&((w * h * 4) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // resolution, palette
    for y in (0..h).rev() {
        for x in 0..w {
            let c = pixmap.pixel(x as u32, y as u32)?.demultiply();
            out.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("burn-shelf-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut p = Pixmap::new(w, h).unwrap();
        p.fill(tiny_skia::Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255));
        p.encode_png().unwrap()
    }

    #[test]
    fn saves_skips_repeats_and_prunes() {
        let dir = temp("save");
        let mut shelf = Shelf::new(&dir);
        let t0 = SystemTime::now();
        let a = shelf.save(&png(800, 400, [255, 0, 0]), t0).unwrap().unwrap();
        assert!(a.path.ends_with(format!("{}.png", a.path.file_stem().unwrap().to_string_lossy())));
        assert!(shelf.save(&png(800, 400, [255, 0, 0]), t0).unwrap().is_none(), "same image twice in a row");
        let b = shelf.save(&png(10, 10, [0, 0, 255]), t0).unwrap().unwrap();
        assert_ne!(a.path, b.path, "same second gets a numbered name");
        assert!(b.path.to_string_lossy().ends_with(" (2).png"));
        let items = shelf.items();
        assert_eq!(items.len(), 2);
        assert_eq!((items.iter().find(|i| i.path == a.path).unwrap().width), 800);
        let thumb = Pixmap::load_png(&a.thumb).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (640, 320));

        // Nothing is old enough yet; a far future "now" drops everything.
        shelf.prune(7, SystemTime::now());
        assert_eq!(shelf.items().len(), 2);
        shelf.prune(7, SystemTime::now() + Duration::from_secs(30 * 86_400));
        assert!(shelf.items().is_empty());
        assert!(!a.thumb.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_stays_inside_the_shelf() {
        let dir = temp("delete");
        let outside = dir.join("keep.png");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&outside, b"x").unwrap();
        let mut shelf = Shelf::new(&dir);
        shelf.delete(std::slice::from_ref(&outside));
        assert!(outside.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dib_round_trip() {
        let source = png(3, 2, [10, 200, 30]);
        let dib = png_to_dib(&source).unwrap();
        assert_eq!(dib.len(), 40 + 3 * 2 * 4);
        let back = Pixmap::decode_png(&dib_to_png(&dib).unwrap()).unwrap();
        assert_eq!((back.width(), back.height()), (3, 2));
        let c = back.pixel(1, 1).unwrap().demultiply();
        assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (10, 200, 30, 255));
    }

    #[test]
    fn dib_with_zero_alpha_is_opaque_and_24_bit_works() {
        // 2x1, 32-bit, top-down, alpha bytes 0 (as many screenshot tools write them).
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&(-1i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]);
        dib.extend_from_slice(&[1, 2, 3, 0, 4, 5, 6, 0]);
        let p = Pixmap::decode_png(&dib_to_png(&dib).unwrap()).unwrap();
        let c = p.pixel(0, 0).unwrap().demultiply();
        assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (3, 2, 1, 255));

        // 1x1, 24-bit, bottom-up, row padded to 4 bytes.
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]);
        dib.extend_from_slice(&[9, 8, 7, 0]);
        let p = Pixmap::decode_png(&dib_to_png(&dib).unwrap()).unwrap();
        let c = p.pixel(0, 0).unwrap().demultiply();
        assert_eq!((c.red(), c.green(), c.blue()), (7, 8, 9));
        assert!(dib_to_png(&dib[..30]).is_none(), "truncated");
    }
}
