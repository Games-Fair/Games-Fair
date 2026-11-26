// SPDX-License-Identifier: GPL-2.0-or-later
//! Decode bundled sprite sheets once per mounted game, on all canvas backends.
use day_core::Bitmap;
use day_reactive::{Scope, Signal};
use day_spec::AssetName;
use std::sync::Arc;
pub fn load(asset: AssetName) -> Signal<Option<Bitmap>> {
    let bitmap = Signal::new(None);
    let task = day_core::task(async move {
        if let Some(bytes) = day_spec::resource(asset)
            && let Ok(decoded) = day_core::image::decode(Arc::new(bytes.to_vec())).await
        {
            bitmap.set(Some(decoded));
        }
    });
    Scope::current().on_cleanup(move || task.abort());
    bitmap
}

/// Arcade team inks shared by the colored sprite illustrations.
pub const TEAM_INKS: [u32; 4] = [0xff6c63, 0x35c5e3, 0x92cd52, 0xffc54e];

#[derive(Clone, Copy)]
pub enum Ink {
    Original,
    Team(u32),
    Shadow,
}

/// Per-mounted-piece bitmap. Rendering happens on first use and when a larger
/// drawing is needed. Small layout changes and animation frames reuse the bitmap.
pub struct VectorSprite {
    asset: AssetName,
    ink: Ink,
    pixel_width: std::cell::Cell<u32>,
    bitmap: Signal<Option<Bitmap>>,
    pending: std::cell::Cell<Option<day_core::TaskHandle>>,
}

impl VectorSprite {
    pub fn new(asset: AssetName, ink: Ink) -> Self {
        Self {
            asset,
            ink,
            pixel_width: 0.into(),
            bitmap: Signal::new(None),
            pending: None.into(),
        }
    }

    pub fn at_width(&self, points: f64) -> Option<Bitmap> {
        // Supersample at 3x for phone Retina and HiDPI desktops; round up to
        // 64px steps so interactive resizes do not start a decode every frame.
        let width = ((points * 3.0 / 64.0).ceil() as u32).min(16) * 64;
        if width > self.pixel_width.get() {
            if let Some(previous) = self.pending.take() {
                previous.abort();
            }
            self.pixel_width.set(width);
            let asset = self.asset.clone();
            let ink = self.ink;
            let bitmap = self.bitmap;
            let task = day_core::task(async move {
                // Resource access and bitmap installation belong to the toolkit thread.
                // Move the CPU work off that thread on native hosts. Web runs it once
                // per requested size on its single-threaded executor.
                let Some(source) = day_spec::resource(asset) else {
                    return;
                };
                let svg = source.as_slice().to_vec();
                if let Ok(Some(png)) =
                    day_core::spawn_blocking(move || rasterize(&svg, ink, width)).await
                    && let Ok(image) = day_core::image::decode(Arc::new(png)).await
                {
                    bitmap.set(Some(image));
                }
            });
            self.pending.set(Some(task));
        }
        self.bitmap.get()
    }
}

impl Drop for VectorSprite {
    fn drop(&mut self) {
        if let Some(task) = self.pending.take() {
            task.abort();
        }
    }
}

// resvg handles paths and gradients identically across toolkits. Day's portable
// bitmap constructor accepts encoded bytes; this PNG exists only in memory.
fn rasterize(source: &[u8], ink: Ink, width: u32) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};
    let mut options = usvg::Options::default();
    if let Ink::Team(rgb) = ink {
        options.style_sheet = Some(format!("svg {{ color: #{rgb:06x}; }}"));
    }
    let tree = usvg::Tree::from_data(source, &options).ok()?;
    let scale = width as f32 / tree.size().width();
    let height = (tree.size().height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    if let Ink::Shadow = ink {
        for pixel in pixmap.pixels_mut() {
            let alpha = pixel.alpha();
            *pixel = tiny_skia::ColorU8::from_rgba(
                23,
                35,
                54,
                ((u16::from(alpha) * 22 + 50) / 100) as u8,
            )
            .premultiply();
        }
    }
    pixmap.encode_png().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_color_changes_only_the_grip_and_shadow_keeps_coverage() {
        // Synthetic two-part illustration using the same fill conventions as the assets.
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 20"><rect width="10" height="8" fill="#abcdef"/><rect y="10" width="5" height="10" fill="currentColor"/></svg>"##;
        for width in [64, 192] {
            let render = |ink| {
                resvg::tiny_skia::Pixmap::decode_png(&rasterize(svg, ink, width).unwrap()).unwrap()
            };
            let red = render(Ink::Team(0xff0000));
            let green = render(Ink::Team(0x00ff00));
            let shadow = render(Ink::Shadow);
            assert_eq!(red.height(), 2 * width);
            let at = |image: &resvg::tiny_skia::Pixmap, x, y| image.pixel(x, y).unwrap();
            assert_eq!(at(&red, 2, 2), at(&green, 2, 2));
            assert_eq!(at(&red, 2, width + 1).red(), 255);
            assert_eq!(at(&green, 2, width + 1).green(), 255);
            assert_eq!(at(&shadow, 2, 2).alpha(), 56);
            assert_eq!(at(&shadow, width - 1, width + 1).alpha(), 0);
        }
    }
}
