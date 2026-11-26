# Bundled weather icon packs

## system-like (default)

- `system-like/w00.png` through `w28.png` are the default icon pack for new weather
  widgets and projects.
- They are 512x512 RGBA vector reconstructions of the 29 Zepp OS system weather
  icons (indices 0..28 of the Huami/Zepp weather table). Every part is drawn from
  simple geometry — circles, ellipses, rounded rectangles, round-capped lines and
  polygons — with one static colour per part, so the set is an original
  reconstruction rather than copied artwork.
- The reconstruction scripts, SVG masters, reference images and comparison sheet
  stay outside this repository; only the bundled PNGs are shipped.
- Names were normalized to `wNN.png`, transparent pixels have RGB=(0,0,0) (the real
  firmware paints white RGB as white rectangles when rotating/scaling an `IMG`),
  and each icon's alpha bounding box is centred on the 512x512 canvas.
- `system-like` is a built-in pack: it is seeded into
  `%APPDATA%\RustyWatch\Weather` on first launch, shown with the built-in badge
  and protected from removal.

## Not Something (bundled)

- `Not Something/w00.png` through `w28.png` are high-resolution (512x512 RGBA)
  circle-based rebuilds of the 29 original ZeppOS weather icons, one vector circle
  per dot, preserving the original dot count, placement and relative size.
- The files were supplied by the project owner from a reconstruction made from the
  original watchface artwork. The rebuild scripts, SVG masters and reference PDF
  stay outside this repository; only the bundled PNGs are shipped.
- Names were normalized to `wNN.png`; transparent pixels have RGB=(0,0,0), and
  every icon's alpha bounding box is centred in the file itself, not by a
  preview-only transform.
- `Not Something` is a built-in, non-removable pack. New widgets use `system-like`;
  users can switch to this pack in the inspector.
