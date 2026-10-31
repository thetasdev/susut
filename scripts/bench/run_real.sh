#!/usr/bin/env bash
# Benchmark on a folder of real photos (JPEG). Same tools as run.sh.
# Usage: scripts/bench/run_real.sh <photo-dir> [path/to/susut]
set -euo pipefail
PHOTOS=$(realpath "${1:?usage: run_real.sh <photo-dir> [susut]}")
cd "$(dirname "$0")/../.."
SUSUT=$(realpath "${2:-target/release/susut}")
N=$(getconf _NPROCESSORS_ONLN)
cd "$PHOTOS"
HF="hyperfine --shell=default --prepare 'rm -rf o_*'"

echo "== JPEG q80, full resolution"
eval $HF --warmup 1 --runs 6 \
  -n susut      "'$SUSUT -q 80 -o o_susut/ *.jpg'" \
  -n caesiumclt "'caesiumclt -q 80 -o o_cs *.jpg'" \
  -n rimage     "'rimage mozjpeg -q 80 -t $N -d o_ri *.jpg'"
echo "== JPEG q80, long edge 2000"
eval $HF --warmup 1 --runs 6 \
  -n susut      "'$SUSUT -q 80 -L 2000 -o o_susut/ *.jpg'" \
  -n caesiumclt "'caesiumclt -q 80 --long-edge 2000 -o o_cs *.jpg'" \
  -n rimage     "'rimage mozjpeg -q 80 --resize 2000l -t $N -d o_ri *.jpg'"
echo "== WebP q80, full resolution"
eval $HF --runs 3 \
  -n susut      "'$SUSUT -f webp -q 80 -o o_susut/ *.jpg'" \
  -n caesiumclt "'caesiumclt -q 80 --format webp -o o_cs *.jpg'" \
  -n rimage     "'rimage webp -q 80 -t $N -d o_ri *.jpg'"
echo "== AVIF q80 speed 6, long edge 1600"
eval $HF --runs 3 \
  -n susut      "'$SUSUT -f avif -q 80 -L 1600 -o o_susut/ *.jpg'" \
  -n rimage     "'rimage avif -q 80 --speed 6 --resize 1600l -t $N -d o_ri *.jpg'"
rm -rf o_*
