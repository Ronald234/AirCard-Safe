#[path = "../research/recovery-store/src/pass_artwork.rs"]
mod pass_artwork;
#[path = "../src/pass_package.rs"]
mod pass_package;
use image::{DynamicImage, ImageFormat};
use pass_package::PassPackage;
use std::io::{Cursor, Write};

fn png() -> Vec<u8> {
    let mut b = Cursor::new(Vec::new());
    DynamicImage::new_rgba8(30, 10)
        .write_to(&mut b, ImageFormat::Png)
        .unwrap();
    b.into_inner()
}
fn package(extra: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let image = png();
    for (name, data) in [
        (
            "pass.json",
            br#"{"storeCard":{},"nfc":{"message":"SECRET"},"serialNumber":"SECRET"}"#.as_slice(),
        ),
        ("strip@2x.png", image.as_slice()),
        ("signature", b"SECRET".as_slice()),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn opens_only_visual_resources_without_changing_source() {
    let bytes = package(&[]);
    let original = bytes.clone();
    let pass = PassPackage::from_bytes(&bytes).unwrap();
    assert!(pass.inventory.nfc_declared);
    assert_eq!(pass.inventory.images, ["strip@2x.png"]);
    assert_eq!(
        (
            pass.image("strip@2x.png").unwrap().width(),
            pass.image("strip@2x.png").unwrap().height()
        ),
        (30, 10)
    );
    assert!(pass.image("pass.json").is_err());
    assert!(pass.image("signature").is_err());
    assert_eq!(bytes, original);
}
#[test]
fn exported_replacement_is_png_with_original_dimensions_and_no_metadata() {
    let pass = PassPackage::from_bytes(&package(&[])).unwrap();
    let bytes = pass
        .replacement("strip@2x.png", &DynamicImage::new_rgb8(80, 80))
        .unwrap();
    let img = image::load_from_memory_with_format(&bytes, ImageFormat::Png).unwrap();
    assert_eq!((img.width(), img.height()), (30, 10));
    assert!(!bytes.windows(6).any(|w| w == b"SECRET"));
    assert!(pass.replacement("signature", &img).is_err());
}
#[test]
fn rejects_unsafe_ambiguous_and_excessive_archives() {
    for name in [
        "../logo.png",
        "a/logo.png",
        "PASS.JSON",
        "logo.png ",
        "x:logo.png",
    ] {
        assert!(
            PassPackage::from_bytes(&package(&[(name, b"x")])).is_err(),
            "{name}"
        );
    }
    assert!(PassPackage::from_bytes(b"not a zip").is_err());
    assert!(PassPackage::from_bytes(&package(&[("logo.png", b"not a png")])).is_err());
    assert!(PassPackage::from_bytes(&vec![0; 8 * 1024 * 1024 + 1]).is_err());
    assert!(
        PassPackage::from_bytes(&package(&[("unused", &vec![0; 4 * 1024 * 1024 + 1])])).is_err()
    );
}

#[test]
fn opening_disk_package_preserves_bytes_and_replacement_pixels_are_used() {
    let bytes = package(&[]);
    let path = std::env::temp_dir().join(format!(
        "aircard-pass-test-{}-{}.pkpass",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(&bytes).unwrap();
    drop(file);
    let pass = PassPackage::open(&path).unwrap();
    let red = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        40,
        40,
        image::Rgba([255, 0, 0, 255]),
    ));
    let output = pass.replacement("strip@2x.png", &red).unwrap();
    let image = image::load_from_memory(&output).unwrap().to_rgba8();
    assert!(image.pixels().all(|p| p.0 == [255, 0, 0, 255]));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn refuses_image_dimensions_above_decode_budget() {
    let mut huge = Cursor::new(Vec::new());
    DynamicImage::new_rgb8(4097, 1)
        .write_to(&mut huge, ImageFormat::Png)
        .unwrap();
    assert!(PassPackage::from_bytes(&package(&[("logo.png", &huge.into_inner())])).is_err());
}

#[test]
fn extreme_aspect_replacement_stays_bounded_and_has_target_size() {
    let mut narrow = Cursor::new(Vec::new());
    DynamicImage::new_rgb8(1, 4096)
        .write_to(&mut narrow, ImageFormat::Png)
        .unwrap();
    let pass = PassPackage::from_bytes(&package(&[("logo.png", &narrow.into_inner())])).unwrap();
    let result = pass
        .replacement("logo.png", &DynamicImage::new_rgb8(8192, 1))
        .unwrap();
    let image = image::load_from_memory(&result).unwrap();
    assert_eq!((image.width(), image.height()), (1, 4096));
}
