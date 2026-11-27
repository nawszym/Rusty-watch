# AGENTS.md — Rusty Watch

Instructions for AI coding agents working in this repository. This is the **single source of
truth**; `CLAUDE.md` and `.github/copilot-instructions.md` point here. Keep this file current
when architecture, build commands or hard-won workarounds change.

## 1. What this project is

Rusty Watch is a native Windows 11 watchface editor for the **Amazfit Active 2 (Round)**.
It creates projects, previews them 1:1, and exports installable **`.zip`** watchface packages
for ZeppOS / amazwatchfaces.

- The only UI is **WinUI 3** (Windows App SDK **2.7.0**), C#/XAML, in `native/RustyWatch/`.
  The former Tauri + React prototype in `ui/` was removed — do not reintroduce a web frontend.
- Business logic and code generation live in a Rust workspace in `crates/`, exposed to C#
  through a C ABI.
- Target device only: **Amazfit Active 2 (Round)**, 466 × 466, ZeppOS 4.x,
  `deviceSource: 8913153` (`DeviceId::Active2Round` in `model.rs`).
- Export **`.zip` only** — never `.zpk`.
- Licence: **GPL-3.0-or-later + Generation Exception**. Generated watchface output is *not*
  subject to GPL; editor source and its derivatives must stay open source. Do not copy
  SashaCX75 source code or third-party assets (inspiration/format only).
- Historical context and decisions: `PLAN.md` (section 8 = confirmed decisions), `README.md`,
  `docs/FORMAT.md`. Read them before changing scope or data formats. The README is English
  only — there is no Polish README and no `Readme.base` file.

## 2. Repository map

| Path | Role |
|---|---|
| `Cargo.toml` | Workspace: `rusty-core`, `rusty-cli`, `rusty-ffi`. |
| `crates/rusty-core/src/` | `model.rs` (Project/Widget/metadata, device constants, starter template), `project.rs` (create/load/save/save-as, folder sanitization, random App ID), `codegen.rs` (generated `app.json`, `app.js`, `watchface/index.js`), `time_format.rs` (shared pattern parser + JS expression builder), `layers.rs` (layer order + editor descriptors), `widgets.rs` (add/edit/remove widget, frame geometry), `package.rs` (ZIP export/inspect), `fonts.rs` (font store, TTF name parser, attachment/pruning), `recent.rs`, `locale.rs` (PL/EN watchface language tables + runtime JS detection), `images.rs` + `image_fit.rs`, `hands.rs`, `status_icons.rs`, `weather.rs`, `asset_cache.rs` (fingerprints/bundled-version markers), `export_paths.rs` (hashed ZIP names), `date_format.rs`, `qr.rs` (pure-Rust QR PNG), `preview.rs` (8-frame PNG/GIF renderer, branded composition), `validate.rs`. |
| `crates/rusty-cli/src/main.rs` | CLI: `new`, `validate`, `export`, `inspect`. |
| `crates/rusty-ffi/src/lib.rs` | C ABI, **66** `rw_*` exports (65 JSON-returning + `rw_free_string`); all take/return UTF-8 JSON; caller frees with `rw_free_string`. |
| `native/RustyWatch/MainWindow.xaml(.cs)` | Shell, HOME, Editor, Assets, App Settings, bottom bar, dialogs. |
| `native/RustyWatch/RustNative.cs` | P/Invoke bridge (`DllImport`, `CallingConvention.Cdecl`, `LPUTF8Str`) with one central `Call` helper. |
| `native/RustyWatch/PreviewFonts.cs` | Copies selected TTFs into `<exe>/FontCache` and builds `ms-appx:///FontCache/...` families so the preview uses the exact exported font. |
| `native/RustyWatch/AppPaths.cs` | Creates `%APPDATA%\RustyWatch` layout and seeds bundled localization. |
| `native/RustyWatch/Assets/` | Bundled `Fonts` (Inter + Noto Sans + OFL), `Hands` (Classic, Light), `Status` (Classic), `Weather/system-like` (default, 29 icons) + `Weather/Not Something`, `Icons/click-zone.png`, `AppIcon.ico`, `Logo.png`. |
| `native/RustyWatch/Localization/` | Bundled `en.json` / `pl.json` seeds, `README.md`, `LocalizationService`, `StringCatalog`, `LocalizationBridge`. Runtime copies live in `%APPDATA%\RustyWatch\Localization`. |
| `native/RustyWatch/RustyWatch.csproj` | Build orchestration: builds Rust FFI, copies DLL, catalogs and fonts. |
| `examples/first-watch/` | Example project (gitignored). |
| `reference-ui.png` | Local-only visual reference for the shell layout; gitignored — do not commit. |

## 3. Build, test, run

Workspace root: this repository directory (Windows, PowerShell).

```powershell
cargo test --workspace

$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH   # required in this shell
dotnet build .\native\RustyWatch\RustyWatch.csproj
.\native\RustyWatch\bin\Debug\net8.0-windows10.0.26100.0\win-x64\RustyWatch.exe
```

CLI smoke test:

```powershell
cargo run -p rusty-cli -- new .\test-tarcza --name "Test" --author "Dev" --app-id 12345678
cargo run -p rusty-cli -- validate .\test-tarcza\project.json
cargo run -p rusty-cli -- export .\test-tarcza\project.json
cargo run -p rusty-cli -- inspect .\test-tarcza\Test.zip
```

`dotnet build` already runs `cargo build -p rusty-ffi`. Expect **0 errors / 0 warnings** on
both toolchains; treat new warnings as regressions. Current suite: **208 tests - 170 core +
38 FFI**. Prerequisites: .NET SDK 10 + Runtime 8, Rust, Windows App SDK via NuGet
(self-contained app).

## 4. Architecture rules (do not violate)

1. **Layering:** `rusty-core` has zero UI dependencies; all logic is pure/testable.
   `rusty-cli` is a thin wrapper. `rusty-ffi` is the only bridge. C# contains no business
   logic — it calls `rw_*`, parses JSON and updates the UI.
2. **FFI contract:** every `rw_*` function (except `rw_free_string`) returns a JSON UTF-8
   string of shape `{"ok":true,"data":…}` or `{"ok":false,"error":"…"}`, allocated by Rust;
   C# frees it with `rw_free_string` in a `finally`. Strings cross as NUL-terminated UTF-8
   (`[MarshalAs(UnmanagedType.LPUTF8Str)]`); the one `bool` parameter is `UnmanagedType.I1`.
   Add new functions in **both** `crates/rusty-ffi/src/lib.rs` and
   `native/RustyWatch/RustNative.cs`.
3. **New feature flow:** implement in `rusty-core` → add tests → expose via `rusty-ffi` →
   wire into `native/RustyWatch` → run `cargo test --workspace` **and** `dotnet build`.
4. **The watchface uses the classic Watchface API** `hmUI.createWidget(hmUI.widget.TEXT/IMG/…)`,
   `hmSensor`, `timer` and global `WatchFace({...})` — **not** `@zos/ui` / `@zos/sensor`
   (those are Mini Programs; with API 3.x the runtime does not expose `hmUI` and the watchface
   renders black — verified in the simulator). `generate_app_json` emits
   `runtime.apiVersion` **1.0.0 / 1.0.1** and **no** `runtime.type` (`type: 2` means `.bin`
   bytecode and blocks loading our `.js`). Seconds in the time pattern ⇒
   `timer.createTimer(0, 1000, tick)` + `timer.stopTimer`; otherwise a `hmSensor` `MINUTEEND`
   listener. Listeners are cleaned up in `onDestroy`. The export **always** emits
   `hmUI.show_level.ONLY_NORMAL` (`codegen::JS_SHOW_LEVEL`), even for projects that still store
   `Always`: `ONAL_AOD` made real firmware paint an opaque AOD overlay. AOD is driven by
   `WIDGET_DELEGATE` + `hmSetting.getScreenType()` instead. New widgets default to
   `ShowLevel::Normal`. Align / text-style / show-level JS expressions live in `model.rs::as_js()`
   with an `hmUI.` prefix.
   4a. **The watchface detects its own language at runtime.** `crates/rusty-core/src/locale.rs`
   owns the PL/EN tables (Polish month/weekday names, AM/PM markers, 29 weather condition names)
   and emits a JS block calling `DeviceRuntimeCore.HmUtils.getLanguage()` with a guarded
   `hmSetting.getLanguage()` fallback — `9` = `pl-PL`, **every other value = English**.
   `codegen` uses `L.months` for Date widgets and `time_format::js_expression_localized`
   (`L.am`/`L.pm`) for time widgets; the JS locale block includes Polish diacritics directly in
   the initial widget text. Do not bake one language into the ZIP. The editor has a
   **watchface preview language** setting (`pl`/`en`, default `en`, no Windows-culture `auto`)
   passed through `rw_format_time` / `rw_month_name` and to generated preview frames.
   4b. **Generated previews are Rust-only and deterministic per export.**
   `crates/rusty-core/src/preview.rs` owns 7 hardcoded presets plus an exact copy of the
   21:37:40 frame (`FRAME_PRESETS`; hours 06–21, minutes/seconds strictly increasing;
   `SOURCE_FRAME_COUNT = 7`, `FRAME_COUNT = 8`, `PREVIEW_FRAME_INDEX = 7`), the random
   `PreviewRandomSeries` (7 real dates 2022-01-03 … 2022-07-31 — Monday…Sunday, January…July),
   and the renderer. Steps derive from distance (`distance_m/1000 * steps_per_km`, 1300–1400);
   battery decreases 100% → 10%. Every date widget in a frame uses that frame's single date
   (Sakamoto weekday, `project.first_day_of_week`, ISO week, day-of-year, `locale.rs` names).
   `FRAME_PRESETS` carries a `status_mask` (ALARM=1, DND=2, BLUETOOTH=4, LOCK=8) with sequence
   `2,2,0,3,0,1,4,4`: frame 7 (21:37:40) shows all four and frame 8 is its exact copy.
   The static `preview.png` is **not a raw frame**: `compose_branded_preview()` draws the
   branded frame around the 21:37 frame — transparent outside; the `logo/gear-outlayer.png`
   brown ring with gear teeth reaches almost the edge (`R_BROWN_OUT = 0.97`, `R_EDGE = 0.99`);
   the full watchface fills the centre (`R_FACE = 0.79`) with only a thin black gap to the ring
   and a `#404040` outline. There is no thick outer black band. `preview.gif` has 8 frames at
   ~1.7 s each, infinite loop;
   the **composed PNG is the first GIF frame**, followed by the 7 raw frames. Shortcut hatches
   are drawn only on the second (first raw) frame via `draw_shortcut_overlays()`. Neither file
   is zipped. Custom Data from the editor is deliberately ignored. Never render preview
   frames with offscreen WinUI controls.
   4c. **Generated JS must never use `Date`.** The classic runtime on a real watch does not
   expose `Date` (the simulator does, and calling it in `build()` leaves a black screen).
   `date_format.rs` emits `JS_DATE_FROM_T`, `JS_WEEKDAY_INDEX`, `JS_ISO_WEEK` and
   `JS_DAY_OF_YEAR` using only arithmetic on fields `d`/`m`/`m0`/`y`/`w` derived from the
   TIME sensor; a regression test bans `Date`/`getDate`/`getMonth`/`getDay`/`getFullYear`/`UTC`
   in generated JS. Text widgets emit `char_space: 0`, `line_space: 0` and
   `font: "fonts/font_<hash>.ttf"` exactly like the working reference.
5. **Layers are the single source of draw order.** `crates/rusty-core/src/layers.rs` resolves
   `Project::layer_order` (bottom → top, **widget ids only**, `z` kept in sync) and exposes
   descriptors for the editor. `codegen::generate_index_js` and the WinUI preview must both use
   `layers::ordered_widgets()` / `layers::describe()` so the preview matches the exported
   watchface. `ordered_widgets` skips hidden widgets. **There is no background layer and no
   background asset** — the background is a constant
   `hmUI.widget.FILL_RECT(x:0,y:0,w:466,h:466,color:0xFF000000)` emitted before all widgets.
   Do not order by vector index or `z` separately in C#. `rw_move_layer` saves immediately.
   Widget creation/editing/removal lives in `crates/rusty-core/src/widgets.rs`. The last
   remaining widget cannot be removed. The left **Widgets tab is a read-only catalog**
   (`rw_supported_widgets`) — it must not contain an Add button. Adding happens from the
   **+ Add widget** button on the right of the editor's Preview bar, whose Flyout has a search
   box and collapsible category groups (collapsed by default; search auto-expands matches).
   The inspector is driven by the layer selected in Layers
   (`LayersList_SelectionChanged` → `LoadWidgetSettings`), never hard-wired to the first time
   widget. Keep every bundled `Localization/*.json` valid JSON — an unescaped `"` silently
   drops the catalog.
6. **Text widget frames (ramka) and scaling.** Every text widget is drawn in the preview as a
   transparent `Border` at exactly `x/y/w/h` with four corner resize handles. `rw_frame_geometry`
   is the single source of truth for gesture math (clamps to 466×466 or `maxSize`, minimum 24 px,
   text size 8–160) and is called on every pointer move; C# only applies returned values. The
   weather icon passes `maxSize = 256` (the slider maximum), so dragging and the slider share one
   limit. `snap` applies the
   16 px grid; `alignX`/`alignY` candidates produce dashed red `guideX`/`guideY` — alignment
   wins over the grid, resolved on the **pre-grid** position so centre snapping works with the
   grid on. Double-click toggles `Widget::auto_scale`: the frame turns red and resizing scales
   `text_size` proportionally; the real value is persisted so preview and export stay 1:1.
   `text_style` (`ellipsis`/`none`/`wrap`) maps to ZeppOS `text_style`; `v_align`
   (`top`/`center`/`bottom`) maps to `align_v`. The `TextBlock` must stay **content-sized**
   (`Height = double.NaN`, `MaxHeight`/`Width` = current frame size) inside a host `Grid`; a
   fixed `Height` leaves a stale anchor and text escapes after scaling. Horizontal/vertical
   alignment is applied in the single `ApplyTextLayout` helper called at the end of every
   `ApplyFrameGeometry`. `ApplyLayersPayload` rebuilds the list/preview only when layer identity
   or order changes; geometry edits update existing `LayerRow`/`FrameView` objects in place
   (`UpdateExistingLayerViews`), otherwise the list jumps and the preview flashes black.
7. **Fonts are one shared asset store, but projects are self-contained.** Installed fonts live
   in `%APPDATA%\RustyWatch\Fonts`; the bundled default is `Inter-Regular.ttf` (SIL OFL 1.1).
   `Widget.font` stores only the file name; `None` means the watch system font. **Zepp OS's
   default system font is Noto Sans** (official Zepp docs). Codegen omits `font:` for `None`,
   the editor preview uses the bundled, Latin/PL-subset `NotoSans-Regular.ttf` from the official
   Zepp CDN (SIL OFL 1.1, `Assets/Fonts/CREDITS.md`), and `preview.rs` uses the same TTF for
   generated frames — so the system-font choice is 1:1 on the watch, in the editor and in the
   generated PNG/GIF. Never fall back to Segoe UI for that choice. Assigning a custom font
   copies it into `<project>/assets/fonts/<file>` and registers `Asset { role: Font }` so
   Save as / ZIP export include it; `prune_unused` removes old copies. The generated JS sets
   `font: "fonts/font_<hash>.ttf"` exactly like the working reference. There are deliberately
   **no** offscreen glyph warm-up widgets. **New projects and newly added widgets leave
   `font` unset**, so they use the watch system font by default; Inter is only applied when
   the user explicitly picks it in the inspector. `rw_set_widget` with `font: ""` **clears**
   the selection to system font (a missing `font` key keeps the previous choice); do not
   auto-fill null fonts when
   opening a project or adding a widget — that used to destroy the system-font selection.
   Custom fonts are never overwritten. WinUI unpackaged apps cannot load `%APPDATA%` fonts
   directly — `PreviewFonts.cs` copies TTFs next to the exe and uses `ms-appx:///FontCache/...`
   families. Never set `FontWeight` on preview text (a synthesized bold breaks dot-matrix fonts).
8. **No background store.** No `Backgrounds` directory, no `bg.png`, no `background` field in
   `project.json`, no `AssetRole::Background`, no background layer/controls and no
   `rw_backgrounds_*` / `rw_set_background` FFI.
9. **Images.** The Images store is `%APPDATA%\RustyWatch\Images`. The **Images widget**
   (`WidgetKind::Images`, catalog kind `images`) uses the shared `image_fit` module:
   `rw_set_widget_image` copies the chosen file into `<project>/assets/images/<widget-id>/<file>`,
   physically resized to `w × h` with `FitMode` (`cover`/`contain`/`stretch`, default `contain`).
   The blue frame is auto-fitted to the image's natural size with one uniform scale clamped to
   466×466 and only moves the widget (corner handles hidden, `handle: "move"`). Double-click
   toggles the red frame, which enables corner handles and **always stretches** the PNG while
   resizing (`rw_set_widget` forces `image_fit = stretch`). `package.rs` applies
   `images::export_asset_bytes` as a safety net for older projects.
10. **Analog time and hand sets.** Addable kinds come from `widgets::supported_widgets()` as
    `{kind, category}`. An analog widget is one layer whose JS is a group of rotating
    `hmUI.widget.IMG` widgets (hour/minute/second/cap), each with a **full-screen frame**
    (`x:0,y:0,w:466,h:466`), `pos_x/pos_y` from the prepared placement, `center_x/center_y = 233`,
    `auto_scale: false`, `angle` updated through `setProperty(hmUI.prop.MORE, …)` from the
    shared tick; the cap is static. Second hand ⇒ 1 s timer, otherwise MINUTEEND.
    Installed sets live in `%APPDATA%\RustyWatch\Hands\<id>\` (`hands`: list/add/remove/
    ensure_default); a set needs square `hour.png` + `minute.png` of equal size, optional
    `second.png`/`cap.png`. Bundled **Classic** (with subtle shadow) and **Light** come from
    `native/RustyWatch/Assets/Hands/`; `ensure_default` seeds **every** valid bundled set and
    skips already-installed ones (never return early after Classic). Selecting a set copies
    files to `<project>/assets/hands/<id>/`, registers `AssetRole::Hand` and prunes unused sets.
    A new widget gets its frame centred and sized from the PNGs; changing the set on an
    existing widget preserves `x/y/w/h`, then `hands::scale_project_hands` re-scales the PNGs.
    Keep hand bitmaps smaller than the screen (bundled = 400×400): full-screen hand textures
    cost ~868 KB each and can reboot real firmware. `prepare_hand_assets` uses one common
    scale for the set, then a **tight alpha bounding box with an 8 px transparent margin** and
    emits per-hand `x/y/w/h`; both export paths must call it. **Transparent pixels must have
    `RGB=(0,0,0)`**: bundled Classic has white RGB under `A=0`, which real firmware paints as
    white rectangles when rotating an `IMG`. `hands::scale_hand` zeroes RGB at `A=0` and
    resamples in **premultiplied alpha** space; never reintroduce straight-alpha Lanczos3.
    The inspector has one checkbox per hand; `rw_set_hand_visibility` clears `*_src` to hide
    and re-copies from the store to re-enable; visibility survives set switches. Codegen emits
    `null` for hidden hands; validation accepts any subset. The preview rotates `Image`
    elements with `RotateTransform` on the same clock timer. Frames/handles are drawn on a
    separate `OutlineCanvas` **outside the round clip** so 400×400 corners stay grabbable.
    Blue mode moves, red mode (double-click) scales; `rw_frame_geometry` receives `square: true`
    for analog layers so `w == h`.
11. **System widgets and STATUS icon packs.** The `system` category holds `battery`
    (`WidgetKind::BatteryText`, `hmSensor.id.BATTERY`) and four `status-*` layers
    (`WidgetKind::StatusIcon`) rendered by `hmUI.widget.IMG_STATUS` with
    `hmUI.system_status.CLOCK/DISTURB/DISCONNECT/LOCK`. ZeppOS toggles their visibility itself —
    no sensors, timers or listeners are generated. Packs live in
    `%APPDATA%\RustyWatch\Status\<id>\`; a pack is valid only with **all four** square PNGs
    `alarm.png`, `dnd.png`, `bluetooth.png`, `lock.png`. Only **Classic** is bundled (default,
    cannot be removed), seeded from `native/RustyWatch/Assets/Status/`. Selecting a pack copies
    it into `<project>/assets/status/<id>/`, registers `AssetRole::StatusIcon`, prunes unused
    packs and physically resizes the PNGs (`IMG` does not scale at runtime). New battery widget:
    `x=23,y=203,w=420,h=60`; new status icons form one centred row
    (`x=145/193/241/289,y=24,w=h=32`). Entering Assets must never block navigation: show the
    view first, refresh stores in `Task.Run`, marshal with `DispatcherQueue.TryEnqueue`, discard
    stale responses by a version counter. Rust stores `preview.fingerprint` +
    `.bundled-version`; thumbnails regenerate only when missing/changed. All font/image/hand/
    status tiles are GridView tiles with at most three per row. Bulk icon scaling never runs on
    the UI thread: interactive edits send `deferAssets: true` and `rw_rescale_widget_assets`
    runs in `Task.Run` with a 180 ms debounce behind `_assetGate`.
12. **Weather.** Shared packs in `%APPDATA%\RustyWatch\Weather\<id>\`, 29 icons `w00.png`…
    `w28.png` from `locale.rs` condition codes. Bundled packs are **`system-like`** (the default:
    512×512 ZeppOS-style vector reconstructions) and **Not Something** (512×512 circle-based
    rebuilds of the original 29 icons); both are centred in the files themselves, PNGs only,
    with provenance in `Assets/Weather/CREDITS.md`. Both are built-in, seeded on first launch and
    protected from removal. `weather::resize_png` resamples in **premultiplied
    alpha space and keeps premultiplied RGB in the saved PNG** — unpremultiplying restored white
    RGB at partial alpha, which real firmware painted as jagged white edges. Switching packs is
    instant: decode only the currently visible icon on the UI thread; the 29-file copy/resize
    runs in `rw_set_weather_pack` on the background queue behind `_assetGate` (120 ms debounce,
    cancellation + version counter) and export awaits it. `rw_set_weather_pack` relies on
    `attach_to_project` alone — do not add a second scaling pass. Generated JS caches
    `_lastWeatherIcon` and only calls `setProperty(hmUI.prop.SRC, …)` on change. **Day 0 (today)
    is different:** the condition text and the icon read the **current** weather code from the
    `curAirIconIndex` property of the `hmSensor.id.WEATHER` instance (`_currentWeatherCode()`)
    — the same source the ZeppOS weather widget and other watchfaces use. Night codes 26/27/28
    are derived locally from TIME and `tideData` (`_wxNightCode()`). The visible icon stays a
    plain `IMG` with manual `SRC`; days 1…4 still read `forecastData.data[i].index`.
    Parsing goes through `_weatherIndex()` (invalid → 25/"Unknown"), never `| 0`. If
    `curAirIconIndex` is unavailable (e.g. simulator), `_fallbackTodayCode()` uses today's
    forecast plus the same night mapping. The weather icon draws no text
    (`weather_text("weather-icon")` returns `""`). Conditions come from `rw_weather_conditions`;
    sunrise/sunset use one validated `HH:MM` field each. `weather-current` uses classic
    `hmUI.widget.TEXT_FONT` with `type: hmUI.data_type.WEATHER_CURRENT` and `unit_type: 0/1` —
    no sensor/timer/listener, no static `text:`. Every weather widget has `weatherDayOffset`
    (0…4); missing forecast days show `--`/`--°`/`--:--`. High/low and sunrise/sunset remain
    whole-day forecast values even for day 0.
    The weather-icon inspector has a **24…256 px SIZE slider** that keeps a square frame
    centred on the watch; dragging a corner of the red frame updates the slider and its px
    label live. Changing it persists geometry and re-scales all 29 icons of the
    selected pack from the pristine store copy through `rw_rescale_widget_assets`. Only packs
    with all 29 correctly named, square, equal-size PNGs are listed/usable; a valid pack added
    by hand to `%APPDATA%` gets its missing `preview.png` generated on the next list call.
    `rw_format_weather_field` never substitutes another pack's icon: a missing file yields an
    empty `iconPath` and the preview frame is cleared, so the editor never shows a different
    template than the selected pack.
13. **Shortcut zones.** `WidgetKind::Shortcut` exports `hmUI.widget.IMG_CLICK` with **`src: ""`**
    and `type: hmUI.data_type.<TARGET>` — no PNG asset is generated or zipped (a bundled 1×1
    transparent PNG was rejected by QR installs and by `zeus install`). No sensor/timer/listener.
    The editor draws a `#7DE2D1` diagonal hatch plus `Assets/Icons/click-zone.png`; icon
    (`min(w,h)/2`) and hatch step (`clamp(min(w,h)/4, 6, 24)`) scale with the frame. The
    branded first GIF frame stays clean; the second frame gets `draw_shortcut_overlays()`.
    **`codegen` creates all shortcuts last** (`creation_order`), after every other visible
    widget — on the watch the last-created widget receives the touch, so a higher visual layer
    can never swallow a shortcut tap. Layer order applies only between shortcuts themselves.
14. **Project and app storage.** One folder per watchface under the projects root:
    `<root>/<sanitized name>/project.json`. Never overwrite an existing folder. `metadata.name`
    is read-only in Project settings (folder and recent list depend on it; use Save as).
    `metadata.appId/appName/author/version` flow into generated `app.json`;
    `vender` is `zepp` and `description` empty for reference parity. App data lives in
    `%APPDATA%\RustyWatch` (roaming): `settings.json`, `recent.json`, default `Projects`, the
    shared `Fonts`/`Images`/`Hands`/`Status`/`Weather` stores, runtime `Localization` catalogs
    and the `Emulator` work dir. `AppPaths.Initialize()` creates the structure, seeds missing
    localization files (never overwrites user copies) and copies legacy files from
    `%LOCALAPPDATA%\RustyWatch` (roaming wins). `rw_ensure_defaults` (MainWindow constructor)
    seeds fonts, hands, STATUS and the bundled weather packs (`system-like`, Not Something).
    `AppSettingsStore` is the single writer for
    settings. Recent projects: `%APPDATA%/RustyWatch/recent.json`, max 10, BOM-tolerant,
    canonically a JSON **array**; `recent::load` also accepts PowerShell's
    `{"value":[…],"Count":N}` and a single object, and rewrites as an array on the next save.
    When editing that file from a script, serialize an array **explicitly**.
    Projects root is `AppSettingsStore.ProjectsDirectory` (default
    `%APPDATA%\RustyWatch\Projects`); New project / Save as create directly there. The
    TextChanged handler of that setting must stay wired in XAML.
15. **No debug hooks in shipped code.** Temporary `RW_DEBUG_*` startup switches were removed;
    do not reintroduce them.

## 5. Localization (PL / EN, ready for more)

- Bundled user-visible text lives in `native/RustyWatch/Localization/en.json` and `pl.json`
  under a `strings` object; XAML binds with `{Binding Strings[Key]}`. At runtime catalogs are
  read **only** from `%APPDATA%\RustyWatch\Localization`; `AppPaths.Initialize()` seeds missing
  files and never overwrites existing ones. Bundled files are *seeds*, not the live source.
- When adding a UI string: add the same key to **every** bundled catalog, then bind it in XAML.
  Use `LocalizationService.Format(key, args)` for runtime formatting and
  `LocalizationService.Get(key)` for plain text. `StringCatalog` falls back to English for
  missing keys. Keep all bundled JSON valid — an unescaped `"` drops the whole catalog.
- `LocalizationService.SetLanguage()` + `ApplyLocalization()` / `LocalizationBridge.Refresh()`
  is the live-switch path. New languages require only dropping `<code>.json` into the appdata
  folder and clicking **Reload languages** in App Settings — no code changes or rebuild.
  The catalog list is discovered from `*.json` filenames; malformed JSON is skipped.
- Rust diagnostics are canonically Polish. English is produced by the pattern map in
  `LocalizationService.Error`. When adding a Rust error message, add its pattern there and keys
  to all catalogs.
- Language choice persists in `settings.json`; default English (never Windows UI culture).
  The separate watchface preview language is also English by default and drives both the live
  preview and generated preview files. Assistant-facing docs/comments in this repo are largely
  Polish; keep new comments consistent with the surrounding file. User-facing strings must
  **always** come from catalogs.

## 6. WinUI 3 / Windows App SDK gotchas (learned the hard way)

Do not "fix" these by reverting to the obvious approach:

- **Never downgrade Windows App SDK below 2.7.0.** 1.7's XamlCompiler fails silently.
- `Window` has no `Width`/`Height`/`FontFamily`/`Resources` XAML members. Set size in code:
  `AppWindow.Resize(new Windows.Graphics.SizeInt32(1280, 800))`.
- `ApplicationBackgroundThemeBrush` does not exist in 2.7 and crashes `Microsoft.UI.Xaml.dll`
  with `0xc000027b`. Root backgrounds use `#1C1C20` / `#161619`.
- A `SelectionChanged` handler that runs during `InitializeComponent()` caused a `0x80004003`
  XAML crash. Guard event handlers with `if (!_initialized) return;`.
- `TreeView.Items` / XAML `Items` don't work here; use `ItemsSource` or avoid `TreeView`.
- `NavigationView` needs `IsSettingsVisible="False"` (App Settings is a custom footer item).
- A `ContentDialog` must be a **direct child of the root `Grid`**; declaring it in
  `Grid.Resources`/`Window.Resources` makes `ShowAsync()` hang or fails with `WMC0011`/`WMC0035`.
  `ContentDialog` has **no** `Opening` event in this SDK.
- `PreserveNewest` can leave a stale `rusty_ffi.dll`. The `CopyRustFfi` target uses
  `SkipUnchangedFiles="false"` — keep it.
- XAML indexer bindings (`{Binding Strings[Key]}`) work here; switching language is
  `LocalizationBridge.Refresh()` + `StringCatalog.Refresh()`. A flyout does not inherit the
  page `DataContext`, so flyout item labels are set in code (`RefreshPreviewOptionLabels`).

## 7. Current scope / placeholders

- **Startup guard:** `App.OnLaunched` initializes paths/localization, then probes
  `rusty_ffi.dll` with `NativeLibrary.TryLoad` **before** constructing `MainWindow` (the first
  `rw_*` call is `EnsureDefaults` in its constructor). If the library cannot load — typically a
  missing **VC++ Redistributable 2015–2022 (x64)** — `VcRedistWindow` shows a localized error with
  a link to `https://aka.ms/vs/17/release/vc_redist.x64.exe` and exits on Close. `ContentDialog`
  lives in that window's XAML Grid and is shown from `RootGrid.Loaded`, when `XamlRoot` exists.
- Fresh app opens **HOME**: New project, Open project, recent list, target-device card.
  Left nav: Home, Editor + Project settings (after opening), **Widgets** (always visible,
  even without a project), Assets, App Settings. Footer items do **not** raise
  `SelectionChanged`, so the constructor attaches `UIElement.TappedEvent` handlers with
  `handledEventsToo: true`; clicking a footer item sets `NavView.SelectedItem` to the hidden
  `BlankNavItem` (tag `none`) and calls `SetFooterHighlight` — never assign `null`.
  Footer items stay **plain `NavigationViewItem`s directly in the footer StackPanel** (no Grid
  wrappers/hand-drawn indicators). `OpenPaneLength="210"` (Polish labels were clipped at 172),
  `CompactPaneLength="52"` (workaround for clipped footer icons in collapsed pane).
- **Editor:** 1:1 round preview + Layers panel (top to bottom). Widget rows have eye / up /
  down / remove buttons in one horizontal stack. Hidden widgets stay in `project.json` and
  Layers (greyed out) but are skipped by preview and generated JS (`Widget::visible`,
  `rw_set_widget_visibility`). Double-clicking empty space clears selection. Selected layer
  gets highest `ZIndex` on `OutlineCanvas`. Inspector: format preset/box only when the catalog
  entry has `hasPattern: true` (`time`/`date` — `WidgetKind::uses_pattern()` is the Rust source
  of truth); sensor-backed widgets show a localized hint instead. Opacity slider applies to all
  kinds including analog/images/STATUS/weather; `alpha` persists on every `rw_set_widget` path.
  Font ComboBox lists system font plus installed fonts. Removing the last widget is blocked.
  The preview draws the constant black background and widgets in export order. Preview options
  (ellipsis) flyout: **Preview grid** (16 px + snapping), **Unselected widgets outline**,
  **Live preview**, gear → `CustomDataDialog`. Predefined data is the default
  (`DEFAULT_PREVIEW_DATETIME` = 31.07.2022 21:37:40 via `rw_preview_default_datetime`);
  Live preview changes **only** time/date/date-derived fields to `DateTime.Now`, never weather
  or sensors; the 1 s `_clockTimer` does nothing when off and `UpdateLiveTimeDateOnly` avoids
  canvas rebuilds. The X and Y sliders sit side by side with live numeric values (single tap
  copies to clipboard, double tap edits clamped value). Under **Center on watchface** are
  **Horizontal** and **Vertical** buttons; both X/Y bars and centering group are shared by
  digital and analog widgets, while digital-only controls live in `DigitalSettingsPanel` /
  `DigitalAlignPanel`. The inspector ScrollViewer reserves 14 px on the right.
- **Widgets catalog** categories: Time (Digital time, Analog time); Date (Date, Day of week
  (number), Week of year, Day of year, Lunar date, Festival, World clock — category `date`);
  Text (Text (user defined)); Images (Image); Distance (Step counter, Distance); Goals (Step,
  Calorie, Stand, Fat burning target); Health (heart current/last/min/max/avg, Stress, PAI
  today/7 days/week, Skin temperature, Calories, SpO₂ current/hourly, Stand hours, Fat
  burning); Sleep (duration, score, Deep/Light/REM, Time awake, start/end, target); System
  (Battery, DND, LOCK, BLUETOOTH, ALARM); Weather (9 kinds); Progress (arc, bar); Shortcut.
  `date_format.rs` owns the pattern (`D`/`DD`, `M`/`MM`, `MMM`/`mmm`, `MMMM`/`mmmm`,
  `DDD`/`ddd`, `DDDD`/`dddd`, `YY`/`YYYY`, `\` escapes); letter case controls name case,
  5/6 repeats give UPPERCASE. Weekday index is 0 = Sunday, matching generated JS Sakamoto.
  The six generated date fields have no editable pattern; they come from `TIME` fields
  (weekday number is **not** `TIME.week`), ISO week, day-of-year, lunar/festival fields and
  `WORLD_CLOCK`. First day of week is an app setting defaulting to `monday` stored on the
  project via `rw_set_first_day_of_week` and passed to `rw_format_date_field`.
  Text widget content lives in `Widget::text` (`UserTextBox`, saved through `rw_set_widget`,
  static text in JS, no timer). `steps` shows `"0"` on PC and emits a guarded STEP sensor
  listener. `distance` supports Watch/Metric/Imperial via `Widget.measurement_system`
  (`null` follows project; Watch calls `hmSetting.getMileageUnit()` in `_initMileageUnit`).
  Goal widgets read `.target`; Stand/Fat-burning may be missing on Active 2 — placeholder
  remains and a localized warning is shown. Health widgets use guarded classic sensors with
  one shared `healthTick` and `onDestroy` cleanup; units controlled by `show_unit`. HRV and
  Readiness are intentionally absent. Weather text and icon fields have `weatherDayOffset`;
  `sun-current` computes time to next sunrise/sunset in `_sunCurrentText()`.
- **Progress** (`CATEGORY_PROGRESS`) shares one `ProgressSettings` on `Widget`. Inspector
  exposes only source, thickness, corner radius, colours, track, rounded ends and (arc only)
  start/end angles; fill is always continuous. New arcs default to 0…360°; the WinUI preview
  draws that as an `EllipseGeometry` (identical endpoints make `ArcSegment` draw nothing).
  Sources: battery, steps, calories, stand, fat-burning, PAI, SpO₂, sleep, heart, fixed;
  goal sources always divide by the watch's `.target`; **distance is legacy-only**. `HEART`
  uses `heart.last` on the `LAST` event only (never `CURRENT`, never from tick) with a fixed
  165 BPM scale; sleep calls `updateInfo()` before `getTotalTime()`. Codegen emits native
  `hmUI.widget.ARC_PROGRESS` / `FILL_RECT` refreshed from `progressTick` (called from `tick`
  and deduplicated sensor listeners), with all JS dimensions as **frame literals** — bare
  `x`/`y`/`w`/`h` identifiers caused a `ReferenceError` and left the bar unfilled. The arc's
  extra point at 0° changes only `Progress.Thickness` (`arc_center_radius()` keeps the ring
  centre fixed); the editor gives the `Path` negative margin and enlarged bounds so anything
  outside the blue outline still draws. `arc_draw_margin()` widens the Rust renderer too
  (`max(0, thickness/2 − 8) + 6`); `arc_coverage()` honours a reversed `start > end` sweep;
  a rounded bar's radius is never 0. The WinUI preview bar fill is anchored with
  `HorizontalAlignment.Left` (or `VerticalAlignment.Bottom` when vertical) inside a `Grid` —
  `Canvas.SetLeft/SetTop` are no-ops there. `UpdateProgressVisual` refreshes stroke thickness,
  brushes and the track's line caps. Bar width/height NumberBoxes go down to 1 px.
- **Assets page:** Fonts, Images, Hands, Status icons and Weather sections with GridView tiles
  (≤3 per row), import/remove/open-folder buttons, generated preview thumbnails and a
  localized `AssetsDefaultBadge` for the **watch system font** and the built-in hand/status
  packs plus the **default** weather pack (`system-like`); other bundled packs have no badge
  but stay protected from removal. The system font is an informational, non-removable virtual
  tile rendered with Noto Sans; Inter has no badge but stays protected from removal as the
  bundled fallback. Caches
  decoded bitmaps by path + fingerprint; shows a `ProgressRing` for new/changed packs.
- **App Settings:** language selector + live switching, Open languages folder, Reload
  languages, Default author, Watchface preview language, Default system of units and First day
  of week (both written to the open project via `rw_set_measurement_system` /
  `rw_set_first_day_of_week`). All values live in `settings.json`; never overwrite one setting
  when saving another. App logo: `Assets/AppIcon.ico`, `Assets/Logo.png`, masters in `logo/`.
- **Bottom bar:** Save, Save as…, Test in emulator (disabled until a project is open) and
  **Export** (rightmost, `AccentButtonStyle`). Save uses `rw_load_project` + `rw_save_project`;
  Save as uses `rw_save_project_as`. Clicking Export renders the branded `preview.png` and
  8-frame `preview.gif` (progress `1/8 … 8/8`, indeterminate while GIF encoding), then opens an
  in-app `ContentDialog` (direct child of `RootGrid`) with the GIF clipped to a
  `CornerRadius="120"` round border above project details and a single **QR code** button —
  the ZIP is already written by the Export click itself (`ExportZipNow()`), which also shows the
  "Exported: …" status. There is no Export ZIP button in the dialog. The dialog is a
  `ScrollViewer` and the QR uses `MaxWidth/MaxHeight` + `Stretch="Uniform"`, so a lower window
  cannot crop it.
  Export prepares a **fresh ZIP, a fresh port, the server and the QR image before the dialog
  opens**; the QR button only shows the prepared image, so the code is visible only after that
  click. Each Export resets the QR view. The QR is pure Rust (`rusty-core::qr`, `QrCode`,
  `EcLevel::L`, same as `qrcode-terminal` in zeus) delivered as base64 PNG via
  `rw_generate_qr`. The package is served by an in-process `TcpListener` on the LAN IPv4 with
  URL `zpkd1://<ip>:<port>/<name>.zip` (exact `zeus preview --local` scheme). The server strips
  queries, URL-decodes the path, accepts any `.zip` request and handles `HEAD`; a local GET
  self-check must return 200 before the QR is shown. No Zepp cloud/account/token/third-party QR
  service. The server stops on `ExportDialog.Closing` and `MainWindow.Closed`. Phone and PC
  must share the LAN; whether the flat v2 ZIP is accepted directly by Zepp still needs a
  real-device check.
- **Test in emulator** (`rw_prepare_emulator_workspace` → `zeus install`) exports to
  `%APPDATA%\RustyWatch\Emulator\app-<appId>`, unpacks it and installs into the official Zepp OS
  Simulator. `ZeppSimulator` detects the install, the downloaded Active 2 image
  (`%APPDATA%\simulator\config.json` — `selectDeviceList` may be object or array) and the open
  watch window (`simuOpen`). If the simulator is missing, only open the official download page —
  never bundle, mirror or silently install Zepp binaries or accept their EULA. zeus target name
  is **`Amazfit Active 2 NFC (Round)`**. zeus can print `[ERROR]` while exiting 0 — treat both
  as failure.

## 8. Device / output verification

`deviceSource: 8913153`, `target_key: "active2round"` and the API versions are profile
constants in `rusty-core/src/model.rs`. Generated `app.json` is a **flat v2** manifest
(`configVersion: "v2"`, no `targets`) with `defaultLanguage`, `packageInfo` and all 8 Active 2
`deviceSource` values. The v3/target layout is created only inside the emulator workspace by
`prepare_emulator_workspace` → `convert_export_for_zeus` (zeus rejects packages without it).

```powershell
zeus install --path <unpacked-project-dir> --target "Amazfit Active 2 NFC (Round)"
```

The simulator needs the `Active 2 v1.1.0` device image downloaded by the user inside its own
download manager; a simulator run validates JS build/runtime but does not replace a real-device
check. ZIP layout is flat v2: `app.json`, `app.js`, `watchface/index.js`, `assets/*` with
explicit directory entries. Every asset is written directly into `assets/` under a deterministic
hashed name (`hand_<hash>.png`, `font_<hash>.ttf`, `status_<hash>.png`, `img_<hash>.png`,
`weather_<hash>.png`; fonts keep the `fonts/` folder). `icon.png` keeps its name. `export_paths.rs`
is the single source of truth shared by codegen and package. There is no `bg.png`. ZIP directory
entries are post-processed to DOS metadata (`0x41ff0010`). `assets/Preview.png` is a 314 × 314
RGBA round-masked rescale of the branded preview, never the old bundled `Logo.png`.
`preview.gif` and the full 466 × 466 `preview.png` stay local (not packaged). Prefer
performance-friendly generated JS: fewer widgets, correct AOD, no listener leaks.

## 9. Definition of done for a change

1. `cargo test --workspace` green (208 tests: 170 core + 38 FFI), no new warnings.
2. `dotnet build .\native\RustyWatch\RustyWatch.csproj` — 0 errors, 0 warnings.
3. App launches; affected screens verified visually (English **and** Polish if UI changed).
4. Round-trip `new → export → open → export` preserves data when project model/codegen changes.
5. No hardcoded user-facing strings; no temporary debug hooks; docs (`README.md`,
   `docs/TECHNICAL.md`, `docs/FORMAT.md`, `PLAN.md` if decisions changed) updated.
