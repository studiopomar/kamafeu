use eframe::egui::{self, TextureHandle};
use std::path::Path;

/// Loads an image once and keeps its GPU texture in egui's context cache.
///
/// UI code runs every frame, so opening/decoding the file inside a panel would
/// otherwise turn a cheap texture draw into disk I/O, image decoding and a GPU
/// upload 60 times per second.
pub fn texture_for_path(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    let cache_id = egui::Id::new(("kamafeu_image_texture", path));
    if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(cache_id)) {
        return Some(texture);
    }

    let image = image::open(path).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let texture = ctx.load_texture(
        format!("image:{}", path.display()),
        color_image,
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    Some(texture)
}
