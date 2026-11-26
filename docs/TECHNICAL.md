# Rusty Watch — technical reference

This document explains **how the code actually works**, module by module, from an editor
click to a watchface running on an Amazfit Active 2 (Round). It is a companion to
[`../README.md`](../README.md) (what the product does) and [`../AGENTS.md`](../AGENTS.md)
(rules for AI agents).

**How to read it.** Start with §1 for the pipeline. Sections §2–§8 describe the pure Rust
core: data, project lifecycle, layers, widgets, FFI, asset stores and the time/date engine.
§9–§10 follow the generated JavaScript, the ZIP, the previews and the QR install path.
§11 covers the WinUI front-end. §12 collects invariants and the traps that were learned the
hard way; §13 is a file index. Symbols and file names are authoritative — line numbers drift,
so they are deliberately omitted.

---

## Contents

1. [Big picture and end-to-end flow](#1-big-picture-and-end-to-end-flow)
2. [Data model (`model.rs`)](#2-data-model)
3. [Project lifecycle and validation](#3-project-lifecycle-and-validation)
4. [Layers: the only draw order](#4-layers-the-only-draw-order)
5. [Widgets, catalog and frame geometry](#5-widgets-catalog-and-frame-geometry)
6. [The FFI bridge](#6-the-ffi-bridge)
7. [Asset stores and scaling](#7-asset-stores-and-scaling)
8. [Time, date and language engine](#8-time-date-and-language-engine)
9. [Code generation](#9-code-generation)
10. [Packaging, preview files and QR install](#10-packaging-preview-files-and-qr-install)
11. [WinUI 3 front-end](#11-winui-3-front-end)
12. [Invariants and learned traps](#12-invariants-and-learned-traps)
13. [File index](#13-file-index)

---

## 1. Big picture and end-to-end flow

```text
 WinUI 3 UI (C#/XAML, native/RustyWatch)
        │  user edits / drags / picks assets
        ▼
 RustNative.cs  ── P/Invoke (cdecl, UTF-8 JSON) ──►  rusty_ffi.dll
                                                       │
                                                       ▼
                                              rusty-core (pure Rust)
                                    model · layers · widgets · formats · assets
                                                       │
                        ┌──────────────────────────────┴───────────────────┐
                        ▼                                                  ▼
                 codegen.rs                                          preview.rs
                 app.json / app.js / index.js                    preview.png / preview.gif
                        │                                                  │
                        ▼                                                  │
                 package.rs ── flat v2 ZIP ──► phone / simulator           │
                 qr.rs ── zpkd1:// URL ──► local TcpListener                │
                        │                                                  ▼
                        ▼                                          export dialog (GIF)
                 watch: hmUI + hmSensor + timer
```

The split has one purpose: **the editor preview and the exported watchface use the same Rust
modules**. `layers::ordered_widgets()` defines draw order for both; `time_format` and
`date_format` build the strings in the editor and emit the equivalent JavaScript for the
watch; `preview.rs` renders the generated preview frames from the same project data. C# only
calls `rw_*`, parses the JSON response and updates controls.

A typical edit:

1. The inspector sends `rw_set_widget(projectFile, settingsJson, appDataRoot)`.
2. Rust deserializes the JSON, validates geometry and settings, updates the `Widget`,
   attaches/prunes assets and saves `project.json`.
3. The response carries fresh layer descriptors; C# updates the Layers list and preview.

A typical export:

1. C# asks Rust for a random series and renders eight frames into `preview-cache/`.
2. Rust composes the branded `preview.png` from frame 7, turns it into GIF frame 1, draws
   shortcut hatches on frame 2 and deletes the cache.
3. The dialog opens. C# exports the ZIP, starts a local server and generates a QR image
   (all before the dialog is shown); the QR button only reveals it.

---

## 2. Data model

### 2.1 Top-level constants and device profile

`SCHEMA_VERSION = 1`; `SCREEN_SIZE = 466`. `DeviceId` has one variant,
`Active2Round` (`"active-2-round"`), whose `DeviceProfile` is:

| Field | Value |
|---|---|
| `target_key` | `"active2round"` |
| `device_source` | `8_913_153` (eight compatible sources are emitted in the manifest) |
| `design_width` | `466` |
| `api_compatible` / `api_target` / `api_min` | `3.0.0` / `3.6.0` / `3.0.0` |
| `screen_shape` | `Round` |

The profile API fields are *not* the manifest's `runtime.apiVersion` — that one is
`1.0.0 / 1.0.1` because the generated package uses classic watchface JS, not a `.bin`.

### 2.2 `Project` and `Metadata`

`Project` (camelCase JSON) carries: `schemaVersion`, `metadata`, `device`, `widgets`,
`assets` (default empty), `customCode` (optional), `layerOrder` (bottom → top, widget ids
only), `measurementSystem` (default `Watch`) and `firstDayOfWeek` (default `Monday`).

`Metadata` has `name`, `appId`, `author`, `version`, `description`. The name is immutable
after creation because the folder and recent list depend on it; Save as is the way to rename.

`Asset` is `{ path, role }`. Roles are `icon`, `font`, `image`, `hand`, `status_icon`,
`weather_icon`, `other`. There is deliberately **no** background role.

### 2.3 `Widget`

`Widget` (camelCase) is one layer. Identity and geometry: `id`, `name`, `kind`, `x`, `y`,
`w`, `h`, `z`. Optional/defaulted fields include:

| Field | Default | Meaning |
|---|---|---|
| `text`, `textSize`, `font`, `timeFormat`, `color` | various | Text appearance; `timeFormat` doubles as the date pattern for `DateText`. |
| `src`, `imageFit` | `null`, `contain` | Image/STATUS/weather asset and fit mode. |
| `handSet`, `hourSrc`, `minuteSrc`, `secondSrc`, `capSrc` | `null` | Analog hands; `null` means hidden. |
| `dataSource` | `null` | Which sensor/tick updates the text. |
| `align`, `vAlign` | `center`, `center` | Horizontal / vertical text alignment. |
| `showLevel` | `normal` | Kept for compatibility; codegen always emits `ONLY_NORMAL`. |
| `autoScale` | `false` | Red frame / scale mode. |
| `textStyle` | `ellipsis` | `ellipsis` / `none` / `wrap` (watch has no wrap; maps to `NONE`). |
| `visible`, `alpha` | `true`, `255` | Eye button and opacity. |
| `measurementSystem`, `showUnit` | `null`, `true` | Distance override and unit suffix. |
| `statusIconSet`, `statusType` | `null` | STATUS pack and which of the four icons. |
| `weatherIconSet`, `weatherDayOffset` | `null`, `0` | Weather pack and forecast day 0…4. |
| `sleepStageMode` | `duration` | Deep/light/REM/wake as minutes or percent. |
| `progress` | `null` | `ProgressSettings` for arc/bar widgets. |
| `shortcutTarget` | `null` | `hmUI.data_type` target of a tap zone. |

`WidgetKind::uses_pattern()` is true only for `TimeText` and `DateText`; it is the single
source of truth for showing the format box and for ignoring `timeFormat` on other kinds.
`WidgetKind` also groups helpers: `is_date_field()`, `is_health()`, `is_sleep()`, `is_goal()`,
`is_weather()`, plus initial-text builders (`health_initial_text`, `goal_initial_text`,
`sleep_stage_initial_text`).

### 2.4 Supporting enums

- `MeasurementSystem`: `Watch` (default), `Metric`, `Imperial`. `Watch` calls
  `hmSetting.getMileageUnit()` at build time; metric divides metres by 1000, imperial by
  1609.344. A widget may override the project default; `null` means "follow project".
- `FirstDayOfWeek`: `Monday` (default) or `Sunday`. Weekday **number** fields are computed
  from `year/month/day` with Sakamoto's algorithm — never from `TIME.week`.
- `ProgressSource`: `battery`, `steps`, `distance` (legacy only), `heart`, `calories`,
  `stand`, `fat-burning`, `pai`, `spo2`, `sleep`, `fixed`.
- `ProgressCorner`: `round` → `corner_flag 0`, `start` → `1`, `end` → `2`, `flat` → `3`.
- `StatusType`: `alarm`/`dnd`/`bluetooth`/`lock`, mapping to
  `hmUI.system_status.CLOCK/DISTURB/DISCONNECT/LOCK` with bits `1/2/4/8`.
- `ShortcutTarget`: 19 values (`battery`, `steps`, `heart`, `stress`, `sleep`, `pai`, `spo2`,
  `weather`, `alarm-clock`, `stopwatch`, `countdown`, four running/cycling/swimming variants,
  `free-training`, `breath-train`, `moon`), each with its `hmUI.data_type` constant.
- `Align` / `VAlign` / `TextStyle` / `ShowLevel` have `as_js()` methods that prefix the
  corresponding `hmUI.*` constant.

Two geometry helpers matter later:

```rust
arc_center_radius(w, h) = max(min(w, h) / 2 - 10, 4)
arc_draw_margin(thickness) = max(thickness / 2 - 8, 0) + 6
```

The second one keeps a thick arc from being clipped in generated PNG/GIF frames.

`ProgressSettings` defaults: source `battery`, range `0…100`, value `50`, colour
`0xFF4FC3F7`, track `0xFF3A3A42`, thickness `16`, angles `0…360`, corner `round`, radius `8`,
orientation `horizontal`, track visible. Older projects may still carry `segments`/`gap`;
they export as one continuous arc.

### 2.5 Starter template

`Project::starter(name)` creates version `0.1.0`, an `icon.png` asset, and one time widget
`x=23, y=183, w=420, h=100`, text `00:00:00`, size `64`, pattern `HH:MM:SS`, white,
`DataSource::Time`, centred, `ShowLevel::Normal`, `layer_order = ["time"]`.

---

## 3. Project lifecycle and validation

`project.rs` owns on-disk layout: `<projects root>/<sanitized name>/project.json`.
`create_project_with` refuses to overwrite, creates `assets/`, generates a small round
`icon.png`, writes the starter project. `watchface_folder_name` replaces control characters
and Windows-forbidden characters with `_`, strips trailing dots/spaces, and prefixes reserved
names (`CON`, `NUL`, `COM1`…). `random_app_id()` is an xorshift32 seeded from time + PID,
mapped to an 8-digit number.

`load_project` requires `schemaVersion == 1`. `save_project` writes pretty JSON with a
trailing newline. `save_project_as` creates a new folder, changes name/author/App ID,
regenerates the description and copies every registered asset from the old project.
`update_project_metadata` changes only author and version (version: non-empty, ≤32 chars,
no whitespace).

`validate::validate(&Project) -> Vec<String>` is called before save and export. It checks:

- project: non-empty name and author, App ID ≠ 0, device `Active2Round`, ≥1 widget;
- widget: non-empty name, unique id, geometry inside 466×466 with positive size;
- text kinds: non-zero `textSize`, parseable pattern (time or date depending on kind),
  colour of form `0xAARRGGBB`, font (if set) attached as an asset;
- progress: settings present, segments 1…24, thickness 1…233, valid colours;
- shortcut: target present;
- STATUS: pack + type present, `src` in assets;
- weather icon / image: `src` in assets;
- asset paths: non-empty, no `..`, not absolute.

Diagnostics are Polish; the UI translates known patterns through `LocalizationService.Error`.

`recent.rs` stores at most ten `{name, file, openedAt}` entries in `recent.json`. Loading is
BOM-tolerant and accepts the canonical array, PowerShell's `{"value":[…],"Count":N}` shape
and a single object — so a broken recent store never prevents opening a project. Saving
always rewrites a canonical array.

---

## 4. Layers: the only draw order

`layers.rs` treats `Project::layerOrder` as the authoritative bottom → top order.

- `order()` keeps only entries whose ids still exist, removes duplicates, then appends the
  remaining widgets sorted stably by `z`.
- `apply_order()` writes that order back and sets each widget's `z = index`.
- `move_layer()` normalizes first, then moves ±1, refusing to move past the end.
- `remove_widget()` refuses to remove the last remaining widget.
- `ordered_widgets()` returns widgets in draw order **excluding hidden ones**; codegen and
  the Rust frame renderer both use it.
- `describe()` returns `LayerView` descriptors for the editor (geometry, text/font/align,
  image fit, hand sources, status/weather selection, progress settings, shortcut target).
- `widget_kind()` maps a widget to its catalog kind string (`analog-time`, `arc-progress`,
  `status-alarm`, `weekday-number`, …), which the UI uses to pick the inspector.

There is **no background layer and no background asset**. Codegen emits a constant
`FILL_RECT(0, 0, 466, 466, 0xFF000000)` before all widgets, and the preview fills its canvas
black before drawing layers.

---

## 5. Widgets, catalog and frame geometry

`widgets::supported_widgets()` produces 58 `{kind, category, hasPattern}` entries. The
categories are `time`, `date`, `text`, `images`, `distance`, `goals`, `health`, `sleep`,
`system`, `weather`, `progress`, `shortcut`. `hasPattern` is true only for `time` and `date`.

`add_widget(project, kind)` picks a spec table, creates a unique id (`time`, `time-2`, …),
adds the widget on top via `layers::normalize()` and gives it sane defaults. Examples:

| Kind | Initial frame | Initial text / pattern |
|---|---|---|
| time | 23,183,420,100 | `00:00:00`, `HH:MM:SS`, size 64 |
| analog-time | 33,33,400,400 | geometry replaced by the hand set's natural size |
| date | 23,250,420,80 | `Jan 1`, pattern `MMM D`, size 48 |
| text | 23,350,420,60 | `Text`, no data source or pattern |
| images | 133,133,200,200 | `FitMode::Contain` |
| battery | 23,203,420,60 | `100%` |
| status icons | x=145/193/241/289, y=24, 32×32 | one centred row |
| weather icon | 201,201,64,64 | centred 64×64 |
| arc / bar | 33,33,400,400 / 23,203,420,60 | full circle by default |
| shortcut | 173,203,120,60 | target `battery`, no PNG |

`update_widget(project, json)` is the inspector's write path. It requires `id`, `x`, `y`,
`w`, `h`, enforces screen bounds, and then branches by kind: text widgets additionally
require `textSize` (8…160) and a valid `0xAARRGGBB` colour; progress clamps its numeric
fields and colours; analog/images/STATUS/weather only update geometry, `autoScale` and
`alpha`; images force `FitMode::Stretch` while `autoScale` is true. `timeFormat` is applied
only when `uses_pattern()`. A `deferAssets: true` flag skips expensive image/hand/status/
weather rescaling so it can run later on a background thread; fonts are still handled.

`frame_geometry` is pure math used on every pointer move. Constants: `MIN_FRAME_SIZE = 24`,
text size 8…160, `SNAP_GRID = 16`, `ALIGN_THRESHOLD = 5`. `FrameInput` carries the current
frame, the dragged handle (`move`, `nw`, `ne`, `sw`, `se`), `dx`/`dy`, `autoScale`, `snap`,
`alignX`/`alignY` candidates, `square`, `minSize` and `maxSize` (default 466, 256 for the
weather icon). Rules:

- `move` translates, then resolves alignment on each axis, then optional grid snap;
- alignment wins over the grid and is checked on the **pre-grid** position (a 420 px widget
  could otherwise never centre on x = 233 with the grid on);
- `square` handles analog resizing with one uniform size and keeps `w == h`;
- `autoScale` recomputes `textSize = old * newW / startW`, clamped to 8…160;
- `guideX`/`guideY` are returned for the editor to draw dashed red guides.

The function never saves anything; C# applies the returned numbers verbatim.

---

## 6. The FFI bridge

### 6.1 Contract

`rusty-ffi` is a `cdylib` producing `rusty_ffi.dll`. There are **66** `#[no_mangle]
pub extern "C"` exports: 65 return a Rust-allocated, NUL-terminated UTF-8 JSON string;
`rw_free_string` returns `void` and releases that pointer. Every result has the envelope
`{"ok":true,"data":…}` or `{"ok":false,"error":"…"}`. Input strings are NUL-terminated UTF-8;
Rust reads them lossily and tolerates null pointers as empty strings. Exceptions never cross
the boundary — each body is wrapped in a `Result`.

C# declares each function in `RustNative.cs` with `[DllImport(Library, CallingConvention =
CallingConvention.Cdecl, EntryPoint = "rw_…")]`, `LPUTF8Str` on every string and `I1` on the
one `bool`. A central `Call(Func<IntPtr>, out string? error)` helper decodes UTF-8, frees the
pointer in a `finally`, and returns the raw `data` JSON. Wrappers inject `%APPDATA%` and
bundled asset directories so callers do not build paths themselves.

### 6.2 Export catalog

| Group | Functions |
|---|---|
| Project | `rw_create_project`, `rw_create_project_in`, `rw_save_project_as`, `rw_load_project`, `rw_save_project`, `rw_set_project_metadata`, `rw_validate_project`, `rw_open_project` |
| Misc | `rw_free_string`, `rw_version`, `rw_random_app_id`, `rw_recent_list/add/remove/clear` |
| Widgets & layers | `rw_editor_layers`, `rw_move_layer`, `rw_supported_widgets`, `rw_frame_geometry`, `rw_add_widget`, `rw_set_widget`, `rw_rescale_widget_assets`, `rw_set_widget_visibility`, `rw_remove_widget`, `rw_set_measurement_system`, `rw_set_first_day_of_week`, `rw_time_widget`, `rw_set_time_widget` |
| Export | `rw_export_zip`, `rw_prepare_emulator_workspace`, `rw_inspect_zip` |
| Fonts | `rw_ensure_defaults`, `rw_fonts_list/add/remove`, `rw_apply_default_font`, `rw_apply_default_font_to_missing` |
| Images | `rw_images_list/add/remove`, `rw_set_widget_image` |
| Hands | `rw_hands_list/add/remove`, `rw_set_hands`, `rw_set_hand_visibility` |
| STATUS | `rw_status_packs_list/add/remove`, `rw_set_status_pack`, `rw_attach_status_pack_to_missing` |
| Weather | `rw_weather_packs_list/add/remove`, `rw_set_weather_pack`, `rw_weather_conditions` |
| Formatting | `rw_preview_default_datetime`, `rw_format_time`, `rw_format_date`, `rw_format_date_field`, `rw_format_weather_field`, `rw_month_name` |
| Preview | `rw_preview_random_series`, `rw_render_preview_frame`, `rw_encode_preview_gif` |
| QR | `rw_generate_qr` |

`rw_inspect_zip` exists in Rust but has no C# declaration — it is used from the CLI path only.
`rw_encode_preview_gif` deliberately ignores its `series_json` argument: the series was used
earlier to render the eight cached PNGs, and the GIF is assembled from those files.

The FFI layer is stateless, which makes `rw_rescale_widget_assets` safe to call from a C#
background thread while the UI stays responsive.

---

## 7. Asset stores and scaling

### 7.1 Shared idea

Installed assets live in `%APPDATA%\RustyWatch\<Store>`; the app bundles first-run copies in
`native/RustyWatch/Assets/`. Choosing an asset in the editor copies it into the project
(`<project>/assets/...`), registers an `Asset { path, role }` and prunes no-longer-used copies.
A project is therefore self-contained: Save as, ZIP export and moving the project folder never
depend on the shared store.

`asset_cache.rs` stores `preview.fingerprint` (FNV-1a 64-bit over name + length + mtime of
source files) and `.bundled-version` next to each generated thumbnail. Thumbnails regenerate
only when missing or changed, and an updated bundled pack reaches existing installations when
its version marker differs. This keeps the Assets page fast and non-blocking.

`export_paths.rs` maps a project asset path to its ZIP name with 32-bit FNV-1a:
`icon.png` stays; `fonts/…` becomes `fonts/font_<hash>.ttf`; `hands/`, `status/`, `images/`
and `weather/` become `hand_<hash>.png`, `status_<hash>.png`, `img_<hash>.png`,
`weather_<hash>.png`. Codegen emits exactly those names, so JS and ZIP cannot disagree.

### 7.2 Fonts

`fonts::ensure_default` seeds `Inter-Regular.ttf` (SIL OFL 1.1). `list()` parses the TTF
`name` table directly (no external font library) to show the family name; `add()` accepts
only a parseable `.ttf` and never overwrites the default. `Widget.font` is just a file name.
`apply_default_font` is still available in the core/FFI, but the UI no longer calls it:
**new projects and newly added widgets keep `font = None`**. A widget with `font = None`
uses the watch's system font: Zepp OS documents **Noto Sans** as that font.
Codegen then omits `font:` entirely. In the inspector, choosing **System font** sends
`font: ""`, which `widgets::update_widget` deliberately maps to `None`; omitting the key keeps
the previous choice. The editor no longer auto-fills null fonts on project open or after
adding a widget, so an explicit system-font selection survives reopening.

The editor cannot load fonts from `%APPDATA%` in an unpackaged WinUI app. `PreviewFonts.cs`
copies the selected TTF to `<exe>/FontCache` and exposes it as
`ms-appx:///FontCache/<file>#<family>`, so preview metrics match the exported font. For the
system-font choice it uses a bundled, Latin/PL subset of the official
`NotoSans-Regular.ttf` from Zepp's resource page (SIL OFL 1.1, see `Assets/Fonts/CREDITS.md`)
instead of Windows' Segoe UI. `preview.rs` embeds the same TTF, so the generated preview
frames match both the editor and the watch.

### 7.3 Images and fit modes

`image_fit::FitMode` is `cover` / `contain` / `stretch` (default `contain`); `fit_image`
uses Lanczos3 when downscaling and CatmullRom when upscaling. `set_widget_image` fits the
picture to its natural size with one uniform scale clamped to 466×466, writes the prepared
PNG into `<project>/assets/images/<widget-id>/<file>` and starts in blue move-only mode.
Double-clicking switches to the red frame: corner handles appear and `rw_set_widget` forces
`stretch`, re-preparing the PNG from the pristine store copy. Export applies
`export_asset_bytes` again as a safety net for older projects.

### 7.4 Hands

A valid set in `Hands/<id>/` has square, equally sized `hour.png` and `minute.png`, with
optional `second.png` and `cap.png`. Bundled sets are **Classic** (with a subtle shadow) and
**Light**; `ensure_default` seeds *every* bundled set and skips already-installed ones.

Two problems shape this module:

1. `hmUI.widget.IMG` does not scale an image at runtime. The PNGs copied into the project
   are physically resized to the widget frame by `scale_project_hands`.
2. Bundled Classic had white RGB under `A = 0`; real firmware renders those pixels as white
   rectangles when rotating an `IMG`, while the simulator hides them. `scale_hand` therefore
   zeroes RGB wherever alpha is zero and resamples in **premultiplied alpha** space, keeping
   premultiplied RGB in the saved PNG.

`prepare_hand_assets` (called by both export paths) scales the whole set by one common
factor, finds a tight alpha bounding box per hand, pads it by 8 px, and emits `x/y/w/h`
placements. That reduces four decoded textures from ~1.4 MB to ~150 KB; a full-screen
466×466 hand texture costs roughly 868 KB and several of them can reboot the watch.

A new analog widget is centred and sized from the PNGs (Classic: 400×400 at 33,33). Changing
the set on an existing widget keeps its frame and re-scales the new PNGs. Per-hand checkboxes
call `rw_set_hand_visibility`: hiding clears the corresponding `*_src`, revealing re-copies
from the store; disabled files are pruned from the project. The generated JS emits `null`
for hidden hands, and validation accepts any subset.

### 7.5 STATUS icon packs

A pack in `Status/<id>/` must contain all four square, equally sized files: `alarm.png`,
`dnd.png`, `bluetooth.png`, `lock.png`. Only **Classic** is bundled and it cannot be removed.
The exported widgets are `hmUI.widget.IMG_STATUS` with `hmUI.system_status.*`; ZeppOS toggles
their visibility itself, so no sensors, timers or listeners are generated. Pack selection
copies and physically resizes the icons to the frame, registers `AssetRole::StatusIcon` and
prunes unused packs. Thumbnails are a 2×2 grid generated in Rust with a fingerprint.

### 7.6 Weather icons

A pack in `Weather/<id>/` contains 29 square PNGs `w00.png`…`w28.png`, one per ZeppOS
condition code. Bundled packs are **`system-like`** (the default; 512×512 ZeppOS-style vector
reconstructions) and **Not Something** (512×512 circle-based rebuilds of the original icons);
both are centred in the file itself, PNGs only, with provenance in
`Assets/Weather/CREDITS.md`. Both are built-in and protected from removal. `resize_png`
resamples in premultiplied alpha space and **keeps premultiplied RGB**;
unpremultiplying restored white RGB at partial alpha, which the firmware painted as jagged
white edges on the real watch. `describe_pack` requires all 29 correctly named, square,
equal-size files, so an incomplete pack never appears in `list()` and cannot be selected;
`rw_weather_packs_list` calls `ensure_default` → `ensure_previews`, which generates a missing
`preview.png` for hand-installed valid packs. `rw_format_weather_field` has no fallback to
another pack: a missing icon yields an empty `iconPath`, and the editor clears the picture
instead of showing a different template under the selected name.

The icon inspector has a 24…256 px SIZE slider. It sets a square `w = h`, moves `x/y` by half
the size delta so the centre stays put (clamped to 466×466), saves geometry and triggers the
debounced `rw_rescale_widget_assets`, which rescales all 29 icons of the selected pack from the
pristine store copy. Drag-resize of the frame is also `square` for `weather-icon`, and while
the corner is being dragged the slider value and its px label follow the frame live (the
slider's own `ValueChanged` is suppressed during that sync so no extra save is triggered).

Switching packs is deliberately split: the UI decodes only the currently visible icon, while
the 29-file copy/resize runs in `rw_set_weather_pack` on a background queue with a shared
gate, cancellation and a version counter. `attach_to_project` already scales all icons, so no
second scaling pass is added. Generated JS caches the last condition index and only sets
`SRC` when it changes — otherwise a per-second tick smudged the soft edges.

### 7.7 Current weather vs. daily forecast

`forecastData.data[i].index` is the **daily** forecast code and does not track the current
condition or time of day — that is why a cloudy daily code (`0`) appeared at night while the
ZeppOS weather widget correctly showed overcast (`4`). Today (`weatherDayOffset = 0`) therefore
uses a different source: `_currentWeatherCode()` reads the `curAirIconIndex` property of the
`hmSensor.id.WEATHER` instance — the same current-condition index that other watchfaces use for
their weather icon (the "Air" in the name refers to atmospheric conditions, not AQI). An earlier
attempt to read a data-bound `IMG_LEVEL`'s `level` via `getProperty()` proved unreliable: other
watchfaces let the firmware draw from that widget and never read it back, so our readback fell
through to the forecast fallback and showed `w26` at night. Night codes `26/27/28` are now
derived locally in `_wxNightCode()` from TIME and `tideData`. The visible icon remains a plain
`IMG` with manual `SRC`, preserving geometry, pack selection and the `_lastWeatherIcon` smudge
cache; days 1…4 keep reading `forecastData.data[i].index`. All codes pass through
`_weatherIndex()` (invalid → `25`/"Unknown") — the old `| 0` coercion turned `null` into `0`
(`w00`/"Cloudy"). When `curAirIconIndex` is unavailable (notably in the simulator),
`_fallbackTodayCode()` uses today's forecast index with the same night mapping.
High/low and sunrise/sunset stay whole-day forecast values for every day, including day 0.

---

## 8. Time, date and language engine

### 8.1 Time patterns

`time_format.rs` parses patterns up to 32 characters. Tokens: `HH`/`H` (24-hour), `hh`/`h`
(12-hour), `MM`/`M` and `mm`/`m` (minutes), `SS`/`S` and `ss`/`s` (seconds), `A`/`a`
(AM/PM). Doubling a letter means zero padding; `\` escapes the next character; anything else
is a literal. `has_seconds()` decides whether the generated watchface uses a 1-second timer
or only a `MINUTEEND` listener.

There are two JS builders: `js_expression()` with literal AM/PM and
`js_expression_localized()` reading `L.am`/`L.pm` from the runtime language tables. The editor
formatting call `rw_format_time` uses the same parser, so preview and export agree.

### 8.2 Date patterns

`date_format.rs` uses the same pattern field (`Widget::timeFormat`) but its own token set:
`D`/`DD` day, `M`/`MM` number, `MMM`/`MMMM` short/full month name,
`DDD`/`DDDD` short/full weekday, `YY`/`YYYY` year, `\` escapes. Letter case controls name
case: uppercase → `Jan`/`Mon`, lowercase → `jan`/`mon`, five or six repeats → UPPERCASE.
Month input is 0-based and weekday index is 0 = Sunday, matching the generated JS.

The module also owns `weekday_number(day, month, year, first_day)`, `day_of_year` and
`iso_week`. Weekday numbering is computed with Sakamoto's algorithm from the date fields —
`TIME.week` is deliberately not used because its first day differs between API versions.

Crucially, generated JS must never call `Date` (the real watch runtime does not expose it).
The module therefore emits helper constants `JS_WEEKDAY_INDEX`, `JS_DATE_FROM_T`,
`JS_ISO_WEEK`, `JS_DAY_OF_YEAR` and `js_weekday_number(first_day)`, which compute everything
from the TIME sensor's `{day, month, year}` fields. A regression test bans `Date`,
`getDate`, `getMonth`, `getDay`, `getFullYear` and UTC methods in generated JS.

### 8.3 Locale

`locale.rs` holds the watchface language tables: English and Polish month/weekday names,
AM/PM markers and 29 weather condition names. Language id `9` means Polish; **every other
value is English**. `js_detection()` emits a guarded probe of
`DeviceRuntimeCore.HmUtils.getLanguage()` with an `hmSetting.getLanguage()` fallback, defines
`IS_POLISH`, builds the `L` tables (including Polish diacritics directly in the JS) and a
`_cityName()` helper. One ZIP therefore works in both languages.

The editor's *UI* language and its *watchface preview* language are separate settings. The
preview language is passed to `rw_format_time`, `rw_month_name` and the generated frames; the
exported watchface always detects the watch language itself.

---

## 9. Code generation

### 9.1 `app.json`

`generate_app_json` emits a **flat v2** manifest: `configVersion: "v2"`, no `targets`.
Notable keys:

- `app`: `appId`, `appName`, `appType: "watchface"`, `version.code/name`, `vender: "zepp"`
  (intentional spelling for reference parity), empty `description`, `icon`/`cover`
  `Preview.png`, `extraInfo`;
- `runtime.apiVersion`: compatible `1.0.0`, target `1.0.1`, min `1.0.0` — and **no**
  `runtime.type`;
- `i18n`: only `en-US`; `defaultLanguage: "en-US"`;
- `module.watchface`: path `watchface/index`, `main: 1`, `lockscreen: 0`;
- `designWidth: 466`;
- `platforms`: eight `Amazfit Active 2` entries with device sources
  `8913152, 8913153, 8913155, 8913159, 10092800, 10092801, 10092803, 10092807`;
- `packageInfo`: `mode: "production"`, current timestamp, `expiredTime: 172800`, `zpm: "2.8.2"`.

`app.js` is the verbatim official `zeus` template (CRLF), with `gettextFactory`,
`getPx(454)`, `getApp()` / `getCurrentPage()`. Only the emulator workspace converts this
manifest to v3 with `targets`.

### 9.2 `watchface/index.js`

`generate_index_js_with_hands(project, placements)` walks `layers::ordered_widgets()`.
The file is a single IIFE with a prelude of helpers, then
`DeviceRuntimeCore.WatchFace({ init_view, onDestroy, onInit, build })`.

**Prelude.** Runtime language detection; `p2()` padding; `_screenIsAod()`; date helpers;
optional mileage-unit detection; lazy sensor getters, each wrapped in `try/catch` so a missing
sensor cannot kill the watchface; weather arrays; sleep and heart statistics helpers;
progress value/apply functions; world-clock getter. `project.custom_code`, if any, is
appended verbatim as an unowned block.

**Background.** `init_view()` begins with
`hmUI.widget.FILL_RECT(0, 0, 466, 466, 0xFF000000)` — always black, always first.

**Widget emitters.**

| Widget | Generated form |
|---|---|
| Text family | `hmUI.widget.TEXT` with `text`, `text_size`, `char_space: 0`, `line_space: 0`, hashed `font`, raw ARGB colour, `align_h`/`align_v`, `text_style`, optional `alpha`. |
| Images / weather icon | `hmUI.widget.IMG` with a hashed `src`; PNGs are pre-sized to the frame. |
| Analog time | Up to four `hmUI.widget.IMG` widgets, each full-screen with `pos_x/pos_y`, `center_x/y = 233`, `angle` updated by `setProperty(hmUI.prop.MORE, …)`; cap is static. |
| STATUS | `hmUI.widget.IMG_STATUS` with the pack's hashed PNG and `hmUI.system_status.*` type; no listeners. |
| Shortcut | `hmUI.widget.IMG_CLICK`, **`src: ""`**, `type: hmUI.data_type.<target>`; no PNG in the ZIP, no listeners. |
| Progress arc | `hmUI.widget.ARC_PROGRESS` with centre, radius, angles, `line_width`, colour and `corner_flag`; optional track. |
| Progress bar | `hmUI.widget.FILL_RECT` with an optional corner radius; fill starts at width 0. |
| `weather-current` | `hmUI.widget.TEXT_FONT` with `type: hmUI.data_type.WEATHER_CURRENT` and `unit_type`; the firmware supplies the value — no sensor and no static `text:`. |

All dimensions in update expressions are **frame literals**, never bare `x`/`y`/`w`/`h`
identifiers (a bare identifier caused a `ReferenceError` and left bars unfilled). Every
widget emits `hmUI.show_level.ONLY_NORMAL`; AOD is handled by `WIDGET_DELEGATE` and
`hmSetting.getScreenType()`.

**Tick wiring.** The main `tick()` reads the TIME sensor and updates time/date/analog lines,
then calls battery, health, progress, sleep, goal and weather ticks. `needs_seconds` is true
when a time pattern has seconds or an analog widget has a second hand: then normal mode uses
`timer.createTimer(0, 1000, tick)`; otherwise (and in AOD) it listens for `TIME.MINUTEEND`.
`onDestroy` stops the timer, removes every listener and uninitialises the world clock.

Sensor details worth remembering:

- Heart-rate text and progress read `heart.last` on the `LAST` event only — no
  `CURRENT` continuous measurement and no polling from `tick`. Min/max/average are derived
  from `HEART.today` once per minute, ignoring non-positive samples; progress uses a fixed
  165 BPM scale.
- Goal sources (steps, calories, stand, fat-burning) divide by the watch's own `.target`.
- Sleep calls `updateInfo()` before `getTotalTime()`.
- Forecast fields emit a literal day index `0…4`; missing days produce `--`/`--°`/`--:--`.
- Weather icons update `SRC` only when the condition index actually changes.

---

## 10. Packaging, preview files and QR install

### 10.1 ZIP export

`package.rs::export_zip` validates the project, ensures unique hashed asset names, prepares
hand assets, and writes a flat v2 archive:

```text
app.json
app.js
watchface/index.js
assets/Preview.png
assets/fonts/font_<hash>.ttf
assets/hand_<hash>.png, status_<hash>.png, img_<hash>.png, weather_<hash>.png
```

Explicit directory entries (`assets/`, `assets/fonts/`, `watchface/`) are stored with DOS
metadata `0x41ff0010`, matching the reference package; files are deflated. Editor
subfolders such as `hands/<set>/` never appear in the ZIP. `export_zip_bytes` performs the
same in memory.

`assets/Preview.png` is the branded preview rescaled to **314 × 314** RGBA and masked to the
round screen shape (transparent outside, RGB zeroed there). If no preview exists, Rust
deterministically renders the 21:37 frame and composes it. `preview.gif` and the full-size
`preview.png` stay next to `project.json` and are not packaged.

### 10.2 Generated preview frames

`preview.rs` renders everything in Rust — no offscreen WinUI controls, which previously left
artifacts under the editor. Text uses the exact TTF of the widget (or the bundled Noto Sans for
`font = None`), with `ab_glyph`'s `PxScale` converted from `text_size` **em** units — its raw
value is the font's cell height, which made the preview text ~30% smaller than the editor.
The data is deterministic unless explicitly randomised for the eight export frames:

- `FRAME_COUNT = 8`, `SOURCE_FRAME_COUNT = 7`, `PREVIEW_FRAME_INDEX = 7`,
  `GIF_FRAME_DELAY = 170` (≈1.7 s), `SCREEN_RADIUS = 233`.
- Preset times: 06:05:04, 09:11:10, 12:17:16, 15:23:22, 18:29:28, 20:33:34, 21:37:40,
  then an exact copy of 21:37:40.
- Seven real dates (one per source frame): 2022-01-03 … 2022-07-31, Monday…Sunday in
  January…July.
- Status-icon mask bits `ALARM=1, DND=2, BLUETOOTH=4, LOCK=8`, sequence
  `2,2,0,3,0,1,4,4`: two empty frames; frame 7 shows all four; frame 8 copies it.
- Steps are derived from distance (`1300…1400` steps/km); battery falls 100% → 10%; heart
  rate is forced to 56 in frame 1 and 120 in the last frames; goals are fixed.
- `random_series()` fills the remaining sensor fields with realistic bounded values and
  duplicates entry 7 into entry 8 for every array.
- Custom Data from the editor is deliberately ignored by generated preview files.

`render_frame()` fills a black 466×466 canvas, walks `layers::describe()` in draw order and
draws text with `ab_glyph`, images, rotated hands, STATUS icons, weather icons and progress.
Outside radius 233 the frame is opaque black. `arc_coverage()` shares the editor's angle
convention (`atan2(-dy)`), honours reversed sweeps, adds round end caps only for partial
arcs, and `arc_draw_margin()` widens the render window for thick arcs. Bars use a rounded
rectangle with a radius of at least 1 px when rounding is enabled.

The static `preview.png` is a **composition**, not a raw frame: `compose_branded_preview()`
places the 21:37 frame inside the `gear-outlayer.png` ring with black safe areas, a
`#404040` outline and full transparency outside. `rw_encode_preview_gif` turns that image
into GIF frame 1, follows it with the seven raw frames, draws the sea-breeze shortcut hatch
(`#7DE2D1`, icon `Assets/Icons/click-zone.png`) on frame 2, sets infinite repeat and deletes
`preview-cache`.

### 10.3 QR installation

`qr.rs` uses the pure-Rust `qrcode` crate with `EcLevel::L` — the same level as
`qrcode-terminal` in zeus — and returns a base64 PNG from `rw_generate_qr`. No cloud service,
account, token or third-party QR API is involved.

Clicking **Export** prepares everything before showing the dialog: a fresh ZIP, a free TCP
port, a `TcpListener` on the LAN IPv4 and the QR image. The URL is
`zpkd1://<ip>:<port>/<name>.zip`, exactly the scheme of `zeus preview --local`. The server
strips query strings and fragments, URL-decodes the path, accepts any `.zip` request and
handles `HEAD`; a local GET self-check must return 200 before the QR is generated. The QR
image stays hidden until the user clicks the QR button, and the server stops when the dialog
or the window closes.

### 10.4 Simulator workspace

`prepare_emulator_workspace(project, root)` clears `<root>/app-<appId>/`, exports the ZIP,
extracts it, then `convert_export_for_zeus` rewrites `app.json` to v3 with
`targets["active2round"]` and copies assets into `assets/active2round/`. The C# side runs
`zeus install --target "Amazfit Active 2 NFC (Round)"`. zeus may print `[ERROR]` while
exiting with code 0 — both are treated as failure. If the simulator is missing, the app only
opens the official download page; Zepp's binaries are never bundled or silently installed.

---

## 11. WinUI 3 front-end

### 11.1 Startup and app data

`App.xaml.cs` creates the single `MainWindow`. Its constructor runs in a deliberate order:
`AppPaths.Initialize()` → `RustNative.EnsureDefaults()` → `LocalizationService.Initialize()` →
`LocalizationBridge` → `InitializeComponent()` → event wiring → `AppWindow.Resize(1280, 800)`
and icon → start of the 1 s `_clockTimer` → load settings/recent/catalogs → show HOME →
`_initialized = true`. Setting `_initialized` last prevents `SelectionChanged` handlers from
running during XAML initialization (a source of past `0x80004003` crashes).

`AppPaths` owns every path under `%APPDATA%\RustyWatch`: `settings.json`, `recent.json`,
`Projects`, `Emulator`, `Fonts`, `Images`, `Hands`, `Status`, `Localization`, plus a
`FontCache` next to the executable. It seeds missing localization files, merges missing keys
into existing catalogs (never overwriting user values) and copies legacy files from
`%LOCALAPPDATA%\RustyWatch` only when the roaming copy is absent. `AppSettingsStore` is the
only writer of settings; `Save()` writes all keys in one object, so saving one setting never
erases another.

### 11.2 Navigation and pages

`NavigationView` has `IsSettingsVisible="False"`, `OpenPaneLength="210"` and
`CompactPaneLength="52"` (both values work around clipped Polish labels and footer icons).
Menu items: Home, Editor (hidden until a project is open), Widgets (always visible), Assets,
plus a hidden `BlankNavItem`. The footer holds **plain** `NavigationViewItem`s for Project
settings and App settings so NavigationView supplies their own selection visuals.

Footer items do not raise `SelectionChanged`; the constructor attaches `TappedEvent` handlers
with `handledEventsToo: true`. Clicking one selects `BlankNavItem` (never `null`, which leaves
a stale blue indicator), highlights the footer item and calls `Navigate(tag)`. The six views
are toggled by `ShowView()`. The bottom bar holds Save, Save as…, Test in emulator and Export
(the rightmost accent button); the latter two are enabled only when the editor is showing an
open project.

### 11.3 Editor preview and gestures

`PopulateWatchface(watchCanvas, outlineCanvas)` is the only preview construction path. It
walks `_layers` in Rust order and skips hidden widgets. The round 466×466 area is a `Viewbox`
with a black ellipse and a `Border` with `CornerRadius="233"`; grid lines, alignment guides and
the unselected outline live in an overlay that ignores hit testing. **Frames and handles live
on a sibling `OutlineCanvas` outside the round clip**, so the corners of a 400×400 hand frame
stay grabbable even though they fall outside the watch circle.

Each `FrameView` owns a host `Grid` at exact frame size, a transparent `Border` frame, a
content-sized `TextBlock`, an optional image/shortcut/progress visual and four corner handles.
Text alignment is applied only in `ApplyTextLayout()`, always at the end of
`ApplyFrameGeometry()`; the `TextBlock` has `Height = double.NaN` with `Width`/`MaxHeight`
matching the current frame, so scaling cannot leave a stale anchor.

Pointer handling:

- `Frame_PointerPressed` selects the layer, determines the handle from the element name and
  captures the pointer. A second press on the same layer within 450 ms toggles scale mode.
  Image-like kinds (analog, images, STATUS, weather icon) are forced to `move` unless the red
  scale mode is active.
- `OutlineCanvas_PointerMoved` sends geometry to `rw_frame_geometry` with `square` for analog,
  arc and weather-icon layers, `minSize = 1` for bars and shortcuts, `maxSize = 256` for the
  weather icon (same as the slider), the checkbox state as `snap`, and
  alignment candidates from the canvas (`0`, `233`, `466`) plus every other visible widget's
  edges and centres. It only applies the returned numbers and draws the returned guides.
- `OutlineCanvas_PointerReleased` persists the final geometry. The arc's extra `thick` handle
  changes only `Progress.Thickness`.

The Layers panel shows top-first rows with eye / up / down / delete buttons in a single
horizontal stack. Double-clicking empty space clears the selection and hides the inspector.
`ApplyLayersPayload()` compares layer structure and, when only geometry changed, updates
existing row/frame objects in place rather than rebuilding the preview.

The inspector is driven by the **selected** layer. `LoadWidgetSettings()` collapses panels by
kind: format presets only for `HasPattern` kinds (`time`, `date`), user text only for `text`,
sensor/goal/distance hints where appropriate, analog hand sets and per-hand checkboxes,
image fit, STATUS pack, weather pack/day, progress settings and shortcut target. Opacity is
shared by every kind. `PersistSettings()` sends one JSON object to `rw_set_widget` with
`deferAssets: true`; expensive asset rescaling is then debounced 180 ms behind `_assetGate`
and executed by `rw_rescale_widget_assets` on a background thread.

The preview defaults to predefined data (31 July 2022, 21:37:40, from
`rw_preview_default_datetime`). With **Live preview** on, the 1 s timer calls
`UpdateLiveTimeDateOnly()`: only time, date, date-derived fields and hand angles move to
`DateTime.Now`; weather, battery and sensor values stay predefined, and
`SetTextIfChanged()` avoids redundant writes. The options flyout also toggles the 16 px grid
(and snapping), the unselected outline and the Custom Data dialog. Custom values live in
`settings.json`, never in `project.json`, and never affect generated preview files.

### 11.4 Assets page

Five GridView sections cover fonts, images, hands, STATUS packs and weather packs. Tiles are
sized to at most three per row; hand/status/weather packs show a Rust-generated thumbnail with
a `ProgressRing` while a new or changed image is being decoded. The fonts grid starts with a
virtual, informational tile for the watch system font (Noto Sans preview, localized default
badge, no Remove button); Inter has no badge but stays protected from removal, and only
user-added TTFs can be removed. Hand/status built-ins keep the default badge; among weather
packs only the default `system-like` is badged, while other bundled packs are protected from
removal without the badge.
Entering Assets never blocks navigation: `Navigate("assets")` shows the view
first, then bumps a version counter, runs four store queries in parallel `Task.Run`s and
marshals results back with `DispatcherQueue.TryEnqueue`; stale responses are discarded.

Selecting a STATUS or hand pack runs `rw_set_status_pack` / `rw_set_hands` in the background,
preserving hand-visibility checkboxes for the new set. Weather is split: the visible icon is
decoded immediately from the shared store, while the 29-file copy/resize is queued behind a
120 ms debounce, a cancellation token and a version check ("last selection wins"). Export
awaits pending asset jobs so the ZIP cannot be built from half-updated assets.

### 11.5 Export dialog and local server

`Export_Click` disables the button, waits for asset jobs, asks Rust for a random series,
renders all eight frames through `rw_render_preview_frame` while counting `1/8 … 8/8`, then
switches the bar to indeterminate during `rw_encode_preview_gif`. The GIF is shown in a 240×240
border with `CornerRadius="120"` so only the round watchface is visible.

The Export click itself calls `ExportZipNow()` (`rw_export_zip`) and shows the
"Exported: …" status — the dialog has **no** Export ZIP button. Before the dialog opens,
`PrepareQrAsync(zipPath)` chooses a free port, starts the `TcpListener`, performs the loopback
self-check and generates the QR bitmap into window state (`_qrBitmap`, `_qrUrl`).
`ResetQrView()` clears all of that on each Export. The QR button only displays the prepared
bitmap; the image is therefore invisible until it is clicked. The dialog is a `ScrollViewer`,
and the QR uses `MaxWidth`/`MaxHeight` with `Stretch="Uniform"` so it cannot be clipped in a
lower window. `ExportDialog.Closing` and `MainWindow.Closed` stop the server.

### 11.6 Simulator integration

`ZeppSimulator.FindExecutable()` checks known install paths and registry uninstall keys. If
the simulator is missing, only the official download page is opened. Device readiness reads
`%APPDATA%\simulator\config.json` (`selectDeviceList` may be an object or an array) and looks
for device source `8913153` with status ≥ 2; a running watch window is detected via `simuOpen`
or `firmwareSimulatorIsReady`. `zeus` is located on `PATH`. The install command runs with
redirected UTF-8 output, and a non-zero exit code **or** the string `[ERROR]` in the log is a
failure.

### 11.7 Localization in the UI

`LocalizationService` discovers catalogs from `*.json` filenames in the appdata folder, uses
the first/English catalog as fallback and tracks the chosen language in `settings.json`.
`StringCatalog` exposes the `{Binding Strings[Key]}` indexer and raises `PropertyChanged` on
refresh; `LocalizationBridge.Refresh()` rebinds dialog titles and option labels. Changing the
language in App Settings calls `SetLanguage()` and then `ApplyLocalization()` to rebuild
catalog-dependent lists. Reload languages re-enumerates the folder. `LocalizationService.Error`
maps the Polish Rust diagnostics to catalog keys for English. The separate watchface preview
language (`pl`/`en`) is fed into the formatting and preview FFI calls.

### 11.8 Build orchestration

`RustyWatch.csproj` targets `net8.0-windows10.0.26100.0`, uses WinUI 3 with Windows App SDK
2.7.0 and is self-contained for `win-x64`. Its `BuildRustFfi` target runs
`cargo build -p rusty-ffi` before `Build`; `CopyRustFfi` copies the DLL after `Build` with
`SkipUnchangedFiles="false"` so a stale DLL cannot survive. Bundled fonts, icons, hand/status/
weather PNGs, license files and localization JSON are copied to the output with
`PreserveNewest`.

---

## 12. Invariants and learned traps

These are the rules that were paid for with a broken watchface or a crashed UI. Do not
"simplify" them away.

- **Classic API only.** `@zos/ui` and `@zos/sensor` are Mini Program APIs; with API 3.x the
  watchface renders black. `app.json` has no `runtime.type`, because `type: 2` loads `.bin`
  bytecode instead of our `.js`.
- **`ONLY_NORMAL` always.** `ONAL_AOD` made real firmware paint an opaque AOD overlay. AOD is
  driven by `WIDGET_DELEGATE` + `hmSetting.getScreenType()`.
- **No `Date` in generated JS.** The simulator tolerates it; the real watch does not.
- **Layers are the draw order.** Both preview and codegen use `layers::ordered_widgets()`.
  There is no background layer or background asset.
- **Preview is Rust-only.** Offscreen WinUI controls leaked artifacts and shifted images.
  Custom editor data never enters generated preview files.
- **Hand textures need premultiplied alpha and transparent RGB = (0,0,0).** Straight-alpha
  resampling of the bundled Classic set produced white rectangles on the watch. `IMG` also
  does not scale at runtime, so PNGs must be physically resized and tightly cropped.
- **Weather icons keep premultiplied RGB in the saved PNG** for the same reason; the icon's
  `SRC` is updated only on change.
- **Heart rate uses `HEART.last` on `LAST`.** `CURRENT` means continuous measurement and
  battery drain; polling from `tick` is not acceptable.
- **Goal progress divides by the watch's `.target`**, in both JS and the Rust preview.
- **All JS dimensions are literals.** Bare `x`/`y`/`w`/`h` identifiers caused a
  `ReferenceError` that left progress bars unfilled.
- **Renderer parity.** Arc margins follow `max(0, thickness/2 − 8) + 6`; reversed sweeps are
  real arcs, not full circles; rounded bars never get a 0 px radius; the editor refreshes the
  track's line caps too.
- **Shortcut zones have `src: ""`.** A bundled 1×1 transparent PNG was rejected by QR installs
  and by `zeus install`. `generate_index_js` creates shortcuts **last** (`creation_order`), so a
  non-clickable widget layered above them cannot swallow a tap; layer order only sorts shortcuts
  relative to each other.
- **Do not set `FontWeight` on preview text** — a synthesized bold breaks dot-matrix fonts.
- **WinUI 3:** no `ApplicationBackgroundThemeBrush`; `Window` has no XAML size/font members;
  `ContentDialog` must be a direct child of the root `Grid` and has no `Opening` event; guard
  handlers until `_initialized`; keep `CopyRustFfi` with `SkipUnchangedFiles="false"`; never
  downgrade Windows App SDK below 2.7.0.
- **Localization catalogs must stay valid JSON**, in both bundled seeds and appdata copies;
  keep `en.json` and `pl.json` keys in sync.
- **No debug hooks** (`RW_DEBUG_*`) in shipped code.

---

## 13. File index

| Path | What lives there |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Workspace members and shared dependencies (`qrcode 0.15`, `base64 0.22`, `image 0.25`, `gif 0.14`, `ab_glyph`, `zip 2`). |
| `crates/rusty-core/src/model.rs` | Schema, enums, device profile, JS expression helpers, starter template. |
| `crates/rusty-core/src/project.rs` | Create/load/save/save-as, folder sanitization, App ID, metadata, icon generation. |
| `crates/rusty-core/src/validate.rs` | Pre-export validation. |
| `crates/rusty-core/src/layers.rs`, `widgets.rs` | Draw order and descriptors; catalog, widget editing, frame geometry. |
| `crates/rusty-core/src/time_format.rs`, `date_format.rs`, `locale.rs` | Shared pattern parsers, `Date`-free JS helpers, PL/EN tables. |
| `crates/rusty-core/src/codegen.rs` | Generated `app.json`, `app.js`, `watchface/index.js`. |
| `crates/rusty-core/src/package.rs`, `export_paths.rs` | Flat v2 ZIP, emulator v3 conversion, hashed asset names, 314 px masked icon. |
| `crates/rusty-core/src/preview.rs` | Deterministic frames, branded composition, GIF, shortcut overlays. |
| `crates/rusty-core/src/qr.rs` | Pure-Rust QR PNG for `zpkd1://` URLs. |
| `crates/rusty-core/src/fonts.rs`, `images.rs`, `image_fit.rs`, `hands.rs`, `status_icons.rs`, `weather.rs`, `asset_cache.rs` | Shared stores, attachment/pruning, scaling algorithms, fingerprints. |
| `crates/rusty-core/src/recent.rs` | BOM-tolerant recent-project store. |
| `crates/rusty-cli/src/main.rs` | `new`, `validate`, `export`, `inspect`. |
| `crates/rusty-ffi/src/lib.rs` | 66 `rw_*` exports and the JSON envelope. |
| `native/RustyWatch/MainWindow.xaml(.cs)` | Shell, six views, editor gestures and preview, three dialogs, export/QR/emulator flows. |
| `native/RustyWatch/RustNative.cs` | P/Invoke declarations, DTOs, central `Call` helper. |
| `native/RustyWatch/AppPaths.cs`, `AppSettingsStore.cs` | App-data layout, seeding, settings persistence. |
| `native/RustyWatch/PreviewFonts.cs` | Preview font cache for unpackaged WinUI. |
| `native/RustyWatch/ZeppSimulator.cs` | Simulator/zeus detection and launch. |
| `native/RustyWatch/Localization/` | Bundled catalog seeds, service, catalog and bridge. |
| `native/RustyWatch/Assets/` | Bundled fonts, hands, STATUS/weather icons, click-zone icon, app logo. |
| `native/RustyWatch/RustyWatch.csproj` | Rust build orchestration and content copying. |
| `docs/FORMAT.md` | Project and package format reference. |
| `PLAN.md` | Goals and confirmed decisions (historical). |
| `../AGENTS.md` | Architecture rules and agent workflow. |
