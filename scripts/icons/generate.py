#!/usr/bin/env python3
"""Generate Sightline icon assets from a single silhouette.

Produces:
  * src-tauri/icons/icon.png          — 512×512 colour (app bundle)
  * src-tauri/icons/128x128.png       — 128×128 colour
  * src-tauri/icons/128x128@2x.png    — 256×256 colour
  * src-tauri/icons/32x32.png         —  32×32 colour
  * src-tauri/icons/tray-16.png       —  16×16 colour (Linux fallback)
  * src-tauri/icons/tray-22.png       —  22×22 colour (Linux StatusNotifierItem)
  * src-tauri/icons/tray-32.png       —  32×32 colour (Windows / Linux hi-dpi)
  * src-tauri/icons/tray-template.png —  32×32 template (macOS menu-bar)
  * src-tauri/icons/tray-template@2x.png — 64×64 retina template
  * src-tauri/icons/icon.ico          — multi-resolution Windows .ico
  * src-tauri/icons/icon.icns         — macOS .icns bundle icon

The silhouette is intentionally simple — two concentric rings + a
small horizontal sweep mark, suggestive of "sight lines" / a camera
lens. The same glyph renders correctly at 16 px in the menu bar.

No third-party dependencies: pure stdlib (zlib, struct).
"""
from __future__ import annotations

import argparse
import math
import os
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = ROOT / "src-tauri" / "icons"

# Brand palette — light accent on dark background. Matches the
# `--color-accent` token in src/styles/globals.css (amber-ish).
BG = (24, 26, 31, 255)        # dark neutral
FG = (235, 200, 90, 255)      # amber foreground
GLYPH_MARGIN = 0.12           # % of size kept clear at edges


# --- PNG encoder ---------------------------------------------------------

def _chunk(tag: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + tag + data + struct.pack(
        ">I", zlib.crc32(tag + data) & 0xFFFFFFFF
    )


def write_png(path: Path, pixels: list[list[tuple[int, int, int, int]]]) -> None:
    """Encode an RGBA pixel grid to a PNG file."""
    height = len(pixels)
    width = len(pixels[0]) if height else 0
    raw = bytearray()
    for row in pixels:
        raw.append(0)  # filter: None
        for r, g, b, a in row:
            raw.extend((r, g, b, a))
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    signature = b"\x89PNG\r\n\x1a\n"
    with path.open("wb") as fh:
        fh.write(signature)
        fh.write(_chunk(b"IHDR", header))
        fh.write(_chunk(b"IDAT", idat))
        fh.write(_chunk(b"IEND", b""))


# --- Glyph renderer ------------------------------------------------------

def _blend(under: tuple[int, int, int, int], over: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    oa = over[3] / 255.0
    ua = under[3] / 255.0
    out_a = oa + ua * (1 - oa)
    if out_a == 0:
        return (0, 0, 0, 0)
    out = []
    for i in range(3):
        c = (over[i] * oa + under[i] * ua * (1 - oa)) / out_a
        out.append(int(round(c)))
    out.append(int(round(out_a * 255)))
    return tuple(out)  # type: ignore[return-value]


def _paint_glyph(size: int, fg: tuple[int, int, int, int], bg: tuple[int, int, int, int]) -> list[list[tuple[int, int, int, int]]]:
    """Render the Sightline glyph at `size`×`size` with simple AA."""
    grid: list[list[tuple[int, int, int, int]]] = [[bg for _ in range(size)] for _ in range(size)]
    cx = cy = (size - 1) / 2.0
    margin = size * GLYPH_MARGIN
    outer_r = (size / 2.0) - margin
    inner_r = outer_r * 0.55
    stroke_outer = max(1.0, outer_r * 0.16)
    # "Sweep" tick: a short horizontal mark at mid-height extending from
    # the outer ring to the edge, evoking a scan line.
    sweep_y = cy
    sweep_x0 = cx + inner_r * 1.15
    sweep_x1 = cx + outer_r * 1.02
    sweep_thickness = max(1.0, outer_r * 0.20)

    for y in range(size):
        for x in range(size):
            # Subpixel super-sampling, 2×2.
            covered = 0.0
            for sy in (0.25, 0.75):
                for sx in (0.25, 0.75):
                    px = x + sx
                    py = y + sy
                    dx = px - cx
                    dy = py - cy
                    r = math.sqrt(dx * dx + dy * dy)
                    in_ring = (
                        (outer_r - stroke_outer) <= r <= outer_r
                        or (inner_r - stroke_outer * 0.75) <= r <= inner_r
                    )
                    in_sweep = (
                        sweep_x0 <= px <= sweep_x1
                        and abs(py - sweep_y) <= sweep_thickness / 2.0
                    )
                    if in_ring or in_sweep:
                        covered += 1.0
            a = covered / 4.0
            if a > 0:
                glyph = (fg[0], fg[1], fg[2], int(round(fg[3] * a)))
                grid[y][x] = _blend(bg, glyph)
    return grid


def _template_glyph(size: int) -> list[list[tuple[int, int, int, int]]]:
    """Render the glyph as a macOS template image: pure black on
    transparent so the OS can colourize it based on menu-bar theme."""
    return _paint_glyph(size, fg=(0, 0, 0, 255), bg=(0, 0, 0, 0))


# --- ICO writer ----------------------------------------------------------

def write_ico(path: Path, sizes: list[int]) -> None:
    images: list[tuple[int, bytes]] = []
    for size in sizes:
        grid = _paint_glyph(size, fg=FG, bg=BG)
        tmp = path.with_suffix(f".{size}.png")
        write_png(tmp, grid)
        data = tmp.read_bytes()
        tmp.unlink()
        images.append((size, data))
    num = len(images)
    with path.open("wb") as fh:
        # ICONDIR: reserved(2) + type(2=icon) + count(2).
        fh.write(struct.pack("<HHH", 0, 1, num))
        offset = 6 + 16 * num
        for size, data in images:
            # ICONDIRENTRY: width, height, colors, reserved, planes,
            # bit-count, size, offset. Width/Height 0 means 256.
            w = size if size < 256 else 0
            h = size if size < 256 else 0
            fh.write(
                struct.pack(
                    "<BBBBHHII",
                    w,
                    h,
                    0,
                    0,
                    1,
                    32,
                    len(data),
                    offset,
                )
            )
            offset += len(data)
        for _, data in images:
            fh.write(data)


# --- ICNS writer ---------------------------------------------------------
# Apple ICNS format: magic "icns" + file length + sequence of type/len/data
# chunks. We write the PNG-carrying types (ic07..ic14) which macOS 10.7+
# understands. No JPEG-2000 support needed.

# (size, type-code) tuples — sizes that macOS expects for an app icon.
ICNS_VARIANTS: list[tuple[int, bytes]] = [
    (16, b"icp4"),   # 16×16
    (32, b"icp5"),   # 32×32
    (64, b"icp6"),   # 64×64
    (128, b"ic07"),  # 128×128
    (256, b"ic08"),  # 256×256
    (512, b"ic09"),  # 512×512
    (1024, b"ic10"), # 1024×1024 (retina)
]


def write_icns(path: Path) -> None:
    chunks: list[bytes] = []
    for size, ty in ICNS_VARIANTS:
        grid = _paint_glyph(size, fg=FG, bg=BG)
        tmp = path.with_suffix(f".{size}.png")
        write_png(tmp, grid)
        data = tmp.read_bytes()
        tmp.unlink()
        chunk_len = 8 + len(data)
        chunks.append(ty + struct.pack(">I", chunk_len) + data)
    body = b"".join(chunks)
    total = 8 + len(body)
    with path.open("wb") as fh:
        fh.write(b"icns" + struct.pack(">I", total) + body)


# --- Entrypoint ----------------------------------------------------------

def ensure_out() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)


def generate() -> None:
    ensure_out()
    app_sizes = [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png"), (512, "icon.png")]
    for size, name in app_sizes:
        write_png(OUT_DIR / name, _paint_glyph(size, FG, BG))

    # Tray icons. Coloured PNGs for Linux + Windows; template for macOS.
    write_png(OUT_DIR / "tray-16.png", _paint_glyph(16, FG, BG))
    write_png(OUT_DIR / "tray-22.png", _paint_glyph(22, FG, BG))
    write_png(OUT_DIR / "tray-32.png", _paint_glyph(32, FG, BG))
    write_png(OUT_DIR / "tray-template.png", _template_glyph(32))
    write_png(OUT_DIR / "tray-template@2x.png", _template_glyph(64))

    # Multi-resolution Windows icon.
    write_ico(OUT_DIR / "icon.ico", [16, 24, 32, 48, 64, 128, 256])

    # macOS .icns bundle icon.
    write_icns(OUT_DIR / "icon.icns")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Generate Sightline icon assets.")
    parser.parse_args(argv)
    generate()
    print(f"wrote icons to {OUT_DIR.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
