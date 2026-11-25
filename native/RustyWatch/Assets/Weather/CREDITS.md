# Weather Icons (OFL 1.1)

- Source: https://github.com/erikflowers/weather-icons (v2.0.10)
- Icon designs: Lukas Bischoff; icon art, HTML/CSS: Erik Flowers.
- Licence: SIL OFL 1.1 (see `OFL-weather-icons.txt`).
- `Source/w00.svg` … `w28.svg` are the upstream SVG masters, renamed after the
  ZeppOS v1.0 weather codes 0..28. They keep the upstream `viewBox="0 0 30 30"`
  and are the crisp, scalable source of truth.
- `Classic/w00.png` … `w28.png` are rendered from those masters at 512×512 RGBA:
  the glyph fills ~80% of the square with ~10% safe padding on every side, so
  nothing is clipped at any widget size. ZeppOS cannot load SVG at runtime, so
  these PNGs are what the editor preview and the exported watchface both use.
  They are committed and bundled with the application; no regeneration tool is
  shipped in the repository.
- Transparent pixels have RGB=(0,0,0) to avoid white rectangles on real firmware.
