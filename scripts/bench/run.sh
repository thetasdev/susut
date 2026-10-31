#!/usr/bin/env bash
# Compare susut with caesiumclt, rimage and jpegoptim.
# Needs: hyperfine, caesiumclt, rimage (cargo install rimage --features build-binary), jpegoptim, Pillow.
# Usage: scripts/bench/run.sh [path/to/susut]   (default: target/release/susut)
set -euo pipefail
cd "$(dirname "$0")/../.."
SUSUT=$(realpath "${1:-target/release/susut}")
WORK=$(mktemp -d)
N=$(getconf _NPROCESSORS_ONLN)
python3 scripts/bench/gen_images.py "$WORK"
cd "$WORK"
HF="hyperfine --shell=default --prepare 'rm -rf o_*'"

echo "== JPEG batch: 12 images 3000x2000, q80, $N threads"
eval $HF --warmup 2 --runs 20 \
  -n susut      "'$SUSUT -q 80 -o o_susut/ imgs/'" \
  -n caesiumclt "'caesiumclt -q 80 -o o_cs imgs/*.jpg'" \
  -n rimage     "'rimage mozjpeg -q 80 -t $N -d o_ri imgs/*.jpg'"

echo "== JPEG single image"
eval $HF --warmup 2 --runs 15 \
  -n susut      "'$SUSUT -q 80 -o o_susut/ imgs/img00.jpg'" \
  -n caesiumclt "'caesiumclt -q 80 -o o_cs imgs/img00.jpg'" \
  -n rimage     "'rimage mozjpeg -q 80 -d o_ri imgs/img00.jpg'" \
  -n jpegoptim  "'mkdir -p o_jo && jpegoptim -q -m80 -d o_jo imgs/img00.jpg'"

echo "== JPEG size target 150 KB (img01)"
eval $HF --warmup 1 --runs 8 \
  -n susut      "'$SUSUT -s 150KB -o o_susut/ imgs/img01.jpg'" \
  -n caesiumclt "'caesiumclt --max-size 153600 -o o_cs imgs/img01.jpg'"

echo "== WebP q80: 12 JPEG 3000x2000 -> WebP"
eval $HF --warmup 1 --runs 10 \
  -n susut      "'$SUSUT -f webp -q 80 -o o_susut/ imgs/'" \
  -n caesiumclt "'caesiumclt -q 80 --format webp -o o_cs imgs/*.jpg'" \
  -n rimage     "'rimage webp -q 80 -t $N -d o_ri imgs/*.jpg'"

echo "== AVIF q80, speed 6: 12 JPEG 1500x1000 -> AVIF"
eval $HF --runs 3 \
  -n susut      "'$SUSUT -f avif -q 80 -o o_susut/ small_jpg/'" \
  -n rimage     "'rimage avif -q 80 --speed 6 -t $N -d o_ri small_jpg/*.jpg'"

for set in small_png ui_png; do
  echo "== PNG lossless (oxipng level 2): $set"
  eval $HF --warmup 1 --runs 5 \
    -n susut      "'$SUSUT -f png --force -o o_susut/ $set/'" \
    -n caesiumclt "'caesiumclt --lossless --png-opt-level 2 -o o_cs $set/*.png'" \
    -n rimage     "'rimage oxipng --effort 2 -t $N -d o_ri $set/*.png'"
done

echo "(work dir: $WORK)"
