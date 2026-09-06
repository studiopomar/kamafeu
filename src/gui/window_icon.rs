//! Shared window icon loading for the Studio and Copaiba executables.

use eframe::egui::IconData;

/// Keep the native window icon small enough for an X11 property request.
/// Sending the full 2048px artwork can fail with MaximumRequestLengthExceeded
/// while winit sets _NET_WM_ICON, before the OpenGL context is created.
pub fn load_window_icon() -> Result<IconData, image::ImageError> {
    let image = image::load_from_memory(include_bytes!("../../assets/icon.png"))?
        .thumbnail(128, 128)
        .into_rgba8();

    Ok(IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::load_window_icon;

    #[test]
    fn bundled_icon_fits_a_core_x11_property_request() {
        let icon = load_window_icon().expect("the bundled window icon must decode");
        assert!(icon.width > 0 && icon.height > 0);
        assert_eq!(
            icon.rgba.len(),
            icon.width as usize * icon.height as usize * 4
        );

        // ChangeProperty has a 24-byte header. _NET_WM_ICON adds two u32s
        // (width and height), followed by one 32-bit ARGB value per pixel.
        // Stay below the core protocol's 16-bit request-length ceiling
        // instead of relying on the server's BIG-REQUESTS extension.
        let request_bytes = 24 + 8 + icon.rgba.len();
        assert!(
            request_bytes <= u16::MAX as usize * 4,
            "window icon exceeds a core X11 request: {request_bytes} bytes"
        );
    }
}
