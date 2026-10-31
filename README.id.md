# susut

**Perkecil gambar dari command line: kompres, resize, dan konversi JPEG, PNG, WebP, dan AVIF, bisa sampai ukuran target.**

[English](README.md) · [Bahasa Indonesia](README.id.md)

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

> Gambar demo adalah gambar sintetis 3000×2000 yang halus, jadi persentasenya
> terlalu optimistis. Foto asli biasanya menyusut lebih sedikit. Jalankan
> `susut -n` (dry run) pada berkasmu sendiri untuk melihat perkiraannya.
> Pesan di atas tampil dalam bahasa Inggris (default); tambahkan `--lang id` untuk bahasa Indonesia.

- **Encoder:** mozjpeg (JPEG), oxipng (PNG), libwebp (WebP), rav1e lewat ravif (AVIF).
- **Target ukuran:** `-s 200KB` mencari kualitas terbaik yang muat, dan baru memperkecil dimensi jika terpaksa.
- **Aman secara default:** tidak menimpa file, tidak menulis hasil yang lebih besar dari aslinya, tidak mengubah file input.
- **Privat secara default:** semua metadata (EXIF, GPS, dll.) dibuang dari hasil.
- **Paralel:** memproses banyak file sekaligus.

## Instalasi

Dari source (butuh Rust, lihat [Build dari source](#build-dari-source)):

```sh
cargo install --git https://github.com/thetasdev/susut
```

Setelah terbit di crates.io: `cargo install susut`.

Binary siap pakai dilampirkan di setiap
[rilis GitHub](https://github.com/thetasdev/susut/releases), lengkap dengan checksum SHA-256:

| OS | Arsitektur |
|---|---|
| Linux (static, musl: jalan di distro apa pun) | x86_64, aarch64 |
| Linux (glibc, dibangun di Ubuntu 24.04) | x86_64, aarch64 |
| macOS | x86_64 (Intel), arm64 (Apple silicon) |
| Windows | x86_64, arm64 (encode AVIF lebih lambat: tanpa assembly SIMD) |

Formula Homebrew direncanakan.

## Pemakaian

```sh
susut photo.jpg                          # -> photo.min.jpg, kualitas 80
susut photo.png -f jpeg -s 200KB         # konversi, maksimal 200 KB
susut photo.jpg -s 50%                   # maksimal setengah ukuran asli
susut *.jpg --resize 50% -n              # dry run: estimasi tanpa menulis
susut -r Photos/ -f webp -L 1600 -o out/ # satu pohon folder, struktur dipertahankan
```

Tanpa `-o`, hasil ditulis di samping file input sebagai `<nama>.min.<ext>`;
file yang sudah berakhiran `.min` dilewati saat memindai folder.

Exit code `0` jika semua file berhasil (atau dilewati) dan `1` jika ada yang gagal.

## Opsi

| Opsi | Keterangan |
|---|---|
| `-o, --output <PATH>` | File output (satu input) atau folder (akhiri dengan `/`) |
| `-f, --format <FMT>` | `jpeg`, `png`, `webp`, atau `avif` (default: sama dengan input) |
| `-q, --quality <1-100>` | Kualitas, default `80` (diabaikan untuk PNG yang lossless) |
| `-s, --max-size <UKURAN>` | Batas ukuran: `200KB`, `1.5MB`, `50000` (byte) atau `50%` dari ukuran asli |
| `--min-quality <1-100>` | Kualitas terendah yang dicoba untuk `-s` sebelum diperkecil, default `40` |
| `--resize <PCT>` | Skala, mis. `50%` |
| `-W, --max-width <PX>` | Lebar maksimal |
| `-H, --max-height <PX>` | Tinggi maksimal |
| `-L, --long-edge <PX>` | Sisi terpanjang maksimal (cocok untuk campuran portrait/landscape) |
| `--short-edge <PX>` | Sisi terpendek maksimal |
| `-r, --recursive` | Proses folder rekursif (struktur dipertahankan di `-o`) |
| `--min-savings <UKURAN>` | Hanya tulis jika hemat minimal sebesar ini: `10%`, `50KB` |
| `--force` | Tulis walau hasil tidak lebih kecil |
| `-n, --dry-run` | Tampilkan hasil tanpa menulis file |
| `-y, --overwrite` | Timpa file output yang sudah ada |
| `--lang <en\|id>` | Bahasa pesan runtime (env: `SUSUT_LANG`), default `en` |

Opsi resize digabung: batas paling ketat yang menang, dan gambar tidak pernah diperbesar.
Input bisa JPEG, PNG, WebP, AVIF, GIF (hanya frame pertama), BMP, dan TIFF.
Output berupa JPEG, PNG, WebP, atau AVIF. Gambar transparan yang dikonversi ke JPEG
diratakan di atas latar putih.

`-s` adalah batas atas, bukan target persis: jika gambar sudah muat pada kualitas
yang dipilih, ukurannya tidak diperkecil lagi. Untuk target yang sangat kecil,
susut juga memperkecil dimensi, yang bisa lebih banyak dari perkiraanmu. Lihat
[docs/target-size.md](docs/target-size.md) (dalam bahasa Inggris).

## Privasi

susut meng-encode ulang setiap gambar dari piksel hasil decode dan tidak menulis
metadata asli: EXIF (kamera, waktu, **lokasi GPS**), XMP, IPTC, komentar, dan
thumbnail tertanam tidak ada di hasil. Orientasi EXIF diterapkan ke piksel lebih
dulu, sehingga foto tidak berakhir terputar.

Efek samping: profil warna ICC tertanam ikut hilang, jadi gambar dengan gamut luas
(Display P3, Adobe RGB) bisa tampak sedikit berbeda.

## Perbandingan dengan alat lain

Fitur berdasarkan README masing-masing proyek saat penulisan. Kecepatan dan ukuran
hasil sudah diukur, lihat [Kecepatan](#kecepatan).

| | susut | [caesiumclt](https://github.com/Lymphatus/caesium-clt) | [rimage](https://github.com/SalOne22/rimage) | [jpegoptim](https://github.com/tjko/jpegoptim) |
|---|---|---|---|---|
| Bahasa | Rust | Rust | Rust | C |
| Format | JPEG, PNG, WebP, AVIF | JPEG, PNG, WebP, GIF | JPEG (mozjpeg), PNG, WebP, AVIF, JPEG XL, QOI, … | hanya JPEG |
| Target ukuran | `-s` (byte atau %) | `--max-size` | tidak ada di README-nya | `--size` |
| Resize | %, lebar, tinggi, sisi panjang/pendek | lebar, tinggi, sisi panjang/pendek | beberapa bentuk (`WxH`, `500l`, `50%`, …) | tidak ada |
| Mode lossless | hanya PNG | ya | tergantung codec | ya (default) |
| Metadata | selalu dibuang | opsional (`-e`) | lihat dokumentasinya | dipertahankan kecuali `--strip-*` |
| Lisensi | MIT OR Apache-2.0 | kustom, lihat repo | Apache-2.0 | GPL-3.0 |

Pilih caesiumclt jika butuh rekompresi JPEG lossless atau GIF; rimage jika butuh
JPEG XL atau kontrol codec yang halus; jpegoptim untuk optimasi JPEG lossless di
skrip. Pilih susut jika ingin CLI kecil yang tidak merepotkan, bisa mengejar
target ukuran, dan selalu membuang metadata.

## Kecepatan

12 gambar sintetis di Apple M3 (8 core), kualitas 80:

| | susut | caesiumclt | rimage |
|---|---|---|---|
| JPEG → JPEG, 12 gambar | **0,78 dtk** | 0,89 dtk | 0,86 dtk |
| JPEG → WebP, 12 gambar | **0,94 dtk** | 1,25 dtk | 1,03 dtk |
| JPEG → AVIF, 12 gambar | 2,04 dtk | tanpa AVIF | 2,04 dtk |
| PNG lossless, 12 gambar | 3,00 dtk | 3,27 dtk | 2,92 dtk |
| Satu JPEG | 156 ms | 148 ms | 150 ms |

Ketiganya memanggil encoder yang sama, jadi file hasilnya identik (WebP, AVIF, PNG) atau hampir
identik (JPEG); yang beda hanya waktu. Keunggulan susut ada di batch paralel; untuk satu file,
AVIF, dan PNG hasilnya seri, dan selisih antar-run sekitar 10%. jpegoptim sekitar empat kali lebih
cepat per JPEG hanya karena mengompres ulang tanpa decode (dan hasilnya 43% lebih besar).
Pada 11 foto asli NASA gambarnya sama, dan me-resize file 45 megapiksel pun paling cepat
dari ketiganya (0,82 dtk dibanding 0,94 dtk untuk rimage dan 1,71 dtk untuk caesiumclt).
Ukuran binary susut 5,9 MB (macOS arm64; 6,0 MB untuk musl static), dibanding 6,3 MB untuk
caesiumclt dan 9,0 MB untuk rimage dengan fitur bawaannya. Tabel memakai gambar sintetis; metode, batasan,
dan skrip yang bisa diulang ada di [docs/benchmarks.md](docs/benchmarks.md) (bahasa Inggris).

## Build dari source

Butuh Rust (1.88 atau lebih baru) dan compiler C. Di **x86_64** kamu juga butuh
[`nasm`](https://www.nasm.us/) karena mozjpeg dan rav1e membangun assembly SIMD di
sana; build ARM tidak membutuhkannya.

```sh
# macOS: brew install nasm        Debian/Ubuntu: apt install nasm
# Windows: choco install nasm
git clone https://github.com/thetasdev/susut
cd susut
cargo build --release     # binary di target/release/susut
cargo test
```

## Kontribusi

Laporan bug dan pull request dipersilakan; lihat [CONTRIBUTING.md](CONTRIBUTING.md).
Karena susut memproses file yang tidak tepercaya, laporkan masalah keamanan secara
privat seperti dijelaskan di [SECURITY.md](SECURITY.md). Proyek ini mengikuti
[Code of Conduct](CODE_OF_CONDUCT.md).

## Lisensi

Berlisensi [Apache License 2.0](LICENSE-APACHE) atau [MIT](LICENSE-MIT), sesuai
pilihanmu. Kontribusi yang kamu kirim otomatis berlisensi ganda seperti di atas,
tanpa syarat tambahan.

Encoder yang disertakan memakai lisensinya masing-masing (mozjpeg: IJG/Zlib/BSD-3-Clause,
libwebp: BSD-3-Clause, rav1e: BSD-2-Clause, oxipng: MIT).
