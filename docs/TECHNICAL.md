# Rusty Watch — technical deep-dive

> **Scope.** How the code actually works, module by module, from an editor click to a
> watchface running on an Amazfit Active 2 (Round). Every claim is tied to a file and
> function/line in this repository. This is a companion to [`FORMAT.md`](FORMAT.md)
> (data format) and [`../AGENTS.md`](../AGENTS.md) (rules for AI agents).

## Table of contents

1. [System map and end-to-end data flow](#1-system-map-and-end-to-end-data-flow)
2. [Data model (`rusty-core/src/model.rs`)](#2-data-model)
3. [Project lifecycle, validation and recents](#3-project-lifecycle-validation-and-recents)
4. [Layers: the single source of draw order](#4-layers-the-single-source-of-draw-order)
5. [Widget system and lifecycle](#5-widget-system-and-lifecycle)
6. [FFI bridge (`rusty-ffi` ↔ `RustNative.cs`)](#6-ffi-bridge)
7. [Asset stores: fonts, backgrounds, images, hands, logo](#7-asset-stores)
8. [Time, date and language engine](#8-time-date-and-language-engine)
9. [Code generation (`codegen.rs`)](#9-code-generation)
10. [Packaging, export and emulator](#10-packaging-export-and-emulator)
11. [WinUI 3 front-end (`native/RustyWatch`)](#11-winui-3-front-end)
12. [Cross-cutting invariants and learned traps](#12-cross-cutting-invariants-and-learned-traps)
13. [File index](#13-file-index)

---

## 1. System map and end-to-end data flow

```
 WinUI 3 (C#/XAML)                 native/RustyWatch
 ├─ HOME / navigation / dialogs
 ├─ Editor: WatchCanvas + OverlayCanvas + OutlineCanvas
 ├─ Layers list + inspector + catalog flyout
 ├─ Assets page (Fonts / Backgrounds / Images / Hands)
 └─ RustNative.cs  ── P/Invoke (cdecl, UTF-8, LPUTF8Str) ──┐
                                                          │
                                              crates/rusty-ffi/src/lib.rs
                                              54 × rw_* exports, JSON envelope
                                                          │
                                              crates/rusty-core/src/*
                                              model · project · layers · widgets
                                              codegen · time/date/locale · assets
                                              validate · package · recent
                                                          │
                                    <project>/project.json + assets/
                                                          │
                          export_zip → .zip (app.json, app.js, watchface/index.js,
                                             assets/active2round/*)
                                                          │
                          zeus install → Zepp OS Simulator → real watch
```

**The FFI envelope.** 53 of the 54 exports return `*mut c_char` built by
`respond()` (`rusty-ffi/src/lib.rs:24-32`): `{"ok":true,"data":…}` or
`{"ok":false,"error":"…"}`. The 54th, `rw_free_string` (`:34-39`), frees that exact
`CString::into_raw()` allocation. `read_input` (`:12-18`) is null-safe and lossy-UTF-8;
`string_result` (`:20-22`) adapts `rusty-core`'s `Result<_, String>` to `anyhow`.
C# has one choke point: `RustNative.Call` (`RustNative.cs:295-318`) reads the string,
**always** frees the pointer in `finally`, parses the envelope, and returns `data` or
throws/propagates `error`.

**One edit, step by step (X slider dragged):**

1. `PositionSlider_ValueChanged` (`MainWindow.xaml.cs:2318`) updates `_settings.X`.
2. `ApplyPreview()` re-applies geometry locally; `PersistSettings()` (`:2116-2162`)
   serialises `{id,x,y,w,h,textSize,font,color,align,vAlign,timeFormat,text,…}`.
3. `rw_set_widget` (`rusty-ffi/src/lib.rs:273`) → `widgets::update_widget`
   (`widgets.rs:880-1092`) validates bounds `x+w ≤ 466`, `y+h ≤ 466`, text size
   8–160, colour `0xAARRGGBB`, updates the `Widget`, re-scales hand PNGs or prepares
   the Images PNG if applicable, prunes unused assets, and saves `project.json`.
4. It returns fresh `layers` (from `layers::describe`). `ApplyLayersPayload`
   (`MainWindow.xaml.cs:2164`) compares structure; if only geometry changed it
   updates existing `LayerRow`/`FrameView` objects in place — no list jump, no preview
   flash.
5. The next 1-second `ClockTimer_Tick` re-formats time/date text and rotates hands.

**One export, step by step:**

1. `DialogExportZip_Click` (`MainWindow.xaml.cs:3426`) computes
   `<project folder>/<ExportFileName(name)>.zip` and calls `rw_export_zip`.
2. `export_zip` (`package.rs:17-80`) loads + validates, calls
   `hands::prepare_hand_assets` (returns `HashMap<src, PreparedHand{bytes, placement}>`),
   writes `app.json`, `app.js`, `watchface/index.js`, then copies each registered
   asset to `assets/active2round/<asset.path>` with per-asset byte preparation.
3. The generated JS reads the same `Project` through `layers::ordered_widgets`,
   the very function the preview uses — that is the 1:1 guarantee.

---

## 2. Data model

`rusty-core/src/lib.rs:1-16` declares 16 public modules. `lib.rs` itself only
re-exports types (`:18-25`) and contains the 84 core tests from `:27` onward.

### 2.1 Constants and `Project` (`model.rs:5-37`)

| Constant | Value |
|---|---|
| `SCHEMA_VERSION` | `1` (rejected on load if different, `project.rs:229-235`) |
| `SCREEN_SIZE` | `466` |
| `BACKGROUND_LAYER_ID` | `"background"` (reserved synthetic layer) |

`Project` is `#[serde(rename_all="camelCase")]` with fields:
`schema_version`, `metadata`, `device`, `widgets: Vec<Widget>`, `assets: Vec<Asset>`
(default, skipped when empty), `custom_code: Option<String>`, `background:
Option<BackgroundConfig>`, `layer_order: Vec<String>` (bottom→top, default = derive
from `z`), `measurement_system: MeasurementSystem` (default `Watch`),
`first_day_of_week: FirstDayOfWeek` (default `Monday`).

`Metadata` (`model.rs:126-134`): `name`, `app_id` (`appId`), `author`, `version`,
`description` — all required. `DeviceId` has one variant, serialized
`"active-2-round"`; `DeviceId::profile()` (`:144-155`) returns
`target_key="active2round"`, `device_source=8_913_153`, `design_width=466`, and
profile API hints 3.0.0/3.6.0 (which are intentionally *not* used in `app.json`).

### 2.2 `Widget` (`model.rs:214-286`)

Every field has a serde default if optional, so older `project.json` files load:

| Group | Fields |
|---|---|
| Identity | `id`, `name`, `kind` (`WidgetKind`, snake_case), `z` (default 0) |
| Geometry | `x`, `y`, `w`, `h` (`i32`), `auto_scale` (`bool`, default false) |
| Text | `text`, `text_size`, `font`, `time_format`, `color`, `text_style` (`Ellipsis` default), `align` (`Option<Align>`), `v_align` (`VAlign::Center` default) |
| Media | `src`, `image_fit` (`FitMode::Contain` default), `hand_set`, `hour_src`, `minute_src`, `second_src`, `cap_src` |
| Data | `data_source: Option<DataSource>`, `visible` (default **true**), `measurement_system: Option<MeasurementSystem>` (override), `show_unit` (default **true**) |

`WidgetKind` has 21 addable/logical variants (`model.rs:288-331`): `Text`, `Image`
(legacy, not in catalog), `Images`, `TimeText`, `DateText`, `WeekdayNumberText`,
`WeekNumberText`, `DayOfYearText`, `LunarDateText`, `FestivalText`, `WorldClockText`,
`StepText`, `DistanceText`, `HeartRateText`, `StressText`, `PaiText`, `SkinTempText`,
`SleepText`, `CalorieText`, `Spo2Text`, `AnalogTime`. Helpers:
`health_initial_text(show_unit)` (`:336-351`), `is_date_field()` (`:354-364`),
`is_health()` (`:367-378`).

Enum → generated JS (`model.rs`):
- `Align::as_js()` (`:415-421`) → `hmUI.align.LEFT|CENTER_H|RIGHT`.
- `VAlign::as_js()` (`:435-441`) → `hmUI.align.TOP|CENTER_V|BOTTOM`; `parse()` accepts
  `top`, `center|middle`, `bottom`.
- `TextStyle::as_js()` (`:474-482`) → `hmUI.text_style.ELLIPSIS` / `.NONE`;
  `Wrap` deliberately also maps to `.NONE` because the watchface API has no WRAP.
- `ShowLevel::as_js()` (`:512-519`) → `ONLY_NORMAL`, **`ONAL_AOD`** (the ZeppOS typo
  is intentional), `ONLY_EDIT`, and `ONLY_NORMAL | ONAL_AOD` for `Always`.
- `MeasurementSystem` (`model.rs:66-124`): `watch` (unit read at runtime), `metric`,
  `imperial`; `parse()` also accepts `auto|km|mi|miles`; `meters_per_unit()` 1000 /
  1609.344; `suffix()` `km`/`mi`; `initial_distance_text()` → `"0.00 km"`.
- `FirstDayOfWeek::parse()` (`model.rs:50-56`) accepts EN and PL spellings
  (`monday|mon|poniedzialek|pon`, `sunday|sun|niedziela|ndz`).

`Asset { path, role }` with `AssetRole` = `Icon|Background|Font|Image|Hand|Other`
(`model.rs:193-212`, snake_case). `BackgroundConfig { image, auto_scale }` defaults
`auto_scale` to `true` (`model.rs:171-181`).

### 2.3 Starter template (`model.rs:523-599`)

`Project::starter_with(name, author, app_id)` produces
`schema_version=1`, `version="0.1.0"`, assets `bg.png`(Background) + `icon.png`(Icon),
**one** widget `id="time"`, kind `TimeText`, `(23,183,420,100)`, `text="00:00:00"`,
`text_size=64`, `time_format="HH:MM:SS"`, `color="0xFFFFFFFF"`, `align=Center`,
`show_level=Always`, and `layer_order=["background","time"]`, `measurement_system=Watch`,
`first_day_of_week=Monday`. (*`create_project_with` then overrides `background` to
the black built-in — see §3.2.*)

---

## 3. Project lifecycle, validation and recents

### 3.1 Creation and folders (`project.rs`)

`NewProject::validate()` (`:33-54`) rejects empty name, empty author, and `app_id == 0`.
`random_app_id()` (`:56-69`) mixes `SystemTime` sub-second nanos with a PID-derived
xorshift32 seed, returning an 8-digit id in `[10_000_000, 100_000_000)`.

`create_project_with(dir, input)` (`:82-123`):
1. reject existing `project.json` (never overwrite);
2. create `assets/`; build `Project::starter_with` from trimmed inputs;
3. write an opaque black 466×466 `assets/bg.png` (`generate_black_background`, `:259-262`);
4. write the same PNG as `assets/backgrounds/black.png` and register it as
   `Asset{path:"backgrounds/black.png", role:Image}`;
5. set `background = Some(BackgroundConfig{image:"black.png", auto_scale:true})`;
6. generate `assets/icon.png` (100×100, dark disc + blue ring, `:285-305`);
7. save `project.json`.

`watchface_folder_name` (`:126-154`) replaces control chars and
`< > : " / \ | ? *` with `_`, trims trailing dots/spaces, falls back to `Watchface`,
and prefixes reserved Windows device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`,
`LPT1-9`) with `_`. `create_project_in` (`:157-165`) refuses an existing folder.

`save_project_as` (`:168-194`) creates a **fresh** project in the destination (new
bg/icon, new App ID), then overwrites metadata name/author/appId/description while
preserving widgets, layers, version and all assets, and physically copies every
`assets/<path>` from the source.

### 3.2 Load / save / metadata (`project.rs:196-246`)

`load_project` checks existence, parses JSON, and rejects `schema_version != 1`.
Missing optional fields are filled by serde defaults. `save_project` writes
pretty JSON + trailing newline. `update_project_metadata` (`:198-219`) changes only
author and version; version must be non-empty, ≤32 chars and whitespace/control-free.
**The name is deliberately immutable** — folders and recents depend on it.

`set_measurement_system` (`:19-26`) also refreshes the initial text of every
`DistanceText` widget without its own override; `set_first_day_of_week` (`:29-31`)
only sets the project field.

### 3.3 Validation (`validate.rs:6-145`)

`validate(project) -> Vec<String>` collects **all** errors (no short-circuit):

- Project: non-empty name/author, `app_id > 0`, `device == Active2Round`, at least one widget.
- Every widget: non-empty name, unique id, `x,y ≥ 0`, `x+w ≤ 466`, `y+h ≤ 466`,
  positive `w/h`.
- Text kinds: `text_size != 0`, pattern parse (date parser for `DateText`, time parser
  otherwise), `0xAARRGGBB` colour, and the referenced font present as a `fonts/…` asset.
- `Image`/`Images`: `src` present and in the asset set.
- `AnalogTime`: each configured `*_src` present in the asset set; a hidden hand
  (`None`) is fine.
- Asset paths: no empty path, no `..`, no leading `/`.

`rw_save_project` runs validation **before** writing (`rusty-ffi/src/lib.rs:161`), so
the UI can never persist an invalid project silently.

### 3.4 Recents (`recent.rs`)

`RecentProject { name, file, opened_at }` (`recent.rs:15-23`, camelCase). Store =
`%APPDATA%/RustyWatch/recent.json`, max 10 entries (`MAX_RECENT`, `:12`). `load`
(`:29-67`) strips a UTF-8 BOM and accepts three shapes: canonical array, PowerShell's
`{"value":[…],"Count":N}`, or a single `{name,file}` object. `write` (`:69-76`) always
rewrites a pretty **array**. `file_key` (`:85-89`) canonicalises and lowercases the
path for Windows-style case-insensitive dedupe. `record` (`:91-107`) drops duplicates,
inserts at index 0, truncates to 10. `rw_open_project` (`rusty-ffi/src/lib.rs:974`)
deliberately falls back to an empty list when the recent store is corrupt — a broken
`recent.json` never prevents opening a project.

---

## 4. Layers: the single source of draw order

`layers.rs` is the authority for **both** the WinUI preview and the generated JS.

- `order(project)` (`:19-47`) filters `project.layer_order` to valid widget ids plus
  `"background"` (only when a `Background`-role asset exists), removes duplicates,
  appends missing widgets stably sorted by `z`, and inserts `"background"` at index 0
  if absent. `z` is thus only a fallback/tiebreaker; `layer_order` wins when valid.
- `apply_order` (`:49-56`) writes `layer_order` **and** sets each widget's `z` to its
  index (so with a background present, widgets start at `z=1`).
- `move_layer(project, id, delta)` (`:65-86`): `id=="background"` → error
  `"Tło jest zablokowane na najniższej warstwie"`; widgets cannot move below index 1;
  result saved immediately.
- `remove_widget` (`:88-105`): background locked; the last remaining widget cannot be removed.
- `set_widget_visibility` (`:344-360`): background cannot be hidden; hidden widgets
  stay in `project.json` and in the Layers list (`visible=false`).
- `describe(project)` (`:204-283`) returns `LayerView` descriptors (camelCase) used by
  the inspector: `id, name, kind, x/y/w/h, text, timeFormat, textSize, font, color,
  align, vAlign, src, imageFit, autoScale, textStyle, visible, backgroundImage,
  backgroundAutoScale, showUnit, handSet, hourSrc/minuteSrc/secondSrc/capSrc,
  measurementSystem`. `widget_kind()` (`:166-194`) maps `DataSource`/`WidgetKind` to
  catalog strings such as `weekday-number`, `steps`, `heart`, `images`, `analog-time`.
- `background_widget(project)` (`:285-324`) synthesises a full-screen `WidgetKind::Image`
  `(0,0,466,466)`, `src="bg.png"`, `image_fit=Cover`, `show_level=Always`.
- `ordered_widgets(project)` (`:326-342`) substitutes that synthetic widget for
  `"background"`, clones the rest, and **filters `visible == false`**. Both
  `codegen.rs:178` and the WinUI preview consume this one function — that is why
  hidden layers disappear from the watchface exactly as they do in the editor.

---

## 5. Widget system and lifecycle

### 5.1 Catalog (`widgets.rs:12-117`)

`supported_widgets()` returns 21 `{kind,category}` entries in this order:

| Category | Kinds |
|---|---|
| `time` | `time`, `analog-time` |
| `date` | `date`, `weekday-number`, `week-number`, `day-of-year`, `lunar-date`, `festival`, `world-clock` |
| `text` | `text` |
| `images` | `images` |
| `distance` | `steps`, `distance` |
| `health` | `heart`, `stress`, `pai`, `skin-temp`, `sleep`, `calories`, `spo2` |

The C# "Widgets" tab renders this catalog as always-expanded plain headers; the
**+ Add widget** flyout renders the same list as collapsed `Expander` groups
(`MainWindow.xaml.cs:2005-2049`) whose search auto-expands matching groups
(`FilterWidgetCatalog`, `:2051-2073`). Adding is `rw_add_widget` →
`widgets::add_widget` (`:513-878`), which generates ids `time`, `time-2`, … and lays
out defaults: analog `(33,33,400,400)`, images `(133,133,200,200)`, date
`(23,250,420,80)` with pattern `"MMM D"`, text `(23,350,420,60)`, steps
`(23,320,420,60)`, distance `(23,260,420,60)`, date fields `(23,200,420,60)`,
health `(23,150,420,60)`, time `(23,183,420,100)` with `"HH:MM:SS"`.

### 5.2 Editing (`widgets.rs:880-1092`)

`update_widget(project, settings)` requires `id,x,y,w,h`; enforces screen bounds,
`textSize ∈ 8..=160`, colour format, parses `align`/`vAlign`/`textStyle`/`imageFit`/
`measurementSystem`, and applies `text` only to `WidgetKind::Text`. A blank font
value *keeps* the current font (`:1012-1015`); `measurementSystem=""` clears the
per-widget override. After the write it recomputes initial preview text from the
pattern (`time_format::format(…,0,0,0)` or `date_format::format(…,1,0,2026)`).

### 5.3 Frame geometry (`widgets.rs:184-439`)

Constants: `MIN_FRAME_SIZE=24`, `MIN_TEXT_SIZE=8`, `MAX_TEXT_SIZE=160`,
`SNAP_GRID=16`, `ALIGN_THRESHOLD=5`. `frame_geometry(FrameInput)` is a **pure**
function called on every pointer move (C# only applies its result; it never snaps
itself). `FrameInput` carries `x,y,w,h,textSize,handle(move|nw|ne|sw|se),dx,dy,
autoScale,snap,alignX,alignY,square`.

- `resolve_axis` (`:214-240`) checks alignment on the **pre-grid** value — essential,
  because a 16 px grid can move a widget up to 8 px and would otherwise break the
  5 px threshold for center snapping. Alignment wins over the grid.
- Move branch (`:300-312`): edges = `{left,center,right}` / `{top,center,bottom}`;
  candidates start with `{0,233,466}` in C# plus every other visible widget's edges
  and centers. Accepted guides are returned as `guideX`/`guideY`.
- Square branch (`:313-386`, analog hands): the axis with the larger delta wins,
  `w == h` always, alignment resolved with `snap=false` first.
- Resize branch (`:387-420`): clamp each edge to 466 and keep a 24 px minimum.
- Auto-scale (`:422-428`): with `auto_scale=true`, `text_size` is scaled by
  `new_w / old_w` using integer rounding, clamped to 8–160. **The real value is
  persisted**, so preview and exported JS stay identical.

### 5.4 Lifecycle side effects

`rw_set_widget` (FFI `:273`), `rw_set_hands` (`:742`),
`rw_set_hand_visibility` (`:781`), `rw_set_widget_image` (`:599`),
`rw_set_background` (`:657`), `rw_remove_widget` (`:827`) and the default-font
exports are the only functions that mutate `%APPDATA%` stores and/or
`<project>/assets/*`; each saves the project before returning. `rw_add_widget`
auto-attaches the Classic hand set to analog widgets; `rw_apply_default_font_to_missing`
(`:494`) is called on every project open so widgets with `font: null` get Inter —
without it the PC preview uses Segoe UI while the watch uses its system font, which
looks like broken alignment.

---

## 6. FFI bridge

### 6.1 Contract details

- 54 `#[no_mangle] pub extern "C" fn rw_*` exports; 25 `#[test]` tests in the same file.
- `rw_inspect_zip` (`:917`) has **no P/Invoke declaration** in `RustNative.cs` — it is
  CLI/test-only. Every other export is declared with
  `[DllImport("rusty_ffi", CallingConvention = CallingConvention.Cdecl, EntryPoint = "rw_…")]`.
- Input strings use `UnmanagedType.LPUTF8Str`. Numeric types map `u32↔uint`, `i32↔int`.
- Booleans are deliberately non-uniform: `visible` uses `[MarshalAs(UnmanagedType.I1)]`
  (Rust `bool` is 1 byte); `autoScale` crosses as `i32` 0/1; hand visibility crosses
  as a JSON object `{"hour":bool?,"minute":bool?,"second":bool?,"cap":bool?}`.
- `RustNative.Call` (`:295-318`) is the single decode/free point. `AppPaths` injects
  the implicit `app_data_root` / bundled-directory arguments that C# callers never pass.

### 6.2 Export catalog (grouped)

| Domain | Functions (line in `rusty-ffi/src/lib.rs`) |
|---|---|
| Memory/version | `rw_free_string` 35, `rw_version` 42 |
| Project | `rw_create_project` 51, `rw_create_project_in` 81, `rw_save_project_as` 112, `rw_random_app_id` 146, `rw_load_project` 151, `rw_save_project` 161, `rw_set_project_metadata` 854, `rw_validate_project` 876, `rw_open_project` 974 |
| Layers/widgets | `rw_editor_layers` 181, `rw_move_layer` 192, `rw_supported_widgets` 213, `rw_frame_geometry` 221, `rw_add_widget` 233, `rw_set_widget` 273, `rw_set_measurement_system` 328, `rw_set_first_day_of_week` 348, `rw_set_widget_visibility` 368, `rw_remove_widget` 827 |
| Defaults | `rw_ensure_defaults` 393 |
| Fonts | `rw_fonts_list` 421, `rw_fonts_add` 438, `rw_fonts_remove` 455, `rw_apply_default_font` 470, `rw_apply_default_font_to_missing` 494 |
| Backgrounds | `rw_backgrounds_list` 522, `rw_backgrounds_add` 532, `rw_backgrounds_remove` 546, `rw_ensure_project_background` 637, `rw_set_background` 657 |
| Images | `rw_images_list` 560, `rw_images_add` 570, `rw_images_remove` 584, `rw_set_widget_image` 599 |
| Hands | `rw_hands_list` 691, `rw_hands_add` 710, `rw_hands_remove` 727, `rw_set_hands` 742, `rw_set_hand_visibility` 781 |
| Export/emulator | `rw_export_zip` 886, `rw_prepare_emulator_workspace` 898, `rw_inspect_zip` 917 |
| Recents | `rw_recent_list` 929, `rw_recent_add` 938, `rw_recent_remove` 951, `rw_recent_clear` 964 |
| Preview formatting | `rw_month_name` 995, `rw_format_time` 1026, `rw_format_date` 1052, `rw_format_date_field` 1079 |
| Legacy time widget | `rw_time_widget` 1112, `rw_set_time_widget` 1143 (exported but unused by current UI) |

Most functions return `layers` in their `data` so the UI can repaint from one response.
C# DTOs mirror the camelCase keys: `LayerView` (`MainWindow.xaml.cs:38-70`) with
explicit `[JsonPropertyName]`, plus `FontAssetDto`, `ImageAssetDto`,
`SupportedWidgetDto`, `HandSetDto` (`RustNative.cs:7-22`).

---

## 7. Asset stores

All stores live in `%APPDATA%\RustyWatch`. Bundled seeds are copied next to the exe.

### 7.1 Fonts (`fonts.rs`)

- `FONT_DIR="Fonts"`, `DEFAULT_FONT_FILE="Inter-Regular.ttf"`, `js_font_path(file) = "fonts/"+file` (`:15-21`).
- `ensure_default` (`:48-63`) copies the bundled Inter TTF if absent; `is_default` is
  case-insensitive; `list` (`:66-103`) sorts default first.
- `add` (`:109-130`) sanitises the filename and refuses the reserved default name.
- `read_family_name`/`parse_family_name` (`:293-385`) parse the TTF `name` table
  (BE u16 table count at `[4..6]`, 16-byte records from byte 12, `name_id == 1`,
  platform 3 → UTF-16BE, otherwise bytes-as-chars) to obtain the family used by both
  the preview and the Assets "Aa" sample.
- `attach_to_project` (`:166-192`) copies to `<project>/assets/fonts/<file>` and
  registers `AssetRole::Font`; `prune_unused` (`:196-216`) deletes unattached fonts.
- `apply_default_font(_to_missing)` (`:219-260`) skips `Image` and `AnalogTime` widgets
  (but not plural `Images`) and preserves custom choices.

### 7.2 Backgrounds (`backgrounds.rs`)

- Store `Backgrounds/`, project source `assets/backgrounds/<file>`, watch-ready
  `assets/bg.png` (`:17-22`). `BUILTIN_BLACK_FILE="black.png"`,
  `BUILTIN_STARRY_FILE="starry.png"`; legacy `"black"`/`"starry"` and `""` are still
  accepted by `set_background` (`:207-213`).
- `ensure_migrated` (`:43-73`) copies legacy images from `Images/` once (marker
  `.migrated`) and **always** calls `ensure_builtins` first, so built-ins appear even
  in existing installs. `ensure_builtins` (`:78-93`) seeds `black.png` (opaque
  RGBA 0,0,0,255) and `starry.png` (radial gradient + ring + deterministic stars),
  skipping any user-modified file. `remove` (`:157-164`) rejects built-ins.
- `set_background` (`:200-280`) chooses source order store → legacy store → project,
  prepares `bg.png` with `fit_image(…,466,466,Cover)` when `auto_scale=true` (otherwise
  native dimensions), registers `bg.png`(Background) and `backgrounds/<file>`(Image),
  and writes `BackgroundConfig`. `set_builtin_background`/`clear_background`
  (`:282-338`) regenerate the PNG and remove the previous non-builtin source.

### 7.3 Images and shared scaling (`images.rs`, `image_fit.rs`)

- Separate store `Images/`; `sanitize_file_name` (`:314-339`) keeps ASCII
  alphanumeric/space/`-`/`_`/`.`, lowercases extensions (`jpeg→jpg`), truncates stems
  to 80 chars. `valid_asset_name` (`:302-312`) rejects `/`, `\`, `..`.
- `set_widget_image` (`:144-189`) requires `WidgetKind::Images`, reads `w×h`, prepares
  the PNG from the store with `fit_image`, writes `assets/images/<widget-id>/<file>`,
  registers the asset, and sets `widget.src`/`widget.image_fit`.
- `scale_widget_image` (`:210-236`) re-prepares from the pristine store copy when the
  frame changes; `export_asset_bytes` (`:240-266`) re-applies the same fit in memory at
  export; `prune_unused` (`:268-289`) removes orphaned `images/…` files.
- `FitMode` = `cover|contain|stretch` (`image_fit.rs:13-43`, default `Contain`).
  `cover_crop` (`:45-85`) centre-crops to the destination aspect; `fit_image`
  (`:97-130`) uses Lanczos3 when downscaling and CatmullRom when upscaling; `Contain`
  pads with **transparent** pixels; `Cover` guarantees no empty edges; output is
  always RGBA8 of the requested size.

### 7.4 Hands (`hands.rs`)

- Store `Hands/<id>/`, required `hour.png`+`minute.png` (equal square size), optional
  `second.png`/`cap.png`; `js_hand_path(id,file)="hands/<id>/<file>"` (`:22-54`).
- `ensure_default` (`:61-100`) seeds **every** bundled set (Classic and Light) and
  skips already-installed ones; it must not return early after Classic.
- `add` (`:131-176`) validates square/equal sizes and sanitises ids (Polish
  diacritic transliteration, `:625-659`); `remove` (`:179-189`) protects `Classic`.
- `attach_to_project` (`:198-252`) copies files, registers `AssetRole::Hand`, and
  preserves an existing widget frame (`x/y/w/h`) when switching sets — otherwise
  centres a new frame of the set's native size.
- `set_hand_visibility` (`:254-354`) clears `*_src` for hidden hands and restores the
  PNG from the store for shown ones, then prunes unused assets.
- `scale_project_hands` (`:542-588`) physically resizes project PNGs to the widget's
  `w×h`, **always starting from the pristine store copy** (no accumulated quality
  loss); `resized_png` (`:374-393`) short-circuits when dimensions already match.
- `prepare_hand_assets` (`:415-520`) is the export-time trick: ZeppOS draws an `IMG`
  at its natural size and clips it to `w×h`, so each hand is decoded, optionally
  resized to the frame, and — if it saves >15 % of the area — cropped to a **square
  centred on the rotation pivot**. `radius = ceil(farthest opaque pixel distance)+1`,
  `size = 2·radius+1` (odd), and the emitted `HandPlacement{x,y,w,h,center_x,center_y}`
  guarantees `x+center_x == widget.x + w/2`. A tight alpha bounding box would clip a
  rotated hand to stubs near the pivot — that is why the crop is square.
- `export_asset_bytes` (`:522-536`) is the safety net for older projects.

### 7.5 Logo and bundled UI assets

`logo/` holds the vector masters (`logo.svg`, `logo-mark.svg`) and PNG/ICO exports;
`github-avatar.png` is the README/GitHub avatar. `native/RustyWatch/Assets/` contains
`AppIcon.ico`, `Logo.png` (HOME), `Fonts/Inter-Regular.ttf` + `OFL-Inter.txt`, and
`Hands/Classic|Light/*.png`. The `.csproj` copies all of these to the output directory
as content (`RustyWatch.csproj:17-34`).

---

## 8. Time, date and language engine

### 8.1 Time patterns (`time_format.rs`)

`Segment` (`:10-18`): `Literal`, `Hour24{pad}`, `Hour12{pad}`, `Minute{pad}`,
`Second{pad}`, `AmPm{upper}`. `parse` (`:22-100`) rejects empty/>32-char/control
strings and requires at least one token. Tokens: `H/HH`, `h/hh`, `M/MM` (any case),
`S/SS`, `A/a`; `\` escapes the next character literally (`:82-88`). `has_seconds`
(`:147-151`) is what selects the 1-second timer path. `js_expression_localized`
(`:232-245`) builds `p2((h)) + ':' + p2((m))` style expressions with
`L.am/L.pm/L.amUp/L.pmUp` for AM/PM; the non-localized variant emits `'AM'/'PM'`.
`Hour12` is `((h % 12) || 12)` so midnight/noon render as 12.

### 8.2 Date patterns (`date_format.rs`)

`DEFAULT_PATTERN="MMM D"`. Token runs (`:71-161`): `D/DD` day, `M/MM` month number,
`MMM/MMMM` month name, `DDD/DDDD` weekday name, `YY/YYYY` year; 5 or 6 repeated
letters force UPPERCASE, lowercase tokens give lowercase names, uppercase give
capitalised (`:63-69`, `:107-126`). Month index is 0-based in `format_with_locale`
(`:185-229`); weekday names are indexed by Sakamoto's algorithm
`weekday_index(year, month, day)` (`:50-61`, offsets
`[0,3,2,5,0,3,5,1,4,6,2,4]`, 0=Sunday) so the Rust preview agrees with JS
`Date.getDay()`.

Derived fields:
- `weekday_number(day, month, year, first_day)` (`:303-310`) — **month is 1-based at
  this FFI boundary**; Monday-first maps Sunday-based index 0→7 (Monday=1, Tuesday=2),
  Sunday-first maps it to 1.
- `JS_WEEKDAY_INDEX` (`:316-322`) mirrors Sakamoto in JS using the TIME sensor's
  1-based `t.month`; `js_weekday_number(first_day)` (`:325-333`) emits the matching
  `((_w+6)%7)+1` or `_w+1` body. **`t.week` is deliberately not used** — ZeppOS API
  versions disagree about its first day.
- `iso_week` (`:350-373`) / `JS_ISO_WEEK` (`:286-293`) and `day_of_year` (`:336-347`) /
  `JS_DAY_OF_YEAR` (`:295-298`) are paired Rust/JS implementations that produce the
  same numbers in preview and export.

### 8.3 Locale (`locale.rs`)

`POLISH_LANGUAGE_ID = 9`; `WatchfaceLocale::from_language_id` (`:20-26`) returns Polish
**iff** the id is 9, English otherwise. Tables (`:44-115`): PL/EN month abbreviations
and full names, weekdays (`days[0] = Sunday`), and AM/PM markers
(`przedp./pop.` vs `am/pm`). `js_detection()` (`:117-133`) emits at the top of generated
`index.js` a guarded `hmSetting.getLanguage()` call (fallback `2`), `const IS_POLISH = … === 9`,
and the `L = {months, monthsFull, days, daysFull, am, amUp, pm, pmUp}` object — so one
ZIP renders either language without user configuration. The editor preview passes the
chosen preview language through `rw_format_time`/`rw_format_date` so it matches.

---

## 9. Code generation

`codegen.rs` has three public entry points:
`generate_app_json` (`:104-155`), `generate_app_js` (`:157-159`),
`generate_index_js` (`:166-168`) → `generate_index_js_with_hands` (`:172-464`).

### 9.1 `app.json`

Pretty JSON with `configVersion:"v3"`, `app:{appId, appName, appType:"watchface",
version:{code:1,name}, icon:"icon.png", vender:<author> (ZeppOS spelling), description}`,
`permissions:[]`, `runtime.apiVersion:{compatible:"1.0.0",target:"1.0.1",minVersion:"1.0.0"}`
and **no `runtime.type`** (`type:2` would mean `.bin` bytecode and block our `.js`),
`targets.active2round` with `watchface:{path:"watchface/index",main:1,editable:1,lockscreen:1}`,
`platforms[0].deviceSource=8913153`, `designWidth=466`, `i18n["en-US"|"pl-PL"].appName`,
and `defaultLanguage:"en-US"` (zeus rejects packages without it). `app.js` is a static
`App({globalData:{},onCreate(){},onDestroy(){}})`.

### 9.2 `watchface/index.js`

Starts with a generated header, the locale block (§8.3), `const p2 = …`, and only the
helper functions actually needed (weekday, ISO week, day-of-year, mileage unit,
world clock, health getters, `_fmtDuration`, optional user `custom_code`).

`font_cache_js` (`:470-506`) emits one offscreen `hmUI.widget.TEXT` per distinct
`(font, text_size)` pair with ASCII 32–126 plus `ĄĆĘŁŃÓŚŹŻąćęłńóśźż°€` — without this
warm-up, characters that first appear after a `setProperty(TEXT)` update render blank.

`build()` (`:318-332`) calls the warm-up, `_initMileageUnit()` when needed, then
creates one widget per `ordered_widgets()` entry in layer order via `widget_call`
(`:638-728`). Text widgets get `text`, `text_size`, `font:"fonts/<file>"`, raw ARGB
`color`, `align_h`, `align_v`, `text_style` and `show_level`; image/Images widgets get
`hmUI.widget.IMG` with `src`; analog widgets get four IMG calls
(`analog_widget_call`, `:621-636`) or `null` for hidden hands.

`tick()` (`:407-428`) reads `TIME` and writes per-widget updates (`update_line`,
`:730-814`):
- Time → `time_format::js_expression_localized`; Date → `date_format::js_expression_localized` on `new Date()`;
- weekday → `_weekdayNumber(t)`, week → `_isoWeek()`, day-of-year → `_dayOfYear()`;
- lunar → `t.lunar_month/lunar_day`; festival → `solar_festival || lunar_festival || lunar_solar_term`;
- world clock → `_getWorldClock().getWorldClockInfo(0)` city + `HH:MM`;
- steps/distance → sensor values with the resolved per-widget unit;
- health → `healthTick` with one lazy getter per sensor;
- analog hands → `angle = (h%12)*30 + m*0.5`, `m*6 + s*0.1`, `s*6` via
  `setProperty(hmUI.prop.MORE, …)`; cap is static.

Cadence (`:429-435`): if any visible time pattern has seconds (or an analog widget has
a second hand), `tickTimer = timer.createTimer(0, 1000, tick)`; otherwise a
`hmSensor` `MINUTEEND` listener. `onDestroy()` (`:436-460`) stops the timer, removes
step/distance/health/world-clock listeners, and nulls all globals. The entire
`WatchFace({onInit,build,onDestroy})` call is wrapped in `try/catch` that logs errors.

Health sensors (`:26-102`): `HEART` (CURRENT, `bpm`), `STRESS` (CHANGE), `PAI`
(`dailypai`), `BODY_TEMP` (`toFixed(1)` + `°C`), `SLEEP` (`getTotalTime()` → `H:MM`),
`CALORIE` (rounded + `kcal`), `SPO2` (`%`). Every getter is lazy and `try/catch`-guarded
so a missing sensor never kills the watchface; `show_unit` controls the suffix.

Distance units (`distance_value_js`, `:550-565`): `Watch` reads
`hmSetting.getMileageUnit()` once at build (`_initMileageUnit`, `:246-253`) and emits
`autoMileage ? 'mi' : 'km'`; explicit `Metric`/`Imperial` emit fixed `/1000` or
`/1609.344`.

---

## 10. Packaging, export and emulator

`export_zip` (`package.rs:17-80`): load → `validate` → `prepare_hand_assets` → write
`app.json`, `app.js`, `watchface/index.js` → for each `project.assets` write
`assets/active2round/<asset.path>` with byte preparation priority:
prepared hand bytes → `images::export_asset_bytes` for `images/…` →
`hands::export_asset_bytes` (raw pass-through for fonts/backgrounds/other) →
Deflate ZIP. `export_zip_bytes` (`:155-192`) does the same hand preparation but
**omits** the `images/…` branch and the Icon-path check — a known asymmetry.

`inspect_zip` (`:137-153`) returns an entry listing plus the raw `app.json`.

`prepare_emulator_workspace` (`:93-135`) exports to
`%APPDATA%\RustyWatch\Emulator\app-<appId>\watchface.zip`, deletes any previous
directory, unpacks into `app/`, and returns `{dir, app_dir, zip, log}`.

The C# side (`ZeppSimulator.cs`) finds `simulator.exe` in known paths and registry
uninstall keys, checks `%APPDATA%/simulator/config.json` for `selectDeviceList`
(array **or** object) containing device `8913153` with `status >= 2`, checks
`simuOpen == 1 || firmwareSimulatorIsReady`, finds `zeus` on PATH, and runs
`zeus install --path "<appDir>" --target "Amazfit Active 2 NFC (Round)"`. Both a
non-zero exit code **and** `[ERROR]` in the log count as failure (zeus can print
`[ERROR]` while exiting 0). If the simulator is absent, the app only opens the
official download page — it never bundles or installs Zepp binaries.

---

## 11. WinUI 3 front-end

### 11.1 Startup (`AppPaths.cs`, `MainWindow.xaml.cs:362-447`)

`AppPaths.Initialize()` creates nine directories under `%APPDATA%\RustyWatch`
(data root, Projects, Emulator, Fonts, Images, Backgrounds, Hands, Localization,
`<exe>/FontCache`), seeds missing localization files and merges missing keys into
existing ones (`SeedLocalization`/`MergeMissingKeys`, `:85-149`), and migrates only
`settings.json`, `recent.json` and JSON catalogs from `%LOCALAPPDATA%` with roaming
winning (`MigrateFromLocalAppData`, `:151-199`).

Constructor order: `AppPaths.Initialize()` → `rw_ensure_defaults` →
`LocalizationService.Initialize()` → `LocalizationBridge` → `InitializeComponent()`
→ outline pointer handlers → footer `AddHandler(TappedEvent, …, handledEventsToo:true)`
→ DataContext → `AppWindow.Resize(1280,800)` / icon / Mica → 1-second
`DispatcherTimer` → load all option lists and asset stores → `ShowView(editor:false)` →
`_initialized=true`. Handlers guard with `if (!_initialized) return;` to avoid the
XAML `SelectionChanged` crash during initialization.

### 11.2 Preview rendering (`MainWindow.xaml:299-322`, `MainWindow.xaml.cs:1171-1313`)

A `Viewbox` wraps a 466×466 `Grid`: black `Ellipse` + rounded `Border` clip the round
screen; `WatchCanvas` holds widgets in export order; `OverlayCanvas` holds the 16 px
grid and dashed red guides; `OutlineCanvas` sits **outside** the round clip so
handles on a 400×400 frame remain grabbable even where the frame falls outside the
circle.

`PopulateWatchface` (`:1188-1307`) skips hidden widgets exactly like
`ordered_widgets`; loads bitmaps from `<project>/assets/<src>` with `LoadFreshBitmap`
(`:1145-1151`, a stream — not a URI, because WinUI caches by URI and would keep the
old `bg.png`); renders analog hands as `Image` + `RotateTransform` around `(0.5,0.5)`;
and creates a transparent `Border` frame with four 10×10 corner handles for every
widget.

`ApplyFrameGeometry` (`:651-710`) sets `Host`/`Frame` to `w×h`, keeps the `TextBlock`
**content-sized** (`Height=double.NaN`, `MaxHeight=h`, `Width=w`) and always ends with
`ApplyTextLayout` (`:623-635`), the only place that sets horizontal/vertical anchoring.
A fixed `Height` would leave a stale bounding box after scaling and make text escape
the canvas. `UpdateExistingLayerViews` (`:2203-2251`) copies state into the same view
objects when only geometry changed, preventing list jumps and preview flashes.

`ClockTimer_Tick` (`:2274`) → `UpdateClockText` (`:783-799`) formats text through Rust
(`FormatDate`, `FormatDateField`, `FormatTime`) and rotates hands with the same formulas
as the generated JS.

### 11.3 Gestures and selection

`Frame_PointerPressed` (`:1327-1364`) selects the layer, toggles scale mode on a
second click within 450 ms, and captures the pointer on `OutlineCanvas`.
`OutlineCanvas_PointerMoved` (`:1366-1429`) builds candidate lines `{0,233,466}` plus
all other visible widgets' edges/centers, sends a JSON payload to `rw_frame_geometry`,
and applies only the returned values; `UpdateGuides` (`:1432-1455`) draws the guides.
`PersistSettings()` saves on release. Selected frames get `ZIndex=100` (handles 101)
so dragging works even under a visually higher layer.

The Layers list (`LayerRow`, `:73-124`) reverses the order for top-first display,
shows a native lock glyph for the background (no eye/arrows/remove), and greys out
hidden widgets (`Opacity=0.45`). `LayersList_SelectionChanged` (`:899`) drives
`LoadWidgetSettings()` (`:952-1139`), which toggles the digital/analog/images/text/
background panels, presets, and hand checkboxes. X/Y values above the sliders copy to
the clipboard on single tap and become editable on double tap (`:2328-2416`);
centering buttons use `(466−w)/2` / `(466−h)/2` (`:2427-2455`).

### 11.4 Assets, settings, navigation, export dialog

- **Assets** (`MainWindow.xaml:817-1083`): four ListViews (Fonts, Backgrounds, Images,
  Hands) with add/remove/open-folder buttons, Windows `FilePicker`/`FolderPicker`
  initialised via `InitializeWithWindow`, and live thumbnails ("Aa" for fonts).
  Built-in backgrounds are protected in UI (`IsBuiltin`) and Rust (`is_builtin`).
- **App Settings** (`MainWindow.xaml:1133-1238`): language + reload, watchface preview
  language, default author, projects folder (the `TextChanged` handler must stay
  wired), measurement system and first day of week; `AppSettingsStore.Save()` writes
  all keys at once so saving one setting never clobbers another.
- **Navigation footer**: two plain `NavigationViewItem`s with `Tapped` handlers
  (`handledEventsToo:true`); clicking one sets `NavView.SelectedItem = BlankNavItem`
  (tag `none`, never `null`) and highlights via `SetFooterHighlight`. Pane lengths
  `210`/`52` are deliberate (Polish labels + collapsed-icon workaround).
- **Export dialog** (`MainWindow.xaml:1297-1332`) is a direct child of `RootGrid` and
  contains only project details + one **Export ZIP** button — no preview, GIF, progress
  bar or background job. `ExportFileName` (`:3447-3466`) sanitizes the name the same
  way as folder names (forbidden characters → `.`).

### 11.5 Localization and preview fonts

`StringCatalog` (`StringCatalog.cs`) is a case-insensitive dictionary with English
fallback and `Refresh()` raising `"Item[]"` so `{Binding Strings[Key]}` re-evaluates.
`LocalizationService` (`LocalizationService.cs`) discovers catalogs from filenames in
the appdata folder, skips malformed JSON, and `Error(raw)` maps ~50 Polish Rust error
prefixes to localized keys while preserving trailing details. `LocalizationBridge.Refresh()`
notifies `Strings` and the New/Save-as dialog titles.

`PreviewFonts.Get` (`:37-80`) copies the TTF to `<exe>/FontCache` when missing/stale
and returns `new FontFamily("ms-appx:///FontCache/<file>#<family>")` — unpackaged
WinUI cannot load `%APPDATA%` or `file:///` fonts. The inspector passes the project's
own `assets/fonts` copy so the preview uses exactly the exported TTF.

### 11.6 Build orchestration (`RustyWatch.csproj`)

`TargetFramework=net8.0-windows10.0.26100.0`, `UseWinUI=true`,
`WindowsPackageType=None`, `WindowsAppSDKSelfContained=true`, `RuntimeIdentifier=win-x64`.
`PackageReference`: `Microsoft.WindowsAppSDK 2.7.0` and
`Microsoft.Windows.SDK.BuildTools 10.0.26100.4654`. The `BuildRustFfi` target runs
`cargo build -p rusty-ffi` before build; `CopyRustFfi` copies `rusty_ffi.dll` after
build with `SkipUnchangedFiles="false"` because `PreserveNewest` can leave a stale DLL.
Expected result on both toolchains: 0 errors / 0 warnings (109 tests total: 84 core + 25 FFI).

---

## 12. Cross-cutting invariants and learned traps

These are hard-won constraints; changing them breaks 1:1 preview/export parity.

1. **Classic API only.** Generated JS uses `hmUI.createWidget`, `hmSensor`, `timer`
   and global `WatchFace({...})`; never `@zos/ui` or `@zos/sensor` (Mini Programs).
   With API 3.x the runtime does not expose `hmUI` and the watchface renders black.
   `app.json` therefore uses `runtime.apiVersion` 1.0.0/1.0.1 and no `runtime.type`.
2. **One order.** Preview and JS both use `layers::ordered_widgets`; hidden widgets are
   filtered there and only there. The background is locked in `move_layer`,
   `remove_widget` and `set_widget_visibility`, and cannot be moved below by any widget.
3. **No runtime image scaling.** ZeppOS draws an `IMG` at the PNG's natural size and
   clips it to `w×h`. Backgrounds are prepared to 466×466, images to `w×h`, and hand
   PNGs are physically resized; the export-time safety nets cover older projects.
4. **Square hand crops only.** `prepare_hand_assets` crops a centred square around the
   pivot, never a tight alpha bbox — a rotated hand would otherwise be clipped to stubs.
5. **Shared time/date algorithms.** `time_format.rs` and `date_format.rs` are the sole
   source for both Rust preview strings and generated JS expressions; weekday index
   uses Sakamoto, never `t.week`. Monday-first is the default and stored per project.
6. **Content-sized TextBlock.** Never set an explicit `Text.Height`; call
   `ApplyTextLayout` at the end of every geometry application.
7. **Fresh bitmaps.** Reload `bg.png` from a stream after `rw_set_background`; URI
   caching shows the old image.
8. **FFI discipline.** Every new `rw_*` export must be added in both `rusty-ffi/src/lib.rs`
   and `RustNative.cs`; callers must free with `rw_free_string`; bool marshalling needs
   an explicit decision (`I1`, `i32`, or JSON).
9. **Localization seeds are not runtime sources.** Bundled `Localization/*.json` only
   seed `%APPDATA%`; every user-facing string must exist in both `en.json` and `pl.json`
   and come from a catalog, and all catalogs must stay valid JSON.
10. **No debug hooks** (`RW_DEBUG_*`) in shipped code.
11. **`.zip` only**, never `.zpk`. No SashaCX75 code or assets — format inspiration only.

---

## 13. File index

| Path | Role |
|---|---|
| `crates/rusty-core/src/model.rs` | `Project`, `Widget`, all enums, constants, starter template |
| `crates/rusty-core/src/project.rs` | create/load/save/save-as, folder/name sanitization, App ID, PNG generators |
| `crates/rusty-core/src/layers.rs` | order, descriptors, background lock, visibility, `ordered_widgets` |
| `crates/rusty-core/src/widgets.rs` | catalog, add/update, frame geometry math |
| `crates/rusty-core/src/codegen.rs` | `app.json`, `app.js`, `watchface/index.js` |
| `crates/rusty-core/src/time_format.rs`, `date_format.rs`, `locale.rs` | shared formatting/parsing, Sakamoto, runtime language tables |
| `crates/rusty-core/src/{fonts,backgrounds,images,image_fit,hands}.rs` | asset stores and scaling |
| `crates/rusty-core/src/package.rs` | ZIP export/inspect, emulator workspace |
| `crates/rusty-core/src/validate.rs`, `recent.rs` | validation, recent projects |
| `crates/rusty-cli/src/main.rs` | `new`, `validate`, `export`, `inspect` |
| `crates/rusty-ffi/src/lib.rs` | 54 `rw_*` C exports + 25 FFI tests |
| `native/RustyWatch/MainWindow.xaml(.cs)` | shell, preview, layers, inspector, assets, settings, dialogs |
| `native/RustyWatch/RustNative.cs` | P/Invoke bridge and DTOs |
| `native/RustyWatch/{AppPaths,AppSettingsStore,PreviewFonts,ZeppSimulator}.cs` | app data, settings, TTF preview, simulator integration |
| `native/RustyWatch/Localization/*` | catalogs, service, bridge, format guide |
| `native/RustyWatch/Assets/*` | icon, HOME logo, Inter TTF + OFL, hand sets |
| `logo/` | vector and raster logo sources |
| `docs/FORMAT.md`, `PLAN.md`, `AGENTS.md` | format spec, history/decisions, agent rules |
