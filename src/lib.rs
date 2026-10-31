//! susut — compress, resize and convert images.
//!
//! The library holds everything except argument parsing and terminal output,
//! so it can later become a shared `core` crate for the CLI and WASM.

mod message;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use fast_image_resize::{
    FilterType, MulDiv, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image,
};
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageReader};

pub use message::{Lang, Msg};

pub const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "avif", "gif", "bmp", "tif", "tiff",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Format {
    #[cfg_attr(feature = "cli", value(alias = "jpg"))]
    Jpeg,
    Png,
    Webp,
    Avif,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Webp => "webp",
            Format::Avif => "avif",
        }
    }

    pub fn lossy(self) -> bool {
        self != Format::Png
    }

    pub fn from_ext(p: &Path) -> Option<Format> {
        match ext_lower(p)?.as_str() {
            "jpg" | "jpeg" => Some(Format::Jpeg),
            "png" => Some(Format::Png),
            "webp" => Some(Format::Webp),
            "avif" => Some(Format::Avif),
            _ => None,
        }
    }
}

/// An absolute size (bytes) or a size relative to the original file (percent).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Amount {
    Bytes(u64),
    Percent(f64),
}

impl Amount {
    pub fn of(self, original: u64) -> u64 {
        match self {
            Amount::Bytes(b) => b,
            Amount::Percent(p) => ((original as f64) * p / 100.0).round() as u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Output format; `None` keeps the input format.
    pub format: Option<Format>,
    pub quality: u8,
    pub min_quality: u8,
    pub max_size: Option<Amount>,
    pub resize: Option<f64>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub long_edge: Option<u32>,
    pub short_edge: Option<u32>,
    pub min_savings: Option<Amount>,
    pub force: bool,
    pub dry_run: bool,
    pub overwrite: bool,
    pub lang: Lang,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            format: None,
            quality: 80,
            min_quality: 40,
            max_size: None,
            resize: None,
            max_width: None,
            max_height: None,
            long_edge: None,
            short_edge: None,
            min_savings: None,
            force: false,
            dry_run: false,
            overwrite: false,
            lang: Lang::default(),
        }
    }
}

impl Options {
    pub fn validate(&self) -> Result<()> {
        if self.min_quality > self.quality {
            bail!(
                "{}",
                Msg::MinQualityTooHigh {
                    min: self.min_quality,
                    quality: self.quality
                }
                .render(self.lang)
            );
        }
        Ok(())
    }
}

/// Where the output of one input goes.
#[derive(Clone, Debug)]
pub enum Destination {
    /// Next to the input, as `<stem>.min.<ext>`.
    Beside,
    /// An explicit output file.
    File(PathBuf),
    /// Inside `dir`, under the relative subdirectory `sub` (keeps `-r` structure).
    Dir { dir: PathBuf, sub: PathBuf },
}

pub enum Status {
    Written,
    Skipped(String),
}

pub struct Outcome {
    pub output: PathBuf,
    pub before: u64,
    pub after: u64,
    pub quality: Option<u8>,
    pub dims: (u32, u32),
    pub hit_target: bool,
    pub status: Status,
}

pub struct Encoded {
    pub bytes: Vec<u8>,
    pub quality: Option<u8>,
    pub img: DynamicImage,
    pub hit_target: bool,
}

pub struct Decoded {
    pub img: DynamicImage,
    pub source: Option<image::ImageFormat>,
}

/// Compress one file according to `opts`; writes the output unless `dry_run`.
pub fn process(opts: &Options, input: &Path, dest: &Destination) -> Result<Outcome> {
    let lang = opts.lang;
    let before = fs::metadata(input)
        .context(Msg::FileNotFound.render(lang))?
        .len();

    let Decoded { mut img, source } = decode(input, lang)?;

    let format = opts.format.unwrap_or_else(|| match source {
        Some(image::ImageFormat::Jpeg) => Format::Jpeg,
        Some(image::ImageFormat::WebP) => Format::Webp,
        Some(image::ImageFormat::Avif) => Format::Avif,
        Some(image::ImageFormat::Png) => Format::Png,
        _ if img.color().has_alpha() => Format::Png,
        _ => Format::Jpeg,
    });

    let output = output_path(input, dest, format);
    if same_file(input, &output) {
        bail!("{}", Msg::SameAsInput.render(lang));
    }
    if output.exists() && !opts.overwrite && !opts.dry_run {
        bail!(
            "{}",
            Msg::AlreadyExists(output.display().to_string()).render(lang)
        );
    }

    img = fit(img, opts)?;

    let enc = match opts.max_size {
        None => {
            let quality = format.lossy().then_some(opts.quality);
            Encoded {
                bytes: encode(&img, format, opts.quality, lang)?,
                quality,
                img,
                hit_target: true,
            }
        }
        Some(amount) => {
            let target = amount.of(before).max(1);
            encode_to_target(img, format, opts, target)?
        }
    };
    let after = enc.bytes.len() as u64;

    let status = match skip_reason(opts, before, after) {
        Some(why) => Status::Skipped(why),
        None => {
            if !opts.dry_run {
                if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
                    fs::create_dir_all(parent).with_context(|| {
                        Msg::CreateDirFailed(parent.display().to_string()).render(lang)
                    })?;
                }
                fs::write(&output, &enc.bytes)
                    .with_context(|| Msg::WriteFailed(output.display().to_string()).render(lang))?;
            }
            Status::Written
        }
    };

    Ok(Outcome {
        output,
        before,
        after,
        quality: enc.quality,
        dims: enc.img.dimensions(),
        hit_target: enc.hit_target,
        status,
    })
}

/// Decode and apply the EXIF orientation. Other metadata is dropped.
pub fn decode(path: &Path, lang: Lang) -> Result<Decoded> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let source = reader.format();
    let mut decoder = reader
        .into_decoder()
        .context(Msg::UnsupportedFormat.render(lang))?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder).context(Msg::DecodeFailed.render(lang))?;
    img.apply_orientation(orientation);
    Ok(Decoded { img, source })
}

pub fn skip_reason(opts: &Options, before: u64, after: u64) -> Option<String> {
    if opts.force {
        return None;
    }
    let saved = before.saturating_sub(after);
    match opts.min_savings {
        None if after >= before => Some(Msg::NotSmaller.render(opts.lang)),
        Some(min) if saved < min.of(before) => Some(
            Msg::SavingsBelowMin {
                saved: human(saved),
                min: match min {
                    Amount::Percent(p) => format!("{p}%"),
                    Amount::Bytes(b) => human(b),
                },
            }
            .render(opts.lang),
        ),
        _ => None,
    }
}

pub fn output_path(input: &Path, dest: &Destination, format: Format) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    match dest {
        Destination::Dir { dir, sub } => dir.join(sub).join(format!("{stem}.{}", format.ext())),
        Destination::File(file) => file.clone(),
        Destination::Beside => input.with_file_name(format!("{stem}.min.{}", format.ext())),
    }
}

pub fn ext_lower(p: &Path) -> Option<String> {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
}

pub fn is_image(p: &Path) -> bool {
    ext_lower(p).is_some_and(|e| IMAGE_EXTS.contains(&e.as_str()))
}

/// Output of an earlier run (`*.min.jpg`) is not processed again.
pub fn is_min_output(p: &Path) -> bool {
    p.file_stem()
        .is_some_and(|s| s.to_string_lossy().ends_with(".min"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Final scale = the strictest of all resize options; never enlarges.
pub fn scale_factor(w: u32, h: u32, opts: &Options) -> f64 {
    let (w, h) = (w as f64, h as f64);
    let candidates = [
        opts.resize.map(|p| p / 100.0),
        opts.max_width.map(|m| m as f64 / w),
        opts.max_height.map(|m| m as f64 / h),
        opts.long_edge.map(|m| m as f64 / w.max(h)),
        opts.short_edge.map(|m| m as f64 / w.min(h)),
    ];
    candidates.into_iter().flatten().fold(1.0, f64::min)
}

pub fn fit(img: DynamicImage, opts: &Options) -> Result<DynamicImage> {
    let (w, h) = img.dimensions();
    let s = scale_factor(w, h, opts);
    if s >= 1.0 {
        return Ok(img);
    }
    let nw = ((w as f64 * s).round() as u32).max(1);
    let nh = ((h as f64 * s).round() as u32).max(1);
    resize(&img, nw, nh)
}

/// Lanczos3 resize with SIMD (fast_image_resize). The result is 8-bit, like every encoder input.
/// Alpha is premultiplied around the resize so transparent edges do not bleed.
pub fn resize(img: &DynamicImage, w: u32, h: u32) -> Result<DynamicImage> {
    let (sw, sh) = img.dimensions();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    let mut resizer = Resizer::new();
    if img.color().has_alpha() {
        let mut src = Image::from_vec_u8(sw, sh, img.to_rgba8().into_raw(), PixelType::U8x4)?;
        let mut dst = Image::new(w, h, PixelType::U8x4);
        let mul_div = MulDiv::default();
        mul_div.multiply_alpha_inplace(&mut src)?;
        resizer.resize(&src, &mut dst, &options)?;
        mul_div.divide_alpha_inplace(&mut dst)?;
        let buf = image::RgbaImage::from_raw(w, h, dst.into_vec())
            .ok_or_else(|| anyhow!("resize buffer size mismatch"))?;
        Ok(DynamicImage::ImageRgba8(buf))
    } else {
        let src = Image::from_vec_u8(sw, sh, img.to_rgb8().into_raw(), PixelType::U8x3)?;
        let mut dst = Image::new(w, h, PixelType::U8x3);
        resizer.resize(&src, &mut dst, &options)?;
        let buf = image::RgbImage::from_raw(w, h, dst.into_vec())
            .ok_or_else(|| anyhow!("resize buffer size mismatch"))?;
        Ok(DynamicImage::ImageRgb8(buf))
    }
}

/// Find the highest quality that fits `target`; if even `min_quality` does not fit, downscale.
pub fn encode_to_target(
    mut img: DynamicImage,
    format: Format,
    opts: &Options,
    target: u64,
) -> Result<Encoded> {
    const MAX_DOWNSCALES: usize = 8;
    let (max_q, min_q, lang) = (opts.quality, opts.min_quality, opts.lang);
    let mut last: Option<(Vec<u8>, Option<u8>)> = None;

    for _ in 0..=MAX_DOWNSCALES {
        if format.lossy() {
            let best = encode(&img, format, max_q, lang)?;
            if best.len() as u64 <= target {
                return Ok(Encoded {
                    bytes: best,
                    quality: Some(max_q),
                    img,
                    hit_target: true,
                });
            }
            last = Some((best, Some(max_q)));
            // Binary search quality in [min_q, max_q).
            let (mut lo, mut hi) = (min_q, max_q - 1);
            let mut found: Option<(Vec<u8>, u8)> = None;
            while lo <= hi {
                let mid = lo + (hi - lo) / 2;
                let data = encode(&img, format, mid, lang)?;
                if data.len() as u64 <= target {
                    found = Some((data, mid));
                    lo = mid + 1;
                } else {
                    last = Some((data, Some(mid)));
                    hi = mid - 1;
                }
            }
            if let Some((data, q)) = found {
                return Ok(Encoded {
                    bytes: data,
                    quality: Some(q),
                    img,
                    hit_target: true,
                });
            }
        } else {
            let data = encode(&img, format, max_q, lang)?;
            if data.len() as u64 <= target {
                return Ok(Encoded {
                    bytes: data,
                    quality: None,
                    img,
                    hit_target: true,
                });
            }
            last = Some((data, None));
        }

        // Still too big: shrink the dimensions in proportion to the size ratio.
        let current = last
            .as_ref()
            .map(|(d, _)| d.len() as f64)
            .unwrap_or(target as f64 * 2.0);
        let scale = ((target as f64 / current).sqrt() * 0.95).clamp(0.5, 0.9);
        let (w, h) = img.dimensions();
        let (nw, nh) = (((w as f64) * scale) as u32, ((h as f64) * scale) as u32);
        if nw < 16 || nh < 16 {
            break;
        }
        img = resize(&img, nw, nh)?;
    }

    let (bytes, quality) = last.ok_or_else(|| anyhow!(Msg::TargetFailed.render(lang)))?;
    Ok(Encoded {
        bytes,
        quality,
        img,
        hit_target: false,
    })
}

pub fn encode(img: &DynamicImage, format: Format, quality: u8, lang: Lang) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    match format {
        Format::Jpeg => {
            let rgb = flatten_on_white(img);
            std::panic::catch_unwind(|| -> std::io::Result<Vec<u8>> {
                let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
                c.set_size(w as usize, h as usize);
                c.set_quality(quality as f32);
                c.set_progressive_mode();
                c.set_optimize_coding(true);
                let mut c = c.start_compress(Vec::new())?;
                c.write_scanlines(rgb.as_raw())?;
                c.finish()
            })
            .map_err(|_| anyhow!(Msg::MozjpegFailed.render(lang)))?
            .context(Msg::EncodeJpegFailed.render(lang))
        }
        Format::Webp => {
            let rgba = img.to_rgba8();
            let mem = webp::Encoder::from_rgba(rgba.as_raw(), w, h).encode(quality as f32);
            Ok(mem.to_vec())
        }
        Format::Avif => {
            let rgba = img.to_rgba8();
            let pixels: &[rgb::RGBA8] = rgb::FromSlice::as_rgba(rgba.as_raw().as_slice());
            let res = ravif::Encoder::new()
                .with_quality(quality as f32)
                .with_alpha_quality(quality as f32)
                .with_speed(6)
                .encode_rgba(ravif::Img::new(pixels, w as usize, h as usize))
                .map_err(|e| anyhow!(Msg::EncodeAvifFailed(e.to_string()).render(lang)))?;
            Ok(res.avif_file)
        }
        Format::Png => {
            let mut raw = Vec::new();
            let img = if img.color().has_alpha() {
                DynamicImage::ImageRgba8(img.to_rgba8())
            } else {
                DynamicImage::ImageRgb8(img.to_rgb8())
            };
            img.write_to(&mut std::io::Cursor::new(&mut raw), image::ImageFormat::Png)?;
            let mut opts = oxipng::Options::from_preset(2);
            opts.strip = oxipng::StripChunks::Safe;
            oxipng::optimize_from_memory(&raw, &opts)
                .map_err(|e| anyhow!(Msg::OptimizePngFailed(e.to_string()).render(lang)))
        }
    }
}

/// JPEG has no alpha: composite onto white.
fn flatten_on_white(img: &DynamicImage) -> image::RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let mut out = image::RgbImage::new(rgba.width(), rgba.height());
    for (o, p) in out.pixels_mut().zip(rgba.pixels()) {
        let a = p[3] as u32;
        for i in 0..3 {
            o[i] = ((p[i] as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        }
    }
    out
}

pub fn parse_percent(s: &str) -> Result<f64, String> {
    let n: f64 = s
        .trim()
        .trim_end_matches('%')
        .parse()
        .map_err(|_| format!("invalid percentage: {s} (example: 50%)"))?;
    if !(n > 0.0 && n <= 100.0) {
        return Err("percentage must be above 0 and at most 100".into());
    }
    Ok(n)
}

pub fn parse_amount(s: &str) -> Result<Amount, String> {
    let t = s.trim().to_ascii_lowercase().replace(' ', "");
    if t.ends_with('%') {
        return parse_percent(&t).map(Amount::Percent);
    }
    let split = t.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(t.len());
    let (num, unit) = t.split_at(split);
    let n: f64 = num.parse().map_err(|_| format!("invalid size: {s}"))?;
    let mult = match unit {
        "" | "b" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(format!("unknown unit: {unit} (use B, KB, MB, GB or %)"));
        }
    };
    let bytes = (n * mult) as u64;
    if bytes == 0 {
        return Err("size must be greater than 0".into());
    }
    Ok(Amount::Bytes(bytes))
}

pub fn human(b: u64) -> String {
    let b = b as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.2} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{b} B")
    }
}

pub fn pct(before: u64, after: u64) -> String {
    if before == 0 {
        return "-".into();
    }
    let d = (after as f64 - before as f64) / before as f64 * 100.0;
    format!("{d:+.0}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(f: impl FnOnce(&mut Options)) -> Options {
        let mut o = Options::default();
        f(&mut o);
        o
    }

    #[test]
    fn amounts() {
        assert_eq!(parse_amount("200KB").unwrap(), Amount::Bytes(204_800));
        assert_eq!(parse_amount("1.5MB").unwrap(), Amount::Bytes(1_572_864));
        assert_eq!(parse_amount("5000").unwrap(), Amount::Bytes(5000));
        assert_eq!(parse_amount("50%").unwrap(), Amount::Percent(50.0));
        assert_eq!(Amount::Percent(25.0).of(1000), 250);
        assert!(parse_amount("2TB").is_err());
        assert!(parse_amount("0").is_err());
        assert!(parse_amount("150%").is_err());
    }

    #[test]
    fn scaling() {
        let none = Options::default();
        assert_eq!(scale_factor(4000, 3000, &none), 1.0);
        assert_eq!(
            scale_factor(4000, 3000, &opts(|o| o.resize = Some(50.0))),
            0.5
        );
        let long = opts(|o| o.long_edge = Some(2000));
        assert_eq!(scale_factor(4000, 3000, &long), 0.5);
        assert_eq!(scale_factor(3000, 4000, &long), 0.5);
        assert_eq!(
            scale_factor(4000, 3000, &opts(|o| o.short_edge = Some(1500))),
            0.5
        );
        // the strictest limit wins
        let both = opts(|o| {
            o.resize = Some(80.0);
            o.max_width = Some(1000);
        });
        assert_eq!(scale_factor(4000, 3000, &both), 0.25);
        // never enlarges
        assert_eq!(scale_factor(800, 600, &long), 1.0);
    }

    #[test]
    fn skipping() {
        let none = Options::default();
        assert!(skip_reason(&none, 100, 100).is_some());
        assert!(skip_reason(&none, 100, 99).is_none());
        assert!(skip_reason(&opts(|o| o.force = true), 100, 200).is_none());
        let pct10 = opts(|o| o.min_savings = Some(Amount::Percent(10.0)));
        assert!(skip_reason(&pct10, 1000, 950).is_some());
        assert!(skip_reason(&pct10, 1000, 900).is_none());
        let kb1 = opts(|o| o.min_savings = Some(Amount::Bytes(1024)));
        assert!(skip_reason(&kb1, 5000, 4500).is_some());
    }
}
