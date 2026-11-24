# Logo

A clean vector rebuild of `logo-how-it-is-supposed-to-look-like.png`: a gold checkmark in an
orange disc, wrapped by a broken dark-orange ring and a three-tooth gear.

## Which file to use

| Use case | File |
| --- | --- |
| Websites, inline HTML/CSS, docs | `logo.svg` (infinitely scalable, ~2 KB) |
| Website favicon | `favicon.ico` (16/32/48/64 px) or `logo-32.png` |
| GitHub profile / social avatar | `github-avatar.png` (400×400) or `logo-512.png` |
| iOS / Android app icon source | `logo-1024.png` |
| Android launcher / general PNG | `logo-512.png` |
| Smaller UI spots | `logo-256.png`, `logo-128.png`, `logo-64.png` |
| On a custom / light / dark background | `logo-mark.svg` or `logo-transparent-*.png` (no dark backdrop) |

The SVGs are the master files — edit those and re-export PNGs if you change the design.

## Colors

- Background: `#3B4146`
- Ring: `#783215`
- Disc + gear: `#BC5F28`
- Checkmark: `#DE9641`

## Notes

- The master artboard is 1024×1024 with a rounded-square background (corner radius 224), so it
  crops cleanly to a circle for profile avatars.
- `logo-mark.svg` is the same mark with a transparent background, for use where the dark tile
  isn't wanted (e.g. on a colored header).
- At 32 px and below the gear detail gets tight; `logo-64.png` is the smallest size where the
  design stays fully readable.
