//! Draws the achievement card in software (tiny-skia for shapes, fontdue for text), in the
//! colours of the theme the shelf is using, with the fonts of the interface.

use super::Popup;
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

/// Card size in logical pixels, and the transparent margin around it for the shadow.
pub const CARD_W: f32 = 440.0;
pub const CARD_H: f32 = 112.0;
pub const PAD: f32 = 18.0;
/// Distance from the left edge of the screen to the card when it is in place. The surface
/// starts at the screen edge, so the card can slide in from outside it.
pub const LEFT: f32 = 24.0;
/// Width of the surface: the gap to the screen edge, the card and its shadow margin.
pub const SURFACE_W: f32 = LEFT + CARD_W + PAD;

struct Fonts {
    title: Font,
    label: Font,
    body: Font,
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let load = |bytes: &'static [u8]| Font::from_bytes(bytes, FontSettings::default()).expect("bundled font");
        Fonts {
            title: load(include_bytes!("../../assets/fonts/outfit-700.ttf")),
            label: load(include_bytes!("../../assets/fonts/atk-700.ttf")),
            body: load(include_bytes!("../../assets/fonts/atk-400.ttf")),
        }
    })
}

/// "#rrggbb" or "#rrggbbaa"; anything else is grey.
pub fn parse_color(hex: &str, alpha: f32) -> Color {
    let h = hex.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(0x80);
    let a = if h.len() >= 8 { byte(6) as f32 / 255.0 } else { 1.0 };
    Color::from_rgba8(byte(0), byte(2), byte(4), ((a * alpha).clamp(0.0, 1.0) * 255.0) as u8)
}

fn paint(color: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color);
    p.anti_alias = true;
    p
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Width of a line of text at a size, with extra spacing between letters.
fn text_width(font: &Font, size: f32, text: &str, spacing: f32) -> f32 {
    text.chars().map(|c| font.metrics(c, size).advance_width + spacing).sum::<f32>() - spacing.max(0.0)
}

/// Shortens text with an ellipsis until it fits.
fn fit(font: &Font, size: f32, text: &str, max: f32) -> String {
    if text_width(font, size, text, 0.0) <= max {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() && text_width(font, size, &(chars.iter().collect::<String>() + "…"), 0.0) > max {
        chars.pop();
    }
    chars.iter().collect::<String>().trim_end().to_string() + "…"
}

/// Draws one line of text with its baseline at `y`, scaled by `s`; returns its width.
#[allow(clippy::too_many_arguments)]
fn draw_text(pixmap: &mut Pixmap, font: &Font, size: f32, text: &str, x: f32, y: f32, color: Color, spacing: f32, s: f32, alpha: f32) -> f32 {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    let mut pen = x * s;
    let (r, g, b, a) = (color.red(), color.green(), color.blue(), color.alpha() * alpha);
    let data = pixmap.data_mut();
    for c in text.chars() {
        let (m, bitmap) = font.rasterize(c, size * s);
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (y * s - m.height as f32 - m.ymin as f32).round() as i32;
        for row in 0..m.height as i32 {
            for col in 0..m.width as i32 {
                let (px, py) = (gx + col, gy + row);
                if px < 0 || py < 0 || px >= w || py >= h {
                    continue;
                }
                let cov = bitmap[(row * m.width as i32 + col) as usize] as f32 / 255.0 * a;
                if cov <= 0.0 {
                    continue;
                }
                // Source-over on premultiplied RGBA.
                let i = ((py * w + px) * 4) as usize;
                let inv = 1.0 - cov;
                data[i] = (r * cov * 255.0 + data[i] as f32 * inv) as u8;
                data[i + 1] = (g * cov * 255.0 + data[i + 1] as f32 * inv) as u8;
                data[i + 2] = (b * cov * 255.0 + data[i + 2] as f32 * inv) as u8;
                data[i + 3] = (cov * 255.0 + data[i + 3] as f32 * inv) as u8;
            }
        }
        pen += m.advance_width + spacing * s;
    }
    pen / s - x
}

/// A trophy in strokes, drawn inside a square at (x, y) of side `size`.
fn draw_trophy(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, color: Color, t: Transform) {
    let u = size / 24.0;
    let p = |px: f32, py: f32| (x + px * u, y + py * u);
    let mut pb = PathBuilder::new();
    // cup
    let (ax, ay) = p(7.0, 4.0);
    pb.move_to(ax, ay);
    let (bx, by) = p(17.0, 4.0);
    pb.line_to(bx, by);
    let (cx, cy) = p(17.0, 9.0);
    pb.line_to(cx, cy);
    let (c1x, c1y) = p(17.0, 13.5);
    let (ex, ey) = p(12.0, 14.0);
    pb.quad_to(c1x, c1y, ex, ey);
    let (c2x, c2y) = p(7.0, 13.5);
    let (fx, fy) = p(7.0, 9.0);
    pb.quad_to(c2x, c2y, fx, fy);
    pb.close();
    // handles
    let (hx, hy) = p(7.0, 6.0);
    pb.move_to(hx, hy);
    let (h1x, h1y) = p(3.0, 6.0);
    let (h2x, h2y) = p(7.5, 11.0);
    pb.cubic_to(h1x, h1y, h1x, h2y, h2x, h2y);
    let (kx, ky) = p(17.0, 6.0);
    pb.move_to(kx, ky);
    let (k1x, k1y) = p(21.0, 6.0);
    let (k2x, k2y) = p(16.5, 11.0);
    pb.cubic_to(k1x, k1y, k1x, k2y, k2x, k2y);
    // stem and base
    let (sx, sy) = p(12.0, 14.0);
    pb.move_to(sx, sy);
    let (s2x, s2y) = p(12.0, 17.5);
    pb.line_to(s2x, s2y);
    let (b1x, b1y) = p(8.0, 20.0);
    pb.move_to(b1x, b1y);
    let (b2x, b2y) = p(16.0, 20.0);
    pb.line_to(b2x, b2y);
    let (n1x, n1y) = p(9.5, 17.5);
    pb.move_to(n1x, n1y);
    let (n2x, n2y) = p(14.5, 17.5);
    pb.line_to(n2x, n2y);
    if let Some(path) = pb.finish() {
        let stroke = Stroke { width: 1.9 * u, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint(color), &stroke, t, None);
    }
}

/// Renders the card at scale `s` (device pixels per logical pixel). `progress` is 0 when the
/// card is off to the left and invisible, 1 when fully shown.
pub fn render(popup: &Popup, s: f32, progress: f32) -> Pixmap {
    let (w, h) = ((SURFACE_W * s).ceil() as u32, ((CARD_H + PAD * 2.0) * s).ceil() as u32);
    let mut pixmap = Pixmap::new(w, h).expect("pixmap size");
    if progress <= 0.0 {
        return pixmap;
    }
    let f = fonts();
    let c = &popup.colors;
    // Slides in from the left edge of the screen, decelerating, and back out the same way.
    let eased = 1.0 - (1.0 - progress).powi(3);
    let alpha = (progress * 2.5).min(1.0);
    let slide = (1.0 - eased) * -SURFACE_W;
    let t = Transform::from_scale(s, s);
    let (x0, y0) = (LEFT + slide, PAD);

    // Soft shadow: stacked, widening rounded rectangles.
    for i in 0..6 {
        let grow = i as f32 * 2.0;
        if let Some(path) = rounded_rect(x0 - grow, y0 - grow + 4.0, CARD_W + grow * 2.0, CARD_H + grow * 2.0, 16.0 + grow) {
            pixmap.fill_path(&path, &paint(Color::from_rgba(0.0, 0.0, 0.0, 0.05 * alpha).unwrap()), FillRule::Winding, t, None);
        }
    }
    // Card and accent border.
    if let Some(path) = rounded_rect(x0, y0, CARD_W, CARD_H, 16.0) {
        pixmap.fill_path(&path, &paint(parse_color(&c.panel, 0.97 * alpha)), FillRule::Winding, t, None);
        let stroke = Stroke { width: 1.5, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint(parse_color(&c.accent, 0.6 * alpha)), &stroke, t, None);
    }
    // Icon tile: the achievement's image, or a trophy on the accent colour.
    let (ix, iy, isz) = (x0 + 20.0, y0 + 20.0, 72.0);
    if let Some(path) = rounded_rect(ix, iy, isz, isz, 14.0) {
        pixmap.fill_path(&path, &paint(parse_color(&c.accent, alpha)), FillRule::Winding, t, None);
    }
    let icon = popup.icon.as_deref().and_then(|p| Pixmap::load_png(p).ok());
    match icon {
        Some(img) => {
            let scale = isz / img.width().max(img.height()) as f32;
            let paint = PixmapPaint { opacity: alpha, quality: tiny_skia::FilterQuality::Bicubic, ..PixmapPaint::default() };
            let tt = Transform::from_row(scale * s, 0.0, 0.0, scale * s, ix * s, iy * s);
            if let Some(mask_path) = rounded_rect(ix, iy, isz, isz, 14.0) {
                let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).expect("mask");
                mask.fill_path(&mask_path, FillRule::Winding, true, t);
                pixmap.draw_pixmap(0, 0, img.as_ref(), &paint, tt, Some(&mask));
            }
        }
        None => draw_trophy(&mut pixmap, ix + 14.0, iy + 14.0, isz - 28.0, parse_color(&c.on_accent, alpha), t),
    }
    // Text column.
    let tx = ix + isz + 18.0;
    let right = x0 + CARD_W - 20.0;
    let points = format!("+{}", popup.points);
    let points_w = if popup.points > 0 { text_width(&f.title, 20.0, &points, 0.0) } else { 0.0 };
    if popup.points > 0 {
        draw_text(&mut pixmap, &f.title, 20.0, &points, right - points_w, y0 + 38.0, parse_color(&c.accent, 1.0), 0.0, s, alpha);
    }
    let label = popup.label.to_uppercase();
    draw_text(&mut pixmap, &f.label, 12.5, &fit(&f.label, 12.5, &label, right - tx - points_w - 12.0), tx, y0 + 36.0, parse_color(&c.accent, 1.0), 1.4, s, alpha);
    draw_text(&mut pixmap, &f.title, 23.0, &fit(&f.title, 23.0, &popup.title, right - tx), tx, y0 + 64.0, parse_color(&c.text, 1.0), 0.0, s, alpha);
    draw_text(&mut pixmap, &f.body, 15.0, &fit(&f.body, 15.0, &popup.description, right - tx), tx, y0 + 88.0, parse_color(&c.muted, 1.0), 0.0, s, alpha);
    pixmap
}
