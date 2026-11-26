# Fonts bundled with Rusty Watch

## Inter-Regular.ttf

- Source: [Inter](https://rsms.me/inter/) by Rasmus Andersson.
- Licence: SIL OFL 1.1 — see `OFL-Inter.txt`.
- Available for manual selection in the inspector. It is attached to the project
  and exported into `assets/fonts/font_<hash>.ttf` when selected. New projects
  use the watch system font (Noto Sans) until the user picks a TTF.

## NotoSans-Regular.ttf

- **Watch system font preview only.** It is never attached to a project and never
  exported: when a widget has no font selected, the generated watchface omits the
  `font` property and the watch renders with its own system font, Noto Sans.
- Source: the official Zepp OS font resource
  `https://upload-cdn.zepp.com/20230908/Noto-Sans-Regular.ttf`, linked from
  <https://docs.zepp.com/docs/designs/download/>. Zepp OS documentation names
  Noto Sans as the default system font (Regular and Medium weights).
- Licence: SIL OFL 1.1 — see `OFL-NotoSans.txt`. Copyright 2022 The Noto Project
  Authors (https://github.com/notofonts/latin-greek-cyrillic).
- The upstream file is ~14 MB because it covers CJK. This bundled copy is subset
  to Latin, Latin Extended, General Punctuation, currency symbols, arrows and
  geometric shapes (including all Polish diacritics), which leaves ~72 KB with
  identical glyph metrics and the original family name `Noto Sans-Regular`.
- The editor uses this file for the **System font** choice so the preview matches
  the watch. Custom fonts and Inter keep using their own TTFs.
