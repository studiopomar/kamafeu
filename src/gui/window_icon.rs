//! Shared window icon loading for the Studio and Copaiba executables.

use eframe::egui::IconData;

/// Keep the native window icon small enough for an X11 property request.
/// Sending the full 2048px artwork can fail with MaximumRequestLengthExceeded
/// while winit sets _NET_WM_ICON, before the OpenGL context is created.
pub fn load_window_icon() -> Result<IconData, image::ImageError> {
    let artwork = image::load_from_memory(include_bytes!("../../assets/icon.png"))?
        .thumbnail(104, 104)
        .into_rgba8();

    // Use a solid white, rounded tile so the artwork remains legible on both
    // dark and light desktop themes.  Keep the outside of the tile transparent
    // for platforms that support shaped window icons.
    let size = 128u32;
    let tile_margin = 12u32;
    let tile_size = size - tile_margin * 2;
    let radius = 17u32;
    let mut image = image::RgbaImage::from_pixel(size, size, image::Rgba([255, 255, 255, 0]));
    for y in tile_margin..(tile_margin + tile_size) {
        for x in tile_margin..(tile_margin + tile_size) {
            let dx = if x < tile_margin + radius {
                tile_margin + radius - x
            } else if x >= tile_margin + tile_size - radius {
                x - (tile_margin + tile_size - radius - 1)
            } else {
                0
            };
            let dy = if y < tile_margin + radius {
                tile_margin + radius - y
            } else if y >= tile_margin + tile_size - radius {
                y - (tile_margin + tile_size - radius - 1)
            } else {
                0
            };
            if dx > 0 && dy > 0 && dx * dx + dy * dy > radius * radius {
                image.put_pixel(x, y, image::Rgba([255, 255, 255, 0]));
            } else {
                image.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
    }
    let artwork_offset = ((size - artwork.width()) / 2, (size - artwork.height()) / 2);
    image::imageops::overlay(&mut image, &artwork, artwork_offset.0 as i64, artwork_offset.1 as i64);

    Ok(IconData {
        width: size,
        height: size,
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
