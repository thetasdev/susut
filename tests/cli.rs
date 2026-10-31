//! End-to-end tests: run the real binary against small, generated fixtures.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use image::{ImageEncoder, RgbImage, codecs::jpeg::JpegEncoder};
use tempfile::TempDir;

fn susut() -> Command {
    let mut cmd = Command::cargo_bin("susut").unwrap();
    cmd.env_remove("SUSUT_LANG");
    cmd
}

/// Deterministic pseudo-random noise: large when encoded, no external assets.
fn noise(w: u32, h: u32) -> RgbImage {
    let mut state = 0x2545_f491_u32;
    RgbImage::from_fn(w, h, |_, _| {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        };
        image::Rgb([next(), next(), next()])
    })
}

fn jpeg_bytes(img: &RgbImage, quality: u8) -> Vec<u8> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

fn write_jpeg(path: &Path, img: &RgbImage, quality: u8) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, jpeg_bytes(img, quality)).unwrap();
}

/// Insert an EXIF APP1 segment (orientation + a GPS IFD) right after SOI.
fn with_exif(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    let mut tiff: Vec<u8> = b"II*\0\x08\0\0\0".to_vec();
    // IFD0: orientation + pointer to the GPS IFD at offset 38.
    tiff.extend(2u16.to_le_bytes());
    tiff.extend([0x12, 0x01, 3, 0, 1, 0, 0, 0]);
    tiff.extend(orientation.to_le_bytes());
    tiff.extend([0, 0]);
    tiff.extend([0x25, 0x88, 4, 0, 1, 0, 0, 0]);
    tiff.extend(38u32.to_le_bytes());
    tiff.extend(0u32.to_le_bytes());
    // GPS IFD: GPSLatitudeRef = "N".
    tiff.extend(1u16.to_le_bytes());
    tiff.extend([0x01, 0x00, 2, 0, 2, 0, 0, 0, b'N', 0, 0, 0]);
    tiff.extend(0u32.to_le_bytes());

    let mut payload = b"Exif\0\0".to_vec();
    payload.extend(tiff);
    let len = (payload.len() + 2) as u16;

    let mut out = jpeg[..2].to_vec();
    out.extend([0xFF, 0xE1]);
    out.extend(len.to_be_bytes());
    out.extend(payload);
    out.extend(&jpeg[2..]);
    out
}

fn dims(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).unwrap()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn reaches_size_target() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("photo.jpg");
    write_jpeg(&input, &noise(300, 300), 95);
    let target = 20 * 1024;
    assert!(fs::metadata(&input).unwrap().len() > target);

    susut().arg(&input).args(["-s", "20KB"]).assert().success();

    let out = dir.path().join("photo.min.jpg");
    assert!(fs::metadata(out).unwrap().len() <= target);
}

#[test]
fn skips_when_result_is_not_smaller() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("small.jpg");
    write_jpeg(&input, &noise(64, 64), 30);

    susut()
        .arg(&input)
        .args(["-q", "100"])
        .assert()
        .success()
        .stdout(predicates::str::contains("skipped"));

    assert!(!dir.path().join("small.min.jpg").exists());
}

#[test]
fn force_writes_even_if_bigger() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("small.jpg");
    write_jpeg(&input, &noise(64, 64), 30);

    susut()
        .arg(&input)
        .args(["-q", "100", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("larger than original"));

    assert!(dir.path().join("small.min.jpg").exists());
}

#[test]
fn dry_run_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("photo.jpg");
    write_jpeg(&input, &noise(200, 200), 95);

    susut()
        .arg(&input)
        .args(["-n", "-o"])
        .arg(dir.path().join("out/"))
        .assert()
        .success()
        .stdout(predicates::str::contains("(dry-run)"));

    assert!(!dir.path().join("out").exists());
    assert!(!dir.path().join("photo.min.jpg").exists());
}

#[test]
fn recursive_keeps_directory_structure() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    write_jpeg(&src.join("a.jpg"), &noise(120, 120), 95);
    write_jpeg(&src.join("sub/deep/b.jpg"), &noise(120, 120), 95);
    let out = dir.path().join("out");

    susut()
        .args(["-r"])
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert!(out.join("a.jpg").exists());
    assert!(out.join("sub/deep/b.jpg").exists());
}

#[test]
fn colliding_output_names_are_rejected() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("photo.jpg");
    let b = dir.path().join("photo.png");
    write_jpeg(&a, &noise(64, 64), 95);
    noise(64, 64).save(&b).unwrap();

    susut()
        .arg(&a)
        .arg(&b)
        .args(["-f", "jpeg"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("would both be written"));

    assert!(!dir.path().join("photo.min.jpg").exists());
}

#[test]
fn exif_orientation_is_applied() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("rotated.jpg");
    let plain = jpeg_bytes(&noise(80, 40), 95);
    fs::write(&input, with_exif(&plain, 6)).unwrap(); // 6 = rotate 90° CW
    assert_eq!(dims(&input), (80, 40)); // raw pixels are still landscape

    susut().arg(&input).args(["-q", "50"]).assert().success();

    assert_eq!(dims(&dir.path().join("rotated.min.jpg")), (40, 80));
}

#[test]
fn gps_metadata_is_stripped() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("geo.jpg");
    fs::write(&input, with_exif(&jpeg_bytes(&noise(100, 100), 95), 1)).unwrap();
    assert!(contains(&fs::read(&input).unwrap(), b"Exif\0\0"));

    susut().arg(&input).args(["-q", "50"]).assert().success();

    let out = fs::read(dir.path().join("geo.min.jpg")).unwrap();
    assert!(!contains(&out, b"Exif\0\0"));
}

#[test]
fn exit_code_is_1_when_any_file_fails() {
    let dir = TempDir::new().unwrap();
    let good = dir.path().join("good.jpg");
    let bad = dir.path().join("bad.jpg");
    write_jpeg(&good, &noise(100, 100), 95);
    fs::write(&bad, b"this is not an image").unwrap();

    susut()
        .arg(&good)
        .arg(&bad)
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("bad.jpg"));

    // the good file is still processed
    assert!(dir.path().join("good.min.jpg").exists());
}

#[test]
fn existing_output_needs_overwrite_flag() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("photo.jpg");
    write_jpeg(&input, &noise(100, 100), 95);

    susut().arg(&input).assert().success();
    susut()
        .arg(&input)
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("already exists"));
    susut().arg(&input).arg("-y").assert().success();
}

#[test]
fn messages_are_english_by_default_and_indonesian_on_request() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nothing/");
    fs::create_dir_all(&missing).unwrap();

    susut()
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicates::str::contains("no images found"));
    susut()
        .arg(&missing)
        .args(["--lang", "id"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("tidak ada gambar"));
    susut()
        .arg(&missing)
        .env("SUSUT_LANG", "id")
        .assert()
        .failure()
        .stderr(predicates::str::contains("tidak ada gambar"));
}

fn read_magic(path: &Path, range: std::ops::Range<usize>) -> Vec<u8> {
    fs::read(path).unwrap()[range].to_vec()
}

#[test]
fn converts_to_every_output_format() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("photo.jpg");
    write_jpeg(&input, &noise(96, 64), 95);

    for fmt in ["jpeg", "png", "webp", "avif"] {
        let out = dir.path().join(format!("out-{fmt}"));
        susut()
            .arg(&input)
            .args(["-f", fmt, "-q", "60", "--force", "-o"])
            .arg(format!("{}/", out.display()))
            .assert()
            .success();

        let ext = if fmt == "jpeg" { "jpg" } else { fmt };
        let file = out.join(format!("photo.{ext}"));
        match fmt {
            "jpeg" => assert_eq!(read_magic(&file, 0..3), [0xFF, 0xD8, 0xFF]),
            "png" => assert_eq!(read_magic(&file, 1..4), *b"PNG"),
            "webp" => assert_eq!(read_magic(&file, 8..12), *b"WEBP"),
            "avif" => assert_eq!(read_magic(&file, 4..12), *b"ftypavif"),
            _ => unreachable!(),
        }
        if fmt != "avif" {
            // the image crate can decode these; AVIF decoding is not enabled in the test build
            assert_eq!(dims(&file), (96, 64), "{fmt}");
        }
    }
}

#[test]
fn png_output_is_lossless() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("pixels.png");
    let original = noise(48, 48);
    original.save(&input).unwrap();

    susut().arg(&input).arg("--force").assert().success();

    let out = image::open(dir.path().join("pixels.min.png"))
        .unwrap()
        .to_rgb8();
    assert_eq!(out, original);
}

#[test]
fn transparent_png_is_flattened_onto_white_for_jpeg() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("clear.png");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 0]))
        .save(&input)
        .unwrap();

    susut()
        .arg(&input)
        .args(["-f", "jpeg", "--force"])
        .assert()
        .success();

    let out = image::open(dir.path().join("clear.min.jpg"))
        .unwrap()
        .to_rgb8();
    assert!(out.pixels().all(|p| p.0.iter().all(|&c| c >= 250)));
}

#[test]
fn size_target_works_for_webp_and_png() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("noisy.png");
    noise(200, 200).save(&input).unwrap();

    for (fmt, ext) in [("webp", "webp"), ("png", "png")] {
        let out = dir.path().join(format!("out-{fmt}"));
        susut()
            .arg(&input)
            .args(["-f", fmt, "-s", "20KB", "-o"])
            .arg(format!("{}/", out.display()))
            .assert()
            .success();
        let size = fs::metadata(out.join(format!("noisy.{ext}")))
            .unwrap()
            .len();
        assert!(size <= 20 * 1024, "{fmt}: {size} bytes");
    }
}
