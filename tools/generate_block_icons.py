#!/usr/bin/env python3
"""Generate authored isometric PNG icons for all normal voxel blocks.

The geometry and face shading match the former HUD block renderer. Namespaced
block ids are stored with their local id so the assets remain Windows-safe:
asteria:stone -> assets/textures/block_icons/stone.png.
"""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
BLOCKS_DIR = ROOT / "data" / "blocks"
ASSETS_DIR = ROOT / "assets"
TEXTURES_DIR = ASSETS_DIR / "textures"
OUTPUT_DIR = TEXTURES_DIR / "block_icons"
LEGACY_OUTPUT_DIR = ASSETS_DIR / "block_icons"

SIZE = 256

# Geometry copied from the old block display renderer.
DISPLAY_LEFT = 0.10
DISPLAY_CENTER_X = 0.50
DISPLAY_TOP_Y = 0.27
DISPLAY_HALF_WIDTH = 0.40
DISPLAY_SLOPE = 0.20
DISPLAY_SIDE_HEIGHT = 0.50

# Same shading as block_display_face_shade().
FACE_SHADE = {
    "top": 1.00,
    "front": 0.86,
    "right": 0.74,
}

FACE_BASIS = {
    "top": (
        (DISPLAY_LEFT, DISPLAY_TOP_Y),
        (DISPLAY_HALF_WIDTH, DISPLAY_SLOPE),
        (DISPLAY_HALF_WIDTH, -DISPLAY_SLOPE),
    ),
    "front": (
        (DISPLAY_LEFT, DISPLAY_TOP_Y),
        (DISPLAY_HALF_WIDTH, DISPLAY_SLOPE),
        (0.0, DISPLAY_SIDE_HEIGHT),
    ),
    "right": (
        (DISPLAY_CENTER_X, DISPLAY_TOP_Y + DISPLAY_SLOPE),
        (DISPLAY_HALF_WIDTH, -DISPLAY_SLOPE),
        (0.0, DISPLAY_SIDE_HEIGHT),
    ),
}


def srgb_to_linear(c: float) -> float:
    if c <= 0.04045:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def linear_to_srgb(c: float) -> float:
    c = max(0.0, min(1.0, c))
    if c <= 0.0031308:
        return c * 12.92
    return 1.055 * (c ** (1.0 / 2.4)) - 0.055


def shade_pixel(pixel: tuple[int, int, int, int], shade: float) -> tuple[int, int, int, int]:
    r, g, b, a = pixel
    linear = [srgb_to_linear(v / 255.0) * shade for v in (r, g, b)]
    rgb = tuple(round(linear_to_srgb(v) * 255.0) for v in linear)
    return rgb + (a,)


def alpha_over(base: tuple[int, int, int, int], overlay: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    br, bg, bb, ba8 = base
    or_, og, ob, oa8 = overlay
    ba = ba8 / 255.0
    oa = oa8 / 255.0
    out_a = oa + ba * (1.0 - oa)
    if out_a <= 1e-9:
        return (0, 0, 0, 0)
    out_rgb = []
    for bc, oc in ((br, or_), (bg, og), (bb, ob)):
        value = (oc * oa + bc * ba * (1.0 - oa)) / out_a
        out_rgb.append(round(value))
    return tuple(out_rgb) + (round(out_a * 255.0),)


def normalize_layers(spec: Any) -> list[dict[str, Any]]:
    if isinstance(spec, str):
        return [{"texture": spec}]
    if isinstance(spec, dict):
        if "texture" in spec:
            return [{"texture": spec["texture"]}]
        if "layers" in spec:
            return normalize_layers(spec["layers"])
    if isinstance(spec, list):
        result: list[dict[str, Any]] = []
        for layer in spec:
            result.extend(normalize_layers(layer))
        return result
    raise ValueError(f"Unsupported face texture specification: {spec!r}")


def load_texture(path: str) -> Image.Image:
    candidates = [ASSETS_DIR / path, TEXTURES_DIR / path]
    for texture_path in candidates:
        if texture_path.is_file():
            return Image.open(texture_path).convert("RGBA")
    raise FileNotFoundError(f"Missing texture: {path}")


def prepare_face_layers(spec: Any) -> list[Image.Image]:
    return [load_texture(layer["texture"]) for layer in normalize_layers(spec)]


def face_spec(textures: dict[str, Any], face: str) -> Any:
    aliases = {
        "top": ("top", "all"),
        "front": ("front", "side", "all"),
        "right": ("right", "side", "all"),
    }
    for key in aliases[face]:
        if key in textures:
            return textures[key]
    raise KeyError(f"Block has no texture for visible face {face!r}")


def sample_layers(layers: list[Image.Image], u: float, v: float) -> tuple[int, int, int, int]:
    out = (0, 0, 0, 0)
    for image in layers:
        sx = min(image.width - 1, max(0, int(u * image.width)))
        sy = min(image.height - 1, max(0, int(v * image.height)))
        out = alpha_over(out, image.getpixel((sx, sy)))
    return out


def draw_face(canvas: Image.Image, face: str, layers: list[Image.Image]) -> None:
    origin, axis_u, axis_v = FACE_BASIS[face]
    ox, oy = origin
    ux, uy = axis_u
    vx, vy = axis_v
    det = ux * vy - uy * vx

    corners = [
        (ox, oy),
        (ox + ux, oy + uy),
        (ox + ux + vx, oy + uy + vy),
        (ox + vx, oy + vy),
    ]
    min_x = max(0, math.floor(min(p[0] for p in corners) * SIZE) - 1)
    max_x = min(SIZE - 1, math.ceil(max(p[0] for p in corners) * SIZE) + 1)
    min_y = max(0, math.floor(min(p[1] for p in corners) * SIZE) - 1)
    max_y = min(SIZE - 1, math.ceil(max(p[1] for p in corners) * SIZE) + 1)

    pixels = canvas.load()
    for y in range(min_y, max_y + 1):
        py = (y + 0.5) / SIZE
        for x in range(min_x, max_x + 1):
            px = (x + 0.5) / SIZE
            dx = px - ox
            dy = py - oy
            u = (dx * vy - dy * vx) / det
            v = (ux * dy - uy * dx) / det
            if 0.0 <= u < 1.0 and 0.0 <= v < 1.0:
                pixel = sample_layers(layers, u, v)
                pixels[x, y] = shade_pixel(pixel, FACE_SHADE[face])


def generate_icon(block: dict[str, Any]) -> Image.Image:
    textures = block.get("textures", block.get("faces"))
    if not isinstance(textures, dict):
        raise ValueError(f"{block.get('id', '<unknown>')}: missing textures object")

    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    for face in ("front", "right", "top"):
        draw_face(canvas, face, prepare_face_layers(face_spec(textures, face)))
    return canvas


def should_skip(path: Path, block: dict[str, Any]) -> bool:
    block_id = str(block.get("id", ""))
    return path.stem.endswith("_layer") or block_id.endswith("_layer")


def local_block_id(block_id: str) -> str:
    return block_id.split(":", 1)[1] if ":" in block_id else block_id


def clear_generated(directory: Path) -> None:
    if not directory.exists():
        return
    for stale in directory.rglob("*.png"):
        stale.unlink()
    gitkeep = directory / ".gitkeep"
    if gitkeep.is_file():
        gitkeep.unlink()
    try:
        directory.rmdir()
    except OSError:
        pass


def main() -> None:
    clear_generated(LEGACY_OUTPUT_DIR)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for stale in OUTPUT_DIR.rglob("*.png"):
        stale.unlink()

    generated = 0
    skipped = 0
    for definition_path in sorted(BLOCKS_DIR.glob("*.json")):
        block = json.loads(definition_path.read_text(encoding="utf-8"))
        if should_skip(definition_path, block):
            skipped += 1
            continue

        block_id = block.get("id")
        if not isinstance(block_id, str) or not block_id:
            raise ValueError(f"{definition_path}: missing block id")

        icon = generate_icon(block)
        output = OUTPUT_DIR / f"{local_block_id(block_id)}.png"
        output.parent.mkdir(parents=True, exist_ok=True)
        icon.save(output, format="PNG", optimize=True)
        generated += 1
        print(f"generated {output.relative_to(ROOT)}")

    print(f"done: {generated} icons generated, {skipped} layer blocks skipped")


if __name__ == "__main__":
    main()
