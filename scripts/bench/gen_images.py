"""Generate the synthetic benchmark images (needs Pillow).

Usage: python3 gen_images.py [output-dir]

Creates, inside output-dir:
  imgs/       12 JPEGs, 3000x2000: smooth gradients + blobs + noise (half of them noisy)
  small_jpg/  the same 12 images at 1500x1000 (JPEG)
  small_png/  the same 12 images at 1500x1000 (PNG, hard to compress losslessly)
  ui_png/     12 flat UI-like PNGs, 1500x1000: rectangles and text (typical lossless gains)
"""
import math
import os
import random
import sys

from PIL import Image, ImageDraw, ImageFilter

ROOT = sys.argv[1] if len(sys.argv) > 1 else "bench-images"
for sub in ("imgs", "small_jpg", "small_png", "ui_png"):
    os.makedirs(f"{ROOT}/{sub}", exist_ok=True)

W, H = 3000, 2000
for n in range(12):
    random.seed(100 + n)
    img = Image.new("RGB", (W, H))
    px = img.load()
    a, b, c = (random.uniform(150, 400) for _ in range(3))
    for y in range(H):
        for x in range(W):
            t = y / H
            px[x, y] = (
                int(40 + 150 * t + 30 * math.sin(x / a)),
                int(90 + 110 * t + 25 * math.sin(x / b + 1)),
                int(180 - 90 * t + 20 * math.cos(x / c)),
            )
    d = ImageDraw.Draw(img)
    for _ in range(60):
        x, y = random.randrange(W), random.randrange(H)
        r = random.randrange(40, 260)
        d.ellipse([x - r, y - r, x + r, y + r], fill=tuple(random.randrange(30, 230) for _ in range(3)))
    img = img.filter(ImageFilter.GaussianBlur(random.choice([3, 6, 10])))
    amt = 0.08 if n % 2 == 0 else 0.30  # odd images are noisy (harder to compress)
    img = Image.blend(img, Image.effect_noise((W, H), random.choice([14, 30])).convert("RGB"), amt)
    img.save(f"{ROOT}/imgs/img{n:02d}.jpg", quality=92)
    # the small sets come from the decoded JPEG, like a real "photo" input
    small = Image.open(f"{ROOT}/imgs/img{n:02d}.jpg").convert("RGB").resize((1500, 1000), Image.LANCZOS)
    small.save(f"{ROOT}/small_jpg/img{n:02d}.jpg", quality=92)
    small.save(f"{ROOT}/small_png/img{n:02d}.png")

for n in range(12):
    random.seed(500 + n)
    im = Image.new("RGB", (1500, 1000), (245, 246, 248))
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, 1500, 70], fill=(30, 41, 59))
    for _ in range(18):
        x, y = random.randrange(40, 1100), random.randrange(100, 900)
        w, h = random.randrange(120, 380), random.randrange(50, 160)
        d.rounded_rectangle([x, y, x + w, y + h], radius=10,
                            fill=tuple(random.randrange(60, 250) for _ in range(3)), outline=(200, 200, 205))
    for _ in range(40):
        d.text((random.randrange(30, 1300), random.randrange(90, 950)),
               "Susut sample text %d" % random.randrange(10000), fill=(20, 20, 30))
    im.save(f"{ROOT}/ui_png/ui{n:02d}.png")
