# How `--max-size` works, and where it falls short

`-s` / `--max-size` asks for an output file **no larger than** a limit
(`200KB`, `1.5MB`, `50000` bytes, or `50%` of the original file size). The
limit is a ceiling, not an exact target. The implementation is
`encode_to_target` in `src/lib.rs`.

## Algorithm

1. **Resize options first.** `--resize`, `-W`, `-H`, `-L` and `--short-edge` are
   applied before anything else, so the search starts from the resized image.
2. **Try the top quality.** The image is encoded at `-q` (default 80). If the
   result fits, it is used as is. A generous limit therefore never lowers the quality.
3. **Binary search the quality.** Otherwise susut searches the integers in
   `[--min-quality, -q - 1]` (default 40 to 79) for the *highest* quality that fits.
   It needs about 6 encodes.
4. **Downscale if nothing fits.** If even `--min-quality` is too big, the
   dimensions are reduced and the search restarts at `-q`:

   ```
   scale = clamp( sqrt(target / current_size) * 0.95, 0.5, 0.9 )
   ```

   This repeats up to 8 times. It stops early if a side would fall below 16 px.
5. **Give up gracefully.** If the target is still not met, the last result is
   written (when it is smaller than the original) and susut prints
   `⚠ size target not reached`. The exit code stays `0`, because the file was
   processed; check the warning if you need a hard guarantee.

PNG is lossless, so quality does not apply: step 3 is skipped and only
downscaling can reduce the size. Because PNG sizes depend mostly on content,
tiny targets for photos in PNG usually shrink the picture a lot.

## Limitations

- **Small targets can shrink the dimensions more than needed.** The scale
  factor is a heuristic taken from the size at the lowest quality, and the
  steps are coarse (each step cuts every side by 10 to 50%). The result can be
  smaller than the minimum that would fit (I have not measured by how much). Example
  from a 3000×2000 photo: `-s 5KB` produced 394×262 pixels at q=43.
  If you care about resolution, set it yourself with `-L` and lower
  `--min-quality`, or raise the limit.
- **Quality is searched in integers, assuming size grows with quality.** That is
  true for almost every image but not guaranteed. The search can miss a slightly
  better quality, but it never returns a file above the limit unless it reports
  that the target was not reached.
- **`50%` is relative to the input file**, which may already be heavily
  compressed. Halving a 100 KB JPEG is much harder than halving a 5 MB one.
- **Cost.** Every attempt is a full encode, so a size target costs roughly 7 to
  60 encodes per image. This is fine for JPEG and WebP but slow for AVIF.
  Use `-n` to preview, and consider a lower AVIF `-q` instead of `-s` for big batches.
- **Metadata is already gone**, so none of the saving can come from it; the
  limit applies to the pixels alone.
- **Not a perceptual guarantee.** susut measures bytes, not visual quality.
  Very low qualities produce visible artifacts; `--min-quality` keeps the floor
  at 40 by default.
