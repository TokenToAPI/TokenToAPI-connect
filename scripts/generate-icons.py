#!/usr/bin/env python3
"""Generate the Tauri desktop icons from the TokenToAPI brand asset."""
from pathlib import Path
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

# .icns (macOS bundle) -- iconutil is unavailable off macOS, so write the
# container directly. Each entry is an ICNS-typed PNG.
ICNS_TYPES = [
    (16, b"icp4"),
    (32, b"icp5"),
    (64, b"icp6"),
    (128, b"ic07"),
    (256, b"ic08"),
    (512, b"ic09"),
    (1024, b"ic10"),
]


def png_chunk(tag: bytes, payload: bytes) -> bytes:
    import struct

    body = tag + payload
    return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib_crc(body))


def zlib_crc(data: bytes) -> int:
    import zlib

    return zlib.crc32(data) & 0xFFFFFFFF


import io

entries = b""
for size, tag in ICNS_TYPES:
    buffer = io.BytesIO()
    square(size).save(buffer, "PNG", optimize=True)
    png_bytes = buffer.getvalue()
    # ICNS entry length includes the 8-byte tag+length header itself.
    entries += tag + (8 + len(png_bytes)).to_bytes(4, "big") + png_bytes
header = b"icns" + (8 + len(entries)).to_bytes(4, "big")
(ICONS / "icon.icns").write_bytes(header + entries)
print(f"{'icon.icns':<48} {len(ICNS_TYPES)} sizes")

print("\nAll TokenToAPI desktop icons generated.")
