//! Read-only local package input. Only decoded visual assets survive import.
use anyhow::{Result, ensure};
use image::{DynamicImage, ImageFormat};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    path::Path,
};
pub struct PassPackage {
    pub inventory: crate::pass_artwork::VisualInventory,
    images: BTreeMap<String, Vec<u8>>,
}
impl PassPackage {
    pub fn open(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 8 * 1024 * 1024, "Package exceeds 8 MiB");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        ensure!(archive.len() <= 512, "Too many entries");
        let mut names = Vec::new();
        let mut total = 0u64;
        for index in 0..archive.len() {
            let file = archive.by_index(index)?;
            ensure!(
                !file.encrypted() && !file.is_symlink() && !file.is_dir(),
                "Unsupported entry"
            );
            ensure!(file.size() <= 4 * 1024 * 1024, "Entry too large");
            total += file.size();
            ensure!(total <= 32 * 1024 * 1024, "Expanded package too large");
            names.push(file.name()?.into_owned());
        }
        let mut metadata = Vec::new();
        archive
            .by_name("pass.json")?
            .take(256 * 1024 + 1)
            .read_to_end(&mut metadata)?;
        let inventory = crate::pass_artwork::inspect(
            &metadata,
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .map_err(|_| anyhow::anyhow!("Unsupported or invalid visual inventory"))?;
        let mut images = BTreeMap::new();
        let mut pixels = 0u64;
        for name in &inventory.images {
            let mut data = Vec::new();
            archive
                .by_name(name)?
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut data)?;
            ensure!(data.len() <= 4 * 1024 * 1024, "Image too large");
            let image = decode(&data)?;
            pixels += u64::from(image.width()) * u64::from(image.height());
            ensure!(pixels <= 16 * 1024 * 1024, "Too many image pixels");
            // Re-encode pixels: discard embedded metadata and ancillary PNG chunks.
            let mut clean = Cursor::new(Vec::new());
            image.write_to(&mut clean, ImageFormat::Png)?;
            images.insert(name.clone(), clean.into_inner());
        }
        Ok(Self { inventory, images })
    }
    pub fn image(&self, name: &str) -> Result<DynamicImage> {
        decode(
            self.images
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("Not a visual resource"))?,
        )
    }
    pub fn replacement(&self, name: &str, image: &DynamicImage) -> Result<Vec<u8>> {
        let original = self.image(name)?;
        ensure!(
            image.width() > 0
                && image.height() > 0
                && image.width() <= 8192
                && image.height() <= 8192,
            "Invalid replacement dimensions"
        );
        let (width, height) = crop_dimensions(
            image.width(),
            image.height(),
            original.width(),
            original.height(),
        );
        let cropped = image.crop_imm(
            (image.width() - width) / 2,
            (image.height() - height) / 2,
            width,
            height,
        );
        let result = cropped.resize_exact(
            original.width(),
            original.height(),
            image::imageops::FilterType::Lanczos3,
        );
        let mut bytes = Cursor::new(Vec::new());
        result.write_to(&mut bytes, ImageFormat::Png)?;
        Ok(bytes.into_inner())
    }
}

fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader.decode()?)
}

fn crop_dimensions(width: u32, height: u32, target_width: u32, target_height: u32) -> (u32, u32) {
    if u64::from(width) * u64::from(target_height) > u64::from(height) * u64::from(target_width) {
        (
            ((u64::from(height) * u64::from(target_width) / u64::from(target_height)) as u32)
                .max(1),
            height,
        )
    } else {
        (
            width,
            ((u64::from(width) * u64::from(target_height) / u64::from(target_width)) as u32).max(1),
        )
    }
}

#[cfg(test)]
mod crop_tests {
    #[test]
    fn extreme_aspect_ratios_crop_before_resizing() {
        assert_eq!(super::crop_dimensions(8192, 1, 1, 4096), (1, 1));
        assert_eq!(super::crop_dimensions(1, 8192, 4096, 1), (1, 1));
        assert_eq!(super::crop_dimensions(80, 80, 30, 10), (80, 26));
    }
}
