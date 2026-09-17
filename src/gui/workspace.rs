//! Window-manager behavior shared by the modular workspace panes.

use eframe::egui::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspacePane {
    PianoRoll,
    Inspector,
}

/// Returns the OS-style target rectangle when a floating pane is released near
/// an edge of the available workspace. The caller applies the result for one
/// frame with `Window::fixed_rect`; egui then keeps the new size and position.
pub fn snap_target(rect: Rect, workspace: Rect, threshold: f32) -> Option<Rect> {
    if rect.min.y <= workspace.min.y + threshold {
        return Some(workspace);
    }

    let left = rect.min.x <= workspace.min.x + threshold;
    let right = rect.max.x >= workspace.max.x - threshold;
    let bottom = rect.max.y >= workspace.max.y - threshold;
    let half_width = workspace.width() * 0.5;
    let half_height = workspace.height() * 0.5;

    match (left, right, bottom) {
        (true, _, true) => Some(Rect::from_min_size(
            workspace.left_bottom() - eframe::egui::vec2(0.0, half_height),
            eframe::egui::vec2(half_width, half_height),
        )),
        (_, true, true) => Some(Rect::from_min_size(
            workspace.center_bottom() - eframe::egui::vec2(0.0, half_height),
            eframe::egui::vec2(half_width, half_height),
        )),
        (true, _, _) => Some(Rect::from_min_size(
            workspace.min,
            eframe::egui::vec2(half_width, workspace.height()),
        )),
        (_, true, _) => Some(Rect::from_min_size(
            eframe::egui::pos2(workspace.center().x, workspace.min.y),
            eframe::egui::vec2(half_width, workspace.height()),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn snaps_to_full_height_halves_and_maximize() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1_000.0, 800.0));
        assert_eq!(
            snap_target(
                Rect::from_min_size(pos2(4.0, 200.0), vec2(300.0, 300.0)),
                area,
                16.0
            ),
            Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 800.0)))
        );
        assert_eq!(
            snap_target(
                Rect::from_min_size(pos2(700.0, 4.0), vec2(250.0, 300.0)),
                area,
                16.0
            ),
            Some(area)
        );
    }
}
