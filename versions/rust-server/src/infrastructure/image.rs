use anyhow::{Context, Result};
use image::codecs::avif::AvifEncoder;
use image::{ExtendedColorType, ImageEncoder};

pub async fn convert_to_avif(source_bytes: Vec<u8>) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let dyn_img = image::load_from_memory(&source_bytes).context("failed to decode image")?;
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut encoded = Vec::new();
        let encoder = AvifEncoder::new_with_speed_quality(&mut encoded, 6, 75);
        encoder
            .write_image(rgba.as_raw(), w, h, ExtendedColorType::Rgba8)
            .context("failed to encode avif")?;
        Ok(encoded)
    })
    .await
    .context("avif worker join error")?
}
