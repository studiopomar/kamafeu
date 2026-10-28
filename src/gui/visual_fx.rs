//! Small immediate-mode visual effects shared by the editor chrome.
//!
//! Everything here is pure painting: no state is stored outside egui's own
//! animation memory, and every continuous effect takes an explicit `motion`
//! flag so callers can honour `AnimationPolicy::allows_motion()` (reduced
//! motion, low frame rate, or animations disabled in the preferences).

use eframe::egui::{
    self, epaint::Shadow, Color32, Mesh, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2,
};
use std::f32::consts::TAU;

// ───────────────────────────── colour helpers ─────────────────────────────

/// Replace the alpha of a colour, working in unmultiplied space so theme
/// colours keep their hue when faded.
pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let alpha = (a as f32 / 255.0) * alpha.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0).round() as u8)
}

pub fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    from.lerp_to_gamma(to, t.clamp(0.0, 1.0))
}

/// Brighten a colour toward white without losing its identity.
pub fn lighten(color: Color32, amount: f32) -> Color32 {
    let [_, _, _, a] = color.to_srgba_unmultiplied();
    let white = Color32::from_rgba_unmultiplied(255, 255, 255, a);
    lerp_color(color, white, amount)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}

pub fn ease_in_out_sine(t: f32) -> f32 {
    -((std::f32::consts::PI * t.clamp(0.0, 1.0)).cos() - 1.0) / 2.0
}

/// Smooth 0..1 "breathing" wave. Returns a constant midpoint when motion is
/// not allowed so callers never need two code paths.
pub fn breathe(time: f64, hz: f32, motion: bool) -> f32 {
    if !motion {
        return 0.5;
    }
    ease_in_out_sine(0.5 + 0.5 * ((time as f32) * TAU * hz).sin())
}

// ──────────────────────────── shape primitives ────────────────────────────

/// Reserve a slot *below* a widget that is about to be drawn. Fill it later
/// with [`paint_glow_into`] so the glow sits behind the widget instead of
/// washing over it.
pub fn reserve(painter: &Painter) -> egui::layers::ShapeIdx {
    painter.add(Shape::Noop)
}

/// Soft blurred halo around `rect`, rendered into a slot obtained from
/// [`reserve`].
pub fn paint_glow_into(
    painter: &Painter,
    slot: egui::layers::ShapeIdx,
    rect: Rect,
    rounding: impl Into<Rounding>,
    color: Color32,
    blur: f32,
) {
    if color.a() == 0 || blur <= 0.0 {
        return;
    }
    let shadow = Shadow {
        offset: Vec2::ZERO,
        blur,
        spread: blur * 0.15,
        color,
    };
    painter.set(slot, shadow.as_shape(rect, rounding));
}

/// Soft blurred halo painted immediately (use for elements drawn by hand
/// where the glow can be painted first).
pub fn paint_glow(
    painter: &Painter,
    rect: Rect,
    rounding: impl Into<Rounding>,
    color: Color32,
    blur: f32,
) {
    let slot = reserve(painter);
    paint_glow_into(painter, slot, rect, rounding, color, blur);
}

/// Horizontal multi-stop gradient. `phase` rotates the stops (0..1), which
/// produces a flowing effect when driven by time.
#[allow(dead_code)]
pub fn paint_horizontal_gradient(painter: &Painter, rect: Rect, stops: &[Color32], phase: f32) {
    if stops.is_empty() || rect.width() <= 0.0 {
        return;
    }
    if stops.len() == 1 {
        painter.rect_filled(rect, 0.0, stops[0]);
        return;
    }
    // Sample the cyclic gradient at fixed columns; enough columns hide the
    // seam while keeping the mesh tiny.
    let columns = 24usize;
    let sample = |u: f32| -> Color32 {
        let u = (u + phase).rem_euclid(1.0) * stops.len() as f32;
        let i = u.floor() as usize % stops.len();
        let j = (i + 1) % stops.len();
        lerp_color(stops[i], stops[j], u.fract())
    };
    let mut mesh = Mesh::default();
    for column in 0..=columns {
        let u = column as f32 / columns as f32;
        let x = egui::lerp(rect.left()..=rect.right(), u);
        let color = sample(u);
        mesh.colored_vertex(Pos2::new(x, rect.top()), color);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), color);
        if column > 0 {
            let base = (column as u32 - 1) * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 2, base + 3);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// Vertical two-colour gradient (top → bottom).
pub fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// A light band sweeping left → right across `rect`, clipped to it.
pub fn paint_shimmer(painter: &Painter, rect: Rect, time: f64, color: Color32, period_s: f32) {
    if rect.width() <= 1.0 {
        return;
    }
    let band = (rect.width() * 0.35).clamp(18.0, 140.0);
    let t = ((time as f32) / period_s.max(0.1)).fract();
    let center = rect.left() - band + t * (rect.width() + band * 2.0);
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let clear = with_alpha(color, 0.0);
    let mut mesh = Mesh::default();
    for (x, c) in [
        (center - band, clear),
        (center, color),
        (center + band, clear),
    ] {
        mesh.colored_vertex(Pos2::new(x, rect.top()), c);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    mesh.add_triangle(2, 3, 4);
    mesh.add_triangle(3, 4, 5);
    painter.add(Shape::mesh(mesh));
}

/// Eight dots orbiting a centre with a fading tail – a compact activity
/// indicator that reads well at 10–16 px.
pub fn paint_orbit_spinner(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    time: f64,
    color: Color32,
    motion: bool,
) {
    let dots = 8;
    let head = if motion {
        ((time as f32) * 1.4).fract() * dots as f32
    } else {
        0.0
    };
    for i in 0..dots {
        let angle = i as f32 / dots as f32 * TAU - std::f32::consts::FRAC_PI_2;
        let behind = (head - i as f32).rem_euclid(dots as f32) / dots as f32;
        let strength = 1.0 - behind;
        let pos = center + Vec2::angled(angle) * radius;
        painter.circle_filled(
            pos,
            (radius * 0.22) * (0.55 + 0.45 * strength),
            with_alpha(color, 0.15 + 0.85 * strength * strength),
        );
    }
}

// ─────────────────────────────── meters ───────────────────────────────

pub struct MeterPalette {
    pub low: Color32,
    pub mid: Color32,
    pub high: Color32,
    pub off: Color32,
}

/// Segmented LED meter (horizontal). Segments light with a green → amber →
/// red gradient, the most recent lit segment glows, and the peak-hold LED is
/// drawn as a bright tick.
pub fn paint_led_meter(
    painter: &Painter,
    rect: Rect,
    level: f32,
    peak: f32,
    palette: &MeterPalette,
) {
    let segments = ((rect.width() / 4.0).floor() as usize).clamp(6, 48);
    let gap = 1.0;
    let seg_w = (rect.width() - gap * (segments as f32 - 1.0)) / segments as f32;
    let level = level.clamp(0.0, 1.0);
    let peak = peak.clamp(0.0, 1.0);
    let lit = (level * segments as f32).ceil() as usize;
    let peak_seg = ((peak * segments as f32).ceil() as usize).saturating_sub(1);
    let color_at = |u: f32| -> Color32 {
        if u < 0.65 {
            lerp_color(palette.low, palette.mid, (u / 0.65).powf(2.2))
        } else if u < 0.85 {
            lerp_color(palette.mid, palette.high, (u - 0.65) / 0.20)
        } else {
            palette.high
        }
    };
    for i in 0..segments {
        let u = (i as f32 + 0.5) / segments as f32;
        let x = rect.left() + i as f32 * (seg_w + gap);
        let seg = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(seg_w, rect.height()));
        if i < lit {
            let c = color_at(u);
            if i + 1 == lit && level > 0.02 {
                paint_glow(painter, seg, 1.0, with_alpha(c, 0.55), 6.0);
            }
            painter.rect_filled(seg, 1.0, c);
            // Glassy highlight on the upper half of each lit LED.
            let top = Rect::from_min_max(seg.min, Pos2::new(seg.max.x, seg.center().y));
            painter.rect_filled(top, 1.0, with_alpha(Color32::WHITE, 0.14));
        } else if i == peak_seg && peak > 0.03 {
            painter.rect_filled(seg, 1.0, lighten(color_at(u), 0.25));
        } else {
            painter.rect_filled(seg, 1.0, palette.off);
        }
    }
}

// ─────────────────────────────── scope ───────────────────────────────

/// Mirrored min/max waveform with a soft filled body and bright outline.
/// `samples` is (min, max) in -1..1, oldest first.
pub fn paint_scope(
    painter: &Painter,
    rect: Rect,
    samples: &[(f32, f32)],
    color: Color32,
    gain: f32,
) {
    let mid = rect.center().y;
    let half = rect.height() * 0.5 - 1.0;
    painter.line_segment(
        [Pos2::new(rect.left(), mid), Pos2::new(rect.right(), mid)],
        Stroke::new(0.6_f32, with_alpha(color, 0.25)),
    );
    if samples.len() < 2 {
        return;
    }
    let n = samples.len();
    let x_at = |i: usize| egui::lerp(rect.left()..=rect.right(), i as f32 / (n - 1) as f32);
    let mut mesh = Mesh::default();
    let mut top_line = Vec::with_capacity(n);
    let mut bottom_line = Vec::with_capacity(n);
    for (i, &(min, max)) in samples.iter().enumerate() {
        // Fade the oldest part of the trace so it reads as motion.
        let age = i as f32 / (n - 1) as f32;
        let fill = with_alpha(color, 0.10 + 0.35 * age);
        let x = x_at(i);
        let y_top = mid - (max * gain).clamp(-1.0, 1.0) * half;
        let y_bot = mid - (min * gain).clamp(-1.0, 1.0) * half;
        let (y_top, y_bot) = (y_top.min(mid - 0.5), y_bot.max(mid + 0.5));
        mesh.colored_vertex(Pos2::new(x, y_top), fill);
        mesh.colored_vertex(Pos2::new(x, y_bot), fill);
        if i > 0 {
            let base = (i as u32 - 1) * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 2, base + 3);
        }
        top_line.push(Pos2::new(x, y_top));
        bottom_line.push(Pos2::new(x, y_bot));
    }
    painter.add(Shape::mesh(mesh));
    let stroke = Stroke::new(1.0_f32, with_alpha(color, 0.9));
    painter.add(Shape::line(top_line, stroke));
    painter.add(Shape::line(bottom_line, stroke));
}

// ─────────────────────────── animated helpers ───────────────────────────

/// Smoothly animate a rectangle toward `target` using egui's animation
/// memory. Used for sliding selection indicators.
pub fn animate_rect(ctx: &egui::Context, id: egui::Id, target: Rect, seconds: f32) -> Rect {
    let l = ctx.animate_value_with_time(id.with("l"), target.left(), seconds);
    let r = ctx.animate_value_with_time(id.with("r"), target.right(), seconds);
    let t = ctx.animate_value_with_time(id.with("t"), target.top(), seconds);
    let b = ctx.animate_value_with_time(id.with("b"), target.bottom(), seconds);
    Rect::from_min_max(Pos2::new(l, t), Pos2::new(r, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_alpha_scales_existing_alpha() {
        let c = Color32::from_rgba_unmultiplied(10, 20, 30, 255);
        assert_eq!(with_alpha(c, 0.5).to_srgba_unmultiplied()[3], 128);
        assert_eq!(with_alpha(c, 0.0).a(), 0);
    }

    #[test]
    fn easing_is_bounded() {
        for t in [-1.0, 0.0, 0.25, 0.5, 1.0, 2.0] {
            let v = ease_out_cubic(t);
            assert!((0.0..=1.0).contains(&v));
            let v = ease_in_out_sine(t);
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn breathe_is_static_without_motion() {
        assert_eq!(breathe(12.3, 1.0, false), 0.5);
    }
}
