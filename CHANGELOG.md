# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-31

First public release.

### Added
- Compress and convert JPEG (mozjpeg), PNG (oxipng), WebP and AVIF (ravif).
- Target file size with `-s` (KB/MB/%): binary search on quality, then downscale.
- Resize with `--resize`, `-W`/`-H` and `-L`/`--short-edge`.
- Recursive mode `-r`, `--dry-run`, skip-if-bigger, `--min-savings`.
- Metadata stripping (EXIF, including GPS), with EXIF orientation applied.
- Runtime messages in English (default) or Indonesian via `--lang id` or `SUSUT_LANG=id`.
- Prebuilt binaries for Linux (musl and glibc, x86_64/aarch64), macOS (x86_64/arm64) and Windows (x86_64/arm64).
- Resizing uses the SIMD Lanczos3 filter of `fast_image_resize` (about 1.7 times faster on 45 MP photos; binary grows by about 2 MB).
- Windows ARM64 binaries are built without rav1e's SIMD assembly (MSVC cannot assemble it), so AVIF encoding is slower there.
- Library crate (`susut`) with the compression logic; the CLI is a thin front end.
