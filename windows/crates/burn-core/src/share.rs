//! The share card: an image of the period's spend, copied to the clipboard as PNG. A port of
//! the Mac app's `ShareCard.swift`, drawn as an SVG string and rasterized with resvg. Always the
//! light theme, with bundled fonts so the result never depends on what is installed.

use crate::model::{Period, ProviderId, UsageBucket, UsageSummary};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Datelike};
use resvg::{tiny_skia, usvg};
use std::fmt::Write as _;
use std::sync::{Arc, LazyLock};

/// Card size in points; the PNG is this times the render scale.
pub const WIDTH: f32 = 600.0;
pub const HEIGHT: f32 = 340.0;

const BASE: &str = "#F7F5F2";
const SECONDARY: &str = "#7A746C";
const LIME: &str = "#DFF26B";
const LIME_INK: &str = "#1A1D06";

const FONT_MEDIUM: &[u8] = include_bytes!("../../../../assets/Fonts/BricolageGrotesque12pt-Medium.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../../assets/Fonts/BricolageGrotesque12pt-Bold.ttf");
const FONT_DISPLAY: &[u8] = include_bytes!("../../../../assets/Fonts/BricolageGrotesque96pt-ExtraBold.ttf");

const GLYPH_CLAUDE: &str = include_str!("../../burn-taskbar/assets/claude.svg");
const GLYPH_OPENAI: &str = include_str!("../../burn-taskbar/assets/openai.svg");
const GLYPH_CURSOR: &str = include_str!("../../burn-taskbar/assets/cursor.svg");

/// Family names inside the font files (checked by a test).
const TEXT: &str = "Bricolage Grotesque 12pt";
const DISPLAY: &str = "Bricolage Grotesque 96pt";

static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut db = usvg::fontdb::Database::new();
    for data in [FONT_MEDIUM, FONT_BOLD, FONT_DISPLAY] {
        db.load_font_data(data.to_vec());
    }
    Arc::new(db)
});

fn options() -> usvg::Options<'static> {
    usvg::Options { fontdb: FONTS.clone(), ..Default::default() }
}

/// Renders the card at `600x340` points times `scale` and returns PNG bytes.
pub fn render_png(usage: &UsageSummary, scale: f32) -> Result<Vec<u8>> {
    let svg = card_svg(usage);
    let tree = usvg::Tree::from_str(&svg, &options())?;
    let (w, h) = ((WIDTH * scale).round() as u32, (HEIGHT * scale).round() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(|| anyhow!("bad card size"))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(pixmap.encode_png()?)
}

// MARK: formatting

/// "$12.34", or "$1,234" from $100 up, like the Mac app's `Format.money`.
pub fn money(v: f64) -> String {
    let digits = if v >= 100.0 { 0 } else { 2 };
    let s = format!("{:.*}", digits, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), format!(".{f}")),
        None => (s.clone(), String::new()),
    };
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{}${grouped}{frac}", if v < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" })
}

/// "1.3B", "245M", "12K", "999".
pub fn tokens(v: i64) -> String {
    let d = v as f64;
    if d >= 1e9 {
        format!("{:.1}B", d / 1e9)
    } else if d >= 1e6 {
        format!("{:.0}M", d / 1e6)
    } else if d >= 1e3 {
        format!("{:.0}K", d / 1e3)
    } else {
        v.to_string()
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// "Today", "Last 7 days", "Last 30 days", "Since November 2026".
pub fn period_phrase(usage: &UsageSummary) -> String {
    match usage.period {
        Period::Today => "Today".into(),
        Period::Week => "Last 7 days".into(),
        Period::Month => "Last 30 days".into(),
        Period::All => {
            let first = usage.buckets.first().and_then(|b| DateTime::from_timestamp_millis(b.start));
            match first {
                Some(d) => {
                    let d = d.with_timezone(&chrono::Local);
                    format!("Since {} {}", MONTHS[d.month0() as usize], d.year())
                }
                None => "All time".into(),
            }
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// MARK: layout

struct Card {
    fill: &'static str,
    ink: &'static str,
}

fn card_colors(p: ProviderId) -> Card {
    match p {
        ProviderId::Claude => Card { fill: "#D97757", ink: "#FFFFFF" },
        ProviderId::Codex => Card { fill: "#141414", ink: "#FFFFFF" },
        ProviderId::Cursor => Card { fill: "#C9C5FF", ink: "#1B1840" },
    }
}

fn glyph_path(p: ProviderId) -> &'static str {
    let src = match p {
        ProviderId::Claude => GLYPH_CLAUDE,
        ProviderId::Codex => GLYPH_OPENAI,
        ProviderId::Cursor => GLYPH_CURSOR,
    };
    let start = src.find(" d=\"").map(|i| i + 4).unwrap_or(0);
    let end = src[start..].find('"').map(|i| start + i).unwrap_or(start);
    &src[start..end]
}

/// Width of `text` laid out at `size` in the given face, measured by parsing a one-line SVG.
fn text_width(text: &str, family: &str, weight: u32, size: f32, spacing: f32) -> f32 {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="4000" height="400"><text id="t" x="0" y="200" font-family="'{family}'" font-weight="{weight}" font-size="{size}" letter-spacing="{spacing}">{}</text></svg>"#,
        escape(text)
    );
    usvg::Tree::from_str(&svg, &options()).map(|t| t.root().bounding_box().map_or(0.0, |r| r.width())).unwrap_or(0.0)
}

/// Font size that keeps `text` within `avail`, down to `min_factor` of `size` (SwiftUI's
/// `minimumScaleFactor`). Kerning scales with the size.
fn fit(text: &str, size: f32, spacing: f32, avail: f32, min_factor: f32) -> (f32, f32) {
    let w = text_width(text, DISPLAY, 800, size, spacing);
    if w <= avail || w == 0.0 {
        return (size, spacing);
    }
    let f = (avail / w).max(min_factor);
    (size * f, spacing * f)
}

#[allow(clippy::too_many_arguments)]
fn text(out: &mut String, x: f32, y: f32, size: f32, weight: u32, fill: &str, opacity: f32, anchor: &str, body: &str) {
    let _ = write!(
        out,
        r#"<text x="{x}" y="{y}" font-family="'{TEXT}'" font-weight="{weight}" font-size="{size}" fill="{fill}" fill-opacity="{opacity}" text-anchor="{anchor}">{}</text>"#,
        escape(body)
    );
}

#[allow(clippy::too_many_arguments)]
fn display(out: &mut String, x: f32, y: f32, size: f32, spacing: f32, fill: &str, body: &str) {
    let _ = write!(
        out,
        r#"<text x="{x}" y="{y}" font-family="'{DISPLAY}'" font-weight="800" font-size="{size}" letter-spacing="{spacing}" fill="{fill}">{}</text>"#,
        escape(body)
    );
}

fn glyph(out: &mut String, p: ProviderId, x: f32, y: f32, size: f32, ink: &str) {
    let k = size / 24.0;
    let _ = write!(out, r#"<path transform="translate({x} {y}) scale({k})" fill="{ink}" d="{}"/>"#, glyph_path(p));
}

fn flame(out: &mut String, x: f32, y: f32, size: f32, fill: &str) {
    // SF Symbols' flame.fill is not available, so a simple flame silhouette on a 24 grid.
    let k = size / 24.0;
    let _ = write!(
        out,
        r#"<path transform="translate({x} {y}) scale({k})" fill="{fill}" d="M12.6 1.2c.4 3.3-1.3 5.2-3 7.2C7.9 10.4 6 12.4 6 15.6 6 19.4 8.7 22.5 12 22.5s6-3.1 6-6.9c0-2.5-1-4.4-2.2-5.9-.3 1.3-1 2.2-1.9 2.7.5-3.9-.4-8.5-1.3-11.2z"/>"#
    );
}

fn bars(out: &mut String, usage: &UsageSummary, x: f32, y: f32, w: f32, h: f32, ink: &str) {
    const SHADES: [(ProviderId, f32); 3] =
        [(ProviderId::Claude, 1.0), (ProviderId::Codex, 0.45), (ProviderId::Cursor, 0.25)];
    let buckets: &[UsageBucket] = &usage.buckets;
    if buckets.is_empty() {
        return;
    }
    let peak = buckets.iter().map(UsageBucket::total).fold(0.0_f64, f64::max).max(f64::MIN_POSITIVE);
    let n = buckets.len() as f32;
    let slot = w / n;
    let spacing = if slot > 6.0 {
        2.0
    } else if slot > 3.0 {
        1.0
    } else {
        0.0
    };
    let bw = (w - spacing * (n - 1.0)) / n;
    for (i, b) in buckets.iter().enumerate() {
        let bx = x + i as f32 * (bw + spacing);
        let mut segs: Vec<(f32, f32)> = Vec::new(); // (height, opacity), bottom first
        for (p, shade) in SHADES {
            let c = b.cost.get(&p).copied().unwrap_or(0.0);
            if c > 0.0 {
                segs.push(((((h as f64) * c / peak) as f32).max(1.0), shade));
            }
        }
        if b.total() == 0.0 {
            segs.push((1.0, 0.15));
        }
        let total: f32 = segs.iter().map(|s| s.0).sum::<f32>().min(h);
        let top = y + h - total;
        let id = format!("c{i}");
        let r = 1.5_f32.min(bw / 2.0).min(total);
        let _ = write!(
            out,
            r#"<clipPath id="{id}"><path d="M{bx} {bot}V{t1}Q{bx} {top} {x1} {top}H{x2}Q{xr} {top} {xr} {t1}V{bot}Z"/></clipPath><g clip-path="url(#{id})">"#,
            bot = y + h,
            t1 = top + r,
            x1 = bx + r,
            x2 = bx + bw - r,
            xr = bx + bw,
        );
        let mut bottom = y + h;
        for (sh, op) in segs {
            bottom -= sh;
            let _ = write!(
                out,
                r#"<rect x="{bx}" y="{bottom}" width="{bw}" height="{sh}" fill="{ink}" fill-opacity="{op}"/>"#
            );
        }
        out.push_str("</g>");
    }
}

fn rrect(out: &mut String, x: f32, y: f32, w: f32, h: f32, r: f32, fill: &str) {
    let _ = write!(out, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"/>"#);
}

fn card_svg(usage: &UsageSummary) -> String {
    let split: Vec<(ProviderId, f64)> = ProviderId::ALL
        .into_iter()
        .filter_map(|p| usage.by_provider.get(&p).copied().filter(|c| *c > 0.0).map(|c| (p, c)))
        .collect();

    let pad = 14.0;
    let footer_h = 16.0;
    let row_h = HEIGHT - 2.0 * pad - 10.0 - footer_h;
    let side_w = 196.0;
    let total_w = if split.len() > 1 { WIDTH - 2.0 * pad - 8.0 - side_w } else { WIDTH - 2.0 * pad };

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}"><rect width="{WIDTH}" height="{HEIGHT}" fill="{BASE}"/>"#
    );

    // Total block.
    let (bx, by) = (pad, pad);
    rrect(&mut s, bx, by, total_w, row_h, 20.0, LIME);
    let (ix, inner_w) = (bx + 18.0, total_w - 36.0);
    text(&mut s, ix, by + 18.0 + 14.0, 15.0, 700, LIME_INK, 1.0, "start", "Burned at API prices");
    let phrase = if split.len() == 1 {
        format!("{} · {}", period_phrase(usage), split[0].0.name())
    } else {
        period_phrase(usage)
    };
    text(&mut s, ix, by + 18.0 + 18.0 + 12.0, 13.0, 500, LIME_INK, 0.65, "start", &phrase);
    let bars_y = by + row_h - 18.0 - 52.0;
    bars(&mut s, usage, ix, bars_y, inner_w, 52.0, LIME_INK);
    let tokens_base = bars_y - 14.0 - 4.0;
    text(&mut s, ix, tokens_base, 13.0, 500, LIME_INK, 0.7, "start", &format!("{} tokens", tokens(usage.tokens)));
    let m = money(usage.cost);
    let (size, spacing) = fit(&m, 72.0, -2.2, inner_w, 0.5);
    display(&mut s, ix, tokens_base - 16.0 - size * 0.2, size, spacing, LIME_INK, &m);

    // Provider blocks.
    if split.len() > 1 {
        let n = split.len() as f32;
        let h = (row_h - 8.0 * (n - 1.0)) / n;
        let x = pad + total_w + 8.0;
        for (i, (p, cost)) in split.iter().enumerate() {
            let y = pad + i as f32 * (h + 8.0);
            let c = card_colors(*p);
            rrect(&mut s, x, y, side_w, h, 20.0, c.fill);
            let ix = x + 16.0;
            glyph(&mut s, *p, ix, y + 16.0 + 1.0, 14.0, c.ink);
            text(&mut s, ix + 20.0, y + 16.0 + 12.0, 14.0, 700, c.ink, 1.0, "start", p.name());
            let share = if usage.cost > 0.0 { (cost / usage.cost * 100.0).round() as i64 } else { 0 };
            let bottom = y + h - 16.0;
            text(&mut s, ix, bottom - 3.0, 12.0, 500, c.ink, 0.75, "start", &format!("{share}% of spend"));
            let m = money(*cost);
            let (size, spacing) = fit(&m, 32.0, -0.9, side_w - 32.0, 0.6);
            display(&mut s, ix, bottom - 17.0, size, spacing, c.ink, &m);
        }
    }

    // Footer.
    let fy = pad + row_h + 10.0;
    flame(&mut s, pad + 6.0, fy + 2.0, 11.0, SECONDARY);
    text(&mut s, pad + 6.0 + 16.0, fy + 12.0, 12.0, 700, SECONDARY, 1.0, "start", "Burn");
    text(&mut s, WIDTH - pad - 6.0, fy + 12.0, 12.0, 500, SECONDARY, 1.0, "end", "github.com/noyex/burn");

    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BucketUnit;
    use std::collections::BTreeMap;

    const DAY: i64 = 86_400_000;
    // 2026-11-02 12:00 UTC
    const T0: i64 = 1_793_620_800_000;

    fn summary(period: Period, providers: &[(ProviderId, f64)], days: i64) -> UsageSummary {
        let buckets = (0..days)
            .map(|i| UsageBucket {
                start: T0 + i * DAY,
                cost: providers
                    .iter()
                    .filter(|_| i % 4 != 3)
                    .map(|(p, c)| (*p, c * (1.0 + (i % 5) as f64) / 10.0))
                    .collect(),
            })
            .collect();
        UsageSummary {
            period,
            cost: providers.iter().map(|p| p.1).sum(),
            tokens: 1_300_000_000,
            by_provider: providers.iter().copied().collect::<BTreeMap<_, _>>(),
            buckets,
            unit: BucketUnit::Day,
        }
    }

    fn multi() -> UsageSummary {
        summary(Period::Week, &[(ProviderId::Claude, 612.4), (ProviderId::Codex, 88.15), (ProviderId::Cursor, 12.0)], 7)
    }

    fn single() -> UsageSummary {
        summary(Period::All, &[(ProviderId::Claude, 1234.5)], 40)
    }

    fn decode(png: &[u8]) -> tiny_skia::Pixmap {
        tiny_skia::Pixmap::decode_png(png).unwrap()
    }

    #[test]
    fn fonts_resolve_to_bundled_families() {
        let families: Vec<String> = FONTS.faces().flat_map(|f| f.families.iter().map(|(n, _)| n.clone())).collect();
        assert!(families.iter().any(|f| f == TEXT), "{families:?}");
        assert!(families.iter().any(|f| f == DISPLAY), "{families:?}");
    }

    #[test]
    fn renders_multi_provider_week() {
        let img = decode(&render_png(&multi(), 2.0).unwrap());
        assert_eq!((img.width(), img.height()), (1200, 680));
        let first = img.pixels()[0];
        assert!(img.pixels().iter().any(|p| *p != first), "card is blank");
    }

    #[test]
    fn renders_single_provider_all_time() {
        let img = decode(&render_png(&single(), 2.0).unwrap());
        assert_eq!((img.width(), img.height()), (1200, 680));
        let first = img.pixels()[0];
        assert!(img.pixels().iter().any(|p| *p != first), "card is blank");
    }

    #[test]
    fn money_formats_like_the_mac_app() {
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(12.3), "$12.30");
        assert_eq!(money(99.99), "$99.99");
        assert_eq!(money(100.0), "$100");
        assert_eq!(money(1234.6), "$1,235");
        assert_eq!(money(1_234_567.0), "$1,234,567");
    }

    #[test]
    fn tokens_format_like_the_mac_app() {
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(12_345), "12K");
        assert_eq!(tokens(245_000_000), "245M");
        assert_eq!(tokens(1_300_000_000), "1.3B");
    }

    #[test]
    fn period_phrases() {
        assert_eq!(period_phrase(&summary(Period::Today, &[], 1)), "Today");
        assert_eq!(period_phrase(&summary(Period::Week, &[], 7)), "Last 7 days");
        assert_eq!(period_phrase(&summary(Period::Month, &[], 30)), "Last 30 days");
        assert_eq!(period_phrase(&summary(Period::All, &[], 0)), "All time");
        let p = period_phrase(&single());
        assert!(p.starts_with("Since November 2026") || p.starts_with("Since October 2026"), "{p}");
    }

    #[test]
    fn long_money_shrinks_to_fit() {
        let (size, _) = fit("$123,456,789", 72.0, -2.2, 100.0, 0.5);
        assert!((36.0..72.0).contains(&size), "{size}");
    }

    #[test]
    #[ignore = "writes sample PNGs to the temp dir for inspection"]
    fn write_samples() {
        for (name, u) in [("multi", multi()), ("single", single())] {
            let path = std::env::temp_dir().join(format!("burn-share-{name}.png"));
            let t = std::time::Instant::now();
            let png = render_png(&u, 2.0).unwrap();
            println!("{} ({} bytes, {:?})", path.display(), png.len(), t.elapsed());
            std::fs::write(&path, png).unwrap();
        }
    }
}
