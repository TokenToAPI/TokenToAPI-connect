#!/usr/bin/env python3
"""Generate the Tauri desktop icons from the TokenToAPI brand asset."""
from pathlib import Path
from shutil import copyfile
from subprocess import run
from tempfile import TemporaryDirectory

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "public" / "tokentoapi.png"
ICONS = ROOT / "src-tauri" / "icons"

base = Image.open(SOURCE).convert("RGBA")


def square(size: int) -> Image.Image:
    return base.resize((size, size), Image.LANCZOS)


def write_png(img: Image.Image, rel: str) -> None:
    path = ICONS / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, "PNG", optimize=True)
    print(f"{rel:<48} {img.size[0]}x{img.size[1]}")


# Desktop icons referenced by tauri.conf.json
for size in (32, 128):
    write_png(square(size), f"{size}x{size}.png")
write_png(square(256), "128x128@2x.png")

# Multi-resolution .ico (Windows executable + shortcuts)
ico = square(256)
ico.save(
    ICONS / "icon.ico",
    "ICO",
    sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
print(f"{'icon.ico':<48} multi-size")

# Tauri writes the small icon representations and masks that macOS expects.
with TemporaryDirectory() as output:
    run(
        ["node", str(ROOT / "node_modules/@tauri-apps/cli/tauri.js"), "icon",
         str(SOURCE), "--output", output],
        cwd=ROOT,
        check=True,
    )
    copyfile(Path(output) / "icon.icns", ICONS / "icon.icns")

print("\nAll TokenToAPI desktop icons generated.")
