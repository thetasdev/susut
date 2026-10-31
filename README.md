# susut

**Shrink images from the command line: compress, resize and convert JPEG, PNG, WebP and AVIF, optionally to a target file size.**

[English](README.md) · [Bahasa Indonesia](README.id.md)

*"Susut" is Indonesian for "shrink".*

```console
$ susut photo.jpg
✓ photo.jpg → photo.min.jpg  492 KB → 102 KB (-79%) 3000×2000 q=80

$ susut photo.jpg -s 5KB
✓ photo.jpg → photo.min.jpg  492 KB → 5 KB (-99%) 394×262 q=43

$ susut -r tree/ -f webp -L 1600 -o out/
✓ tree/a/b/z.jpg → out/a/b/z.webp  492 KB → 26 KB (-95%) 1600×1067 q=80
✓ tree/a/y.jpg → out/a/y.webp  492 KB → 26 KB (-95%) 1600×1067 q=80
✓ tree/x.jpg → out/x.webp  492 KB → 26 KB (-95%) 1600×1067 q=80

3 saved · 1.44 MB → 77 KB (-95%) · 0.3s
```

> The demo image is a synthetic, smooth 3000×2000 picture, so the percentages are
> optimistic. Real photos usually shrink less. Run `susut -n` (dry run) on your own
> files to see what to expect.

- **Encoders:** mozjpeg (JPEG), oxipng (PNG), libwebp (WebP), rav1e via ravif (AVIF).
- **Size target:** `-s 200KB` finds the best quality that fits, and only shrinks the image if it must.
- **Safe by default:** never overwrites, never writes a result that is bigger than the original, never touches the input.
- **Private by default:** all metadata (EXIF, GPS, …) is dropped from the output.
- **Parallel:** processes many files at once.

## Install

From source (needs a Rust toolchain, see [Building](#building-from-source)):

```sh
cargo install --git https://github.com/thetasdev/susut
```

Once published on crates.io: `cargo install susut`.

Prebuilt binaries are attached to each
[GitHub release](https://github.com/thetasdev/susut/releases), with SHA-256 checksums:

| OS | Architectures |
|---|---|
| Linux (static, musl: runs on any distro) | x86_64, aarch64 |
| Linux (glibc, built on Ubuntu 24.04) | x86_64, aarch64 |
| macOS | x86_64 (Intel), arm64 (Apple silicon) |
| Windows | x86_64, arm64 (AVIF encoding is slower: no SIMD assembly) |

A Homebrew formula is planned.

## Usage

```sh
susut photo.jpg                          # -> photo.min.jpg at quality 80
susut photo.png -f jpeg -s 200KB         # convert, at most 200 KB
susut photo.jpg -s 50%                   # at most half the original size
susut *.jpg --resize 50% -n              # dry run: estimate without writing
susut -r Photos/ -f webp -L 1600 -o out/ # a whole tree, structure preserved
```

Without `-o`, the result is written next to the input as `<name>.min.<ext>`;
files that already end in `.min` are skipped when scanning folders.

The exit code is `0` if every file succeeded (or was skipped) and `1` if any file failed.

## Options

| Option | Description |
|---|---|
| `-o, --output <PATH>` | Output file (single input) or directory (end with `/`) |
| `-f, --format <FMT>` | `jpeg`, `png`, `webp` or `avif` (default: same as input) |
| `-q, --quality <1-100>` | Quality, default `80` (ignored for PNG, which is lossless) |
| `-s, --max-size <SIZE>` | Size ceiling: `200KB`, `1.5MB`, `50000` (bytes) or `50%` of the original |
| `--min-quality <1-100>` | Lowest quality tried for `-s` before downscaling, default `40` |
| `--resize <PCT>` | Scale, e.g. `50%` |
| `-W, --max-width <PX>` | Maximum width |
| `-H, --max-height <PX>` | Maximum height |
| `-L, --long-edge <PX>` | Maximum longer side (good for mixed portrait/landscape) |
| `--short-edge <PX>` | Maximum shorter side |
| `-r, --recursive` | Recurse into directories (structure kept under `-o`) |
| `--min-savings <SIZE>` | Only write if at least this much is saved: `10%`, `50KB` |
| `--force` | Write even if the result is not smaller |
| `-n, --dry-run` | Show results without writing files |
| `-y, --overwrite` | Overwrite existing output files |
| `--lang <en\|id>` | Language of runtime messages (env: `SUSUT_LANG`), default `en` |

Resize options combine: the strictest limit wins, and images are never enlarged.
Inputs can be JPEG, PNG, WebP, AVIF, GIF (first frame only), BMP and TIFF.
Output is JPEG, PNG, WebP or AVIF. Transparent images converted to JPEG are
flattened onto white.

`-s` is a ceiling, not an exact target: if the image already fits at the chosen
quality, it is not made smaller. For very small targets susut reduces the
dimensions as well, which may shrink the picture more than you expect. See
[docs/target-size.md](docs/target-size.md).

## Privacy

susut re-encodes every image from decoded pixels and writes none of the original
metadata: EXIF (camera, timestamps, **GPS location**), XMP, IPTC, comments and
embedded thumbnails are gone from the output. The EXIF orientation is applied to
the pixels first, so photos do not end up rotated.

One side effect: embedded ICC color profiles are dropped too, so images in wide
gamuts (Display P3, Adobe RGB) may look slightly different afterwards.

## Compared with similar tools

Features are based on each project's own README at the time of writing. Speed and output
size were measured, see [Speed](#speed).

| | susut | [caesiumclt](https://github.com/Lymphatus/caesium-clt) | [rimage](https://github.com/SalOne22/rimage) | [jpegoptim](https://github.com/tjko/jpegoptim) |
|---|---|---|---|---|
| Language | Rust | Rust | Rust | C |
| Formats | JPEG, PNG, WebP, AVIF | JPEG, PNG, WebP, GIF | JPEG (mozjpeg), PNG, WebP, AVIF, JPEG XL, QOI, … | JPEG only |
| Target file size | `-s` (bytes or %) | `--max-size` | not documented in its README | `--size` |
| Resize | %, width, height, long/short edge | width, height, long/short edge | several forms (`WxH`, `500l`, `50%`, …) | no |
| Lossless mode | PNG only | yes | codec dependent | yes (default) |
| Keeps metadata | never | optional (`-e`) | see its docs | keeps unless `--strip-*` |
| License | MIT OR Apache-2.0 | custom, see repo | Apache-2.0 | GPL-3.0 |

Pick caesiumclt if you need lossless JPEG recompression or GIF; rimage if you need
JPEG XL or fine codec control; jpegoptim for lossless JPEG optimization in a script.
Pick susut if you want a small CLI that stays out of the way, hits a size target,
and always drops metadata.

## Speed

12 synthetic images on an Apple M3 (8 cores), quality 80:

| | susut | caesiumclt | rimage |
|---|---|---|---|
| JPEG → JPEG, 12 images | **0.78 s** | 0.89 s | 0.86 s |
| JPEG → WebP, 12 images | **0.94 s** | 1.25 s | 1.03 s |
| JPEG → AVIF, 12 images | 2.04 s | no AVIF | 2.04 s |
| PNG lossless, 12 images | 3.00 s | 3.27 s | 2.92 s |
| One JPEG | 156 ms | 148 ms | 150 ms |

All of them call the same encoders, so output files are identical (WebP, AVIF, PNG) or nearly so
(JPEG); only the time differs. susut's edge is parallel batches; for a single file or for
AVIF and PNG it is a tie, and run-to-run noise is around 10%. jpegoptim is about four times faster per
JPEG only because it recompresses without decoding (and writes 43% larger files).
On 11 real NASA photos the picture is the same, and resizing 45-megapixel files is the
fastest of the three too (0.82 s against 0.94 s for rimage and 1.71 s for caesiumclt). The binary is
5.9 MB (macOS arm64; 6.0 MB static musl) against 6.3 MB for caesiumclt and
9.0 MB for rimage with its default features. The table uses synthetic images; method, caveats and a
reproducible script are in [docs/benchmarks.md](docs/benchmarks.md).

## Building from source

You need a Rust toolchain (1.88 or newer) and a C compiler. On **x86_64** you also
need [`nasm`](https://www.nasm.us/) because mozjpeg and rav1e build SIMD
assembly there; ARM builds do not need it.

```sh
# macOS: brew install nasm        Debian/Ubuntu: apt install nasm
# Windows: choco install nasm
git clone https://github.com/thetasdev/susut
cd susut
cargo build --release     # binary in target/release/susut
cargo test
```

## Contributing

Bug reports and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
Because susut parses untrusted files, please report security problems privately
as described in [SECURITY.md](SECURITY.md). This project follows the
[Code of Conduct](CODE_OF_CONDUCT.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. Unless you state otherwise, any contribution you submit for
inclusion is dual licensed as above, without additional terms or conditions.

Bundled encoders keep their own licenses (mozjpeg: IJG/Zlib/BSD-3-Clause,
libwebp: BSD-3-Clause, rav1e: BSD-2-Clause, oxipng: MIT).
