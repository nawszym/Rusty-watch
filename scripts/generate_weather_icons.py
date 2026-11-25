"""Generate the bundled Classic weather icon pack from upstream OFL SVG masters.

The watch cannot render SVG, so the masters are committed under
`native/RustyWatch/Assets/Weather/Source/` and rendered to high-resolution PNGs
(512x512) that the editor preview and the exported watchface both use.

Run from anywhere:  python scripts/generate_weather_icons.py

Requires: pymupdf (SVG rasterisation), pillow.  The first run downloads the
upstream SVGs; later runs reuse the committed masters, so regeneration works
offline.
"""

from __future__ import annotations

import urllib.request
from pathlib import Path

import pymupdf
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "native" / "RustyWatch" / "Assets" / "Weather"
SOURCE = ASSETS / "Source"
CLASSIC = ASSETS / "Classic"
UPSTREAM = "https://raw.githubusercontent.com/erikflowers/weather-icons/master/svg/{}.svg"

# ZeppOS v1.0 forecast index -> upstream weather-icons SVG.
ICON_SOURCES = {
    0: "wi-cloudy",                 # Cloudy
    1: "wi-showers",                # Showers
    2: "wi-day-snow",               # Snow showers
    3: "wi-day-sunny",              # Sunny
    4: "wi-cloudy-gusts",           # Overcast
    5: "wi-day-sprinkle",           # Light rain
    6: "wi-snow",                   # Light snow
    7: "wi-day-rain",               # Moderate rain
    8: "wi-day-snow-wind",          # Moderate snow
    9: "wi-snow-wind",              # Heavy snow
    10: "wi-rain-wind",             # Heavy rain
    11: "wi-sandstorm",             # Sandstorm
    12: "wi-sleet",                 # Rain and snow
    13: "wi-fog",                   # Fog
    14: "wi-day-haze",              # Hazy
    15: "wi-thunderstorm",          # Thunderstorm
    16: "wi-night-alt-snow-wind",   # Snowstorm
    17: "wi-smoke",                 # Floating dust
    18: "wi-storm-showers",         # Heavy downpour
    19: "wi-day-hail",              # Rain and hail
    20: "wi-day-thunderstorm",      # Thunderstorm and hail
    21: "wi-rain",                  # Heavy downpour
    22: "wi-dust",                  # Dust
    23: "wi-windy",                 # Heavy sandstorm
    24: "wi-rain-wind",             # Rainstorm
    25: "wi-alien",                 # Unknown
    26: "wi-night-cloudy",          # Cloudy (night)
    27: "wi-night-showers",         # Showers (night)
    28: "wi-night-clear",           # Clear (night)
}

CANVAS = 512
GLYPH_FRACTION = 0.80  # 80% of the square, i.e. ~10% safe padding on each side
RENDER = 2048


def master(index: int) -> Path:
    """Committed SVG master named after the ZeppOS code (stable, offline)."""
    return SOURCE / f"w{index:02}.svg"


def ensure_master(index: int, upstream_name: str) -> str:
    path = master(index)
    if not path.exists():
        SOURCE.mkdir(parents=True, exist_ok=True)
        data = urllib.request.urlopen(UPSTREAM.format(upstream_name), timeout=60).read()
        path.write_bytes(data)
    return path.read_text(encoding="utf-8")


def render_icon(svg: str) -> Image.Image:
    # Upstream paths have no fill; force white so the glyph is visible on the
    # black watchface.  The root fill cascades to all nested paths.
    svg = svg.replace("<svg", '<svg fill="#FFFFFF"', 1)
    document = pymupdf.open(stream=svg.encode("utf-8"), filetype="svg")
    page = document[0]
    scale = RENDER / max(page.rect.width, page.rect.height)
    pixmap = page.get_pixmap(matrix=pymupdf.Matrix(scale, scale), alpha=True)
    image = Image.frombytes("RGBA", (pixmap.width, pixmap.height), pixmap.samples)
    alpha = image.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        raise ValueError("empty SVG glyph")
    glyph = image.crop(bbox)
    target = round(CANVAS * GLYPH_FRACTION)
    scale = min(target / glyph.width, target / glyph.height)
    size = (max(1, round(glyph.width * scale)), max(1, round(glyph.height * scale)))
    glyph = glyph.resize(size, Image.Resampling.LANCZOS)

    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    canvas.paste(glyph, ((CANVAS - size[0]) // 2, (CANVAS - size[1]) // 2))
    # Real firmware paints RGB under A=0 as white rectangles when rotating IMG.
    pixels = canvas.load()
    for y in range(CANVAS):
        for x in range(CANVAS):
            if pixels[x, y][3] == 0:
                pixels[x, y] = (0, 0, 0, 0)
    return canvas


def write_preview() -> None:
    """Same 2x2 composition as weather::generate_preview (3,5,9,15)."""
    size, cell, icon = 192, 96, 80
    canvas = Image.new("RGBA", (size, size), (28, 28, 32, 255))
    for slot, index in enumerate([3, 5, 9, 15]):
        image = Image.open(CLASSIC / f"w{index:02}.png").convert("RGBA")
        image = image.resize((icon, icon), Image.Resampling.LANCZOS)
        x = (slot % 2) * cell + (cell - icon) // 2
        y = (slot // 2) * cell + (cell - icon) // 2
        canvas.alpha_composite(image, (x, y))
    canvas.save(CLASSIC / "preview.png", optimize=True)


def main() -> None:
    CLASSIC.mkdir(parents=True, exist_ok=True)
    for index in range(29):
        svg = ensure_master(index, ICON_SOURCES[index])
        icon = render_icon(svg)
        icon.save(CLASSIC / f"w{index:02}.png", optimize=True)
        print(f"w{index:02}.png <- {ICON_SOURCES[index]}")
    write_preview()
    print("done")


if __name__ == "__main__":
    main()
