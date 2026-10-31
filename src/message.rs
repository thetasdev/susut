//! User-facing runtime messages, rendered in English or Indonesian.

/// Language of runtime messages (not of `--help`, which is always English).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Lang {
    #[default]
    En,
    Id,
}

pub enum Msg {
    MinQualityTooHigh { min: u8, quality: u8 },
    NoImages,
    OutputNotDir,
    CreateDirFailed(String),
    SubdirsIgnored,
    Collision { a: String, b: String, out: String },
    FileNotFound,
    UnsupportedFormat,
    DecodeFailed,
    SameAsInput,
    AlreadyExists(String),
    WriteFailed(String),
    NotSmaller,
    SavingsBelowMin { saved: String, min: String },
    TargetMissed,
    LargerThanOriginal,
    Skipped,
    DryRun,
    Saved(usize),
    SkippedCount(usize),
    Failed(usize),
    MozjpegFailed,
    EncodeJpegFailed,
    EncodeAvifFailed(String),
    OptimizePngFailed(String),
    TargetFailed,
}

impl Msg {
    pub fn render(&self, lang: Lang) -> String {
        use Lang::{En, Id};
        match (self, lang) {
            (Msg::MinQualityTooHigh { min, quality }, En) => {
                format!("--min-quality ({min}) must not exceed --quality ({quality})")
            }
            (Msg::MinQualityTooHigh { min, quality }, Id) => {
                format!("--min-quality ({min}) tidak boleh lebih besar dari --quality ({quality})")
            }
            (Msg::NoImages, En) => "no images found".into(),
            (Msg::NoImages, Id) => "tidak ada gambar yang ditemukan".into(),
            (Msg::OutputNotDir, En) => {
                "-o is a file but there is more than one input; end it with / for a directory"
                    .into()
            }
            (Msg::OutputNotDir, Id) => {
                "-o berupa file tapi input lebih dari satu; akhiri dengan / untuk folder".into()
            }
            (Msg::CreateDirFailed(p), En) => format!("failed to create directory {p}"),
            (Msg::CreateDirFailed(p), Id) => format!("gagal membuat folder {p}"),
            (Msg::SubdirsIgnored, En) => "ℹ subdirectories ignored; use -r to recurse".into(),
            (Msg::SubdirsIgnored, Id) => {
                "ℹ subfolder diabaikan; pakai -r untuk memproses secara rekursif".into()
            }
            (Msg::Collision { a, b, out }, En) => {
                format!("{a} and {b} would both be written to {out}; use a different -f or -o")
            }
            (Msg::Collision { a, b, out }, Id) => {
                format!("{a} dan {b} sama-sama akan ditulis ke {out}; pakai -f atau -o berbeda")
            }
            (Msg::FileNotFound, En) => "file not found".into(),
            (Msg::FileNotFound, Id) => "file tidak ditemukan".into(),
            (Msg::UnsupportedFormat, En) => "unsupported image format".into(),
            (Msg::UnsupportedFormat, Id) => "format gambar tidak didukung".into(),
            (Msg::DecodeFailed, En) => "failed to decode image".into(),
            (Msg::DecodeFailed, Id) => "gagal decode gambar".into(),
            (Msg::SameAsInput, En) => {
                "output is the same as input; choose another name/directory".into()
            }
            (Msg::SameAsInput, Id) => "output sama dengan input; pilih nama/folder lain".into(),
            (Msg::AlreadyExists(p), En) => format!("{p} already exists (use -y to overwrite)"),
            (Msg::AlreadyExists(p), Id) => format!("{p} sudah ada (pakai -y untuk menimpa)"),
            (Msg::WriteFailed(p), En) => format!("failed to write {p}"),
            (Msg::WriteFailed(p), Id) => format!("gagal menulis {p}"),
            (Msg::NotSmaller, En) => "result is not smaller (use --force)".into(),
            (Msg::NotSmaller, Id) => "hasil tidak lebih kecil (pakai --force)".into(),
            (Msg::SavingsBelowMin { saved, min }, En) => {
                format!("saved {saved} < required {min} (use --force)")
            }
            (Msg::SavingsBelowMin { saved, min }, Id) => {
                format!("hemat {saved} < minimal {min} (pakai --force)")
            }
            (Msg::TargetMissed, En) => "⚠ size target not reached".into(),
            (Msg::TargetMissed, Id) => "⚠ target ukuran tidak tercapai".into(),
            (Msg::LargerThanOriginal, En) => "⚠ larger than original".into(),
            (Msg::LargerThanOriginal, Id) => "⚠ lebih besar dari asli".into(),
            (Msg::Skipped, En) => "skipped".into(),
            (Msg::Skipped, Id) => "dilewati".into(),
            (Msg::DryRun, _) => "(dry-run)".into(),
            (Msg::Saved(n), En) => format!("{n} saved"),
            (Msg::Saved(n), Id) => format!("{n} disimpan"),
            (Msg::SkippedCount(n), En) => format!("{n} skipped"),
            (Msg::SkippedCount(n), Id) => format!("{n} dilewati"),
            (Msg::Failed(n), En) => format!("{n} failed"),
            (Msg::Failed(n), Id) => format!("{n} gagal"),
            (Msg::MozjpegFailed, En) => "mozjpeg failed".into(),
            (Msg::MozjpegFailed, Id) => "mozjpeg gagal".into(),
            (Msg::EncodeJpegFailed, En) => "JPEG encoding failed".into(),
            (Msg::EncodeJpegFailed, Id) => "encode JPEG gagal".into(),
            (Msg::EncodeAvifFailed(e), En) => format!("AVIF encoding failed: {e}"),
            (Msg::EncodeAvifFailed(e), Id) => format!("encode AVIF gagal: {e}"),
            (Msg::OptimizePngFailed(e), En) => format!("PNG optimization failed: {e}"),
            (Msg::OptimizePngFailed(e), Id) => format!("optimasi PNG gagal: {e}"),
            (Msg::TargetFailed, En) => "failed to reach the size target".into(),
            (Msg::TargetFailed, Id) => "gagal mencapai target ukuran".into(),
        }
    }
}
