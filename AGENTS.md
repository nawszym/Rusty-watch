# AGENTS.md — Rusty Watch

Instructions for AI coding agents working in this repository. Keep this file up to date when
architecture, build commands, or hard-won workarounds change. Other agent entry points
(`CLAUDE.md`, `.github/copilot-instructions.md`) point back here — edit this file, not copies.

## 1. What this project is

Rusty Watch is a native Windows 11 **watchface editor for Amazfit Active 2 (Round)**.
It creates projects, previews them 1:1, and exports installable `.zip` watchface packages
for ZeppOS / amazwatchfaces.

- **Only UI:** WinUI 3 (Windows App SDK **2.7.0**), C#/XAML, in `native/RustyWatch/`.
- **Business logic / codegen:** Rust workspace in `crates/`, exposed to C# through a C ABI.
- There is no secondary UI. The former Tauri + React prototype in `ui/` was removed — do not
  reintroduce a web-based frontend.
- Target device only: **Amazfit Active 2 (Round)**, 466×466, ZeppOS 4.x, `deviceSource: 8913153`.
- Export **`.zip` only** — never `.zpk`.
- License: **GPL-3.0-or-later + Generation Exception**. Generated watchface output is *not*
  subject to GPL; editor source and its derivatives must stay open source. Do not copy
  SashaCX75 source code or third-party assets (inspiration/format only).

Historical context and decisions: `PLAN.md` (section 8 = confirmed decisions), `README.md`,
`docs/FORMAT.md`. Read them before changing scope or data formats.

## 2. Repository map

| Path | Role |
|---|---|
| `Cargo.toml` | Rust workspace: `rusty-core`, `rusty-cli`, `rusty-ffi` |
| `crates/rusty-core/src/` | `model.rs` (Project/Widget/metadata, device constants, starter template), `project.rs` (create/load/save/save-as/folder sanitization/random App ID), `codegen.rs` (generated `app.json`, `app.js`, `watchface/index.js`), `time_format.rs` (shared pattern parser + JS expression builder), `layers.rs` (layer order + editor layer descriptors), `widgets.rs` (add/edit/remove widget), `package.rs` (zip export/inspect), `fonts.rs` (font store, TTF name parser, project attachment/pruning), `recent.rs` (recent-project store), `locale.rs` (watchface language: PL/EN tables + runtime JS detection), `images.rs` (shared image store + background preparation), `image_fit.rs` (reusable Cover/Contain/Stretch scaling), `hands.rs` (shared hand-set store + project attachment), `status_icons.rs` (shared STATUS icon-pack store + project attachment), `preview.rs` (7-frame PNG/GIF renderer, hardcoded presets + random series, round cutout), `validate.rs` |
| `crates/rusty-cli/src/main.rs` | CLI: `new`, `validate`, `export`, `inspect` |
| `crates/rusty-ffi/src/lib.rs` | C ABI, 57 `rw_*` exports (incl. `rw_free_string`, `rw_ensure_defaults`, `rw_set_hand_visibility`, `rw_preview_random_series`, `rw_render_preview_frame`, `rw_encode_preview_gif`); all take/return UTF-8 JSON; caller frees with `rw_free_string` |
| `native/RustyWatch/MainWindow.xaml(.cs)` | Shell, HOME, Editor, Assets (Fonts), App Settings, bottom bar, dialogs |
| `native/RustyWatch/RustNative.cs` | P/Invoke bridge (`DllImport`, `CallingConvention.Cdecl`, `LPUTF8Str`) |
| `native/RustyWatch/PreviewFonts.cs` | Copies font TTFs next to the .exe and builds `ms-appx:///FontCache/...` families so the preview uses the exact font exported to the watch |
| `native/RustyWatch/Assets/Fonts/` | Bundled `Inter-Regular.ttf` + `OFL-Inter.txt` (SIL OFL 1.1) |
| `native/RustyWatch/Localization/` | Bundled `*.json` catalog seeds + `README.md` (format/user guide) + `LocalizationService`, `StringCatalog`, `LocalizationBridge`. Runtime copies live in `%APPDATA%\RustyWatch\Localization` |
| `native/RustyWatch/RustyWatch.csproj` | Build orchestration: builds Rust FFI, copies DLL, language catalogs and bundled fonts |
| `examples/first-watch/` | example project (gitignored) |
| `reference-ui.png` | local-only visual reference for the shell layout (screenshot of a third-party editor UI); gitignored — do not commit |

## 3. Build, test, run

Workspace root: `D:\.agent - projekty\Rusty-Watch` (Windows, PowerShell).

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

`dotnet build` already runs `cargo build -p rusty-ffi`. Expect **0 errors / 0 warnings** on both
toolchains; treat new warnings as regressions. Prerequisites installed: .NET SDK 10 + Runtime 8,
Rust, Windows App SDK via NuGet (self-contained app).

## 4. Architecture rules (do not violate)

1. **Layering:** `rusty-core` has zero UI dependencies; all logic is pure/testable.
   `rusty-cli` is a thin wrapper. `rusty-ffi` is the only bridge. C# contains no business logic —
   it calls `rw_*`, parses JSON, and updates the UI.
2. **FFI contract:** every `rw_*` function returns a JSON UTF-8 string of the shape
   `{"ok":true,"data":...}` or `{"ok":false,"error":"..."}`, allocated by Rust; C# must free it
   with `rw_free_string`. Add new functions in **both** `rusty-ffi/src/lib.rs` and
   `native/RustyWatch/RustNative.cs`.
3. **New feature flow:** implement in `rusty-core` → add tests → expose via `rusty-ffi` →
   wire into `native/RustyWatch` → run `cargo test --workspace` **and** `dotnet build`.
4. **Time format is shared:** `rusty-core/src/time_format.rs` is the single source of truth for
   preview formatting and generated JS. Tokens: `HH/H/hh/h`, `MM/M/m`, `SS/S/s`, `A/a`, `\` for
   literals. Seconds in the pattern ⇒ generated JS uses `timer.createTimer(0, 1000, tick)` + `timer.stopTimer`;
   otherwise a `hmSensor` `MINUTEEND` listener. **Generated watchface JS must use the classic
   Watchface API `hmUI.createWidget(hmUI.widget.TEXT/IMG, ...)`, `hmSensor`, `timer` and global
   `WatchFace({...})`  -  NOT `@zos/ui` / `@zos/sensor` (those are for Mini Programs; with API 3.x
   the runtime does not expose `hmUI` and the watchface renders black  -  verified in the simulator).**
   `generate_app_json` therefore emits `runtime.apiVersion` **1.0.0 / 1.0.1** and **no**
   `runtime.type` (`type: 2` means `.bin` bytecode and blocks loading our `.js`). Seconds mode uses
   `timer.createTimer(0, 1000, tick)` + `timer.stopTimer`; listeners are cleaned up in `onDestroy`.
   The export **always** emits `hmUI.show_level.ONLY_NORMAL` (`codegen::JS_SHOW_LEVEL`), even for projects that still store `Always`: `ONAL_AOD` made the real firmware paint an opaque AOD overlay (white square with small hand frames, black screen with full-screen frames). AOD is driven by `WIDGET_DELEGATE` + `hmSetting.getScreenType()` instead. New widgets default to `ShowLevel::Normal`.
   Align / text-style / show-level JS expressions live in `model.rs::as_js()` with an `hmUI.` prefix.
 4a. **The watchface detects its own language at runtime.** `crates/rusty-core/src/locale.rs` owns the PL/EN tables (Polish month abbreviations and AM/PM markers) and emits a JS block (inside the IIFE) calling `DeviceRuntimeCore.HmUtils.getLanguage()` with a guarded `hmSetting.getLanguage()` fallback — `9` = `pl-PL`, **every other value = English**. `codegen` uses `L.months` for Date widgets and `time_format::js_expression_localized` (`L.am`/`L.pm`) for time widgets; the JS locale block includes Polish diacritics directly in the initial widget text. Do not bake one language into the ZIP. The editor has a **watchface preview language** setting (`auto` = Windows UI culture, `pl`, `en`) in `AppSettingsStore`, passed through `rw_format_time`/`rw_month_name`, so the preview matches whichever variant the watch will show.
  4b. **Generated previews are Rust-only and deterministic per export.** `crates/rusty-core/src/preview.rs` owns the 7 hardcoded presets (`FRAME_PRESETS`: hours 06–21, minutes/seconds strictly increasing), the random `PreviewRandomSeries` (7 real dates 2022-01-03 … 2022-07-31 — Monday…Sunday in months January…July; realistic heart rate/calories/distance), and the renderer. Steps are always derived from distance (`distance_m/1000 * steps_per_km`, 1300–1400 steps/km); battery decreases 100% → 10%. Every date widget in a frame uses that frame's single date (Sakamoto weekday with `project.first_day_of_week`, ISO week, day-of-year, `locale.rs` month names). `FRAME_PRESETS` also carries a `status_mask` (ALARM=1, DND=2, BLUETOOTH=4, LOCK=8): most frames show no icons; frame 7 (21:37:40) shows all four and is the source of `preview.png`. The mask sequence is `2,2,0,3,0,1,4` — non-monotonic, with 2 empty frames; frame 4 has ALARM+DND+LOCK, frame 6 only BLUETOOTH. `preview.png` is exactly frame 7 (21:37:40), `preview.gif` has 7 frames with ~1.7 s per frame and an infinite loop and a black opaque cutout outside radius 233. Neither file is zipped. Custom Data from the editor is deliberately ignored. Never render preview frames with offscreen WinUI controls.
  4c. **The generated watchface JS must never use `Date`.** The classic ZeppOS watchface runtime on the real watch does not expose `Date` (it works in the simulator, which has a full JS engine); calling it during `build()` aborts the watchface and leaves a black screen. `date_format.rs` emits `JS_DATE_FROM_T` (plain fields `d`/`m`/`m0`/`y`/`w` computed with Sakamoto's algorithm), `JS_ISO_WEEK` and `JS_DAY_OF_YEAR` using only arithmetic on those fields. `js_expression_localized` reads the same fields instead of `getDate()`/`getMonth()`/`getDay()`/`getFullYear()`. A regression test bans any `Date` API in generated JS. Text widgets emit `char_space: 0`, `line_space: 0` and `font: "fonts/font_<hash>.ttf"` exactly like the working reference (see 6a).
 5. **Layers are the single source of draw order.** `crates/rusty-core/src/layers.rs` resolves
   `Project::layer_order` (bottom → top, **widget ids only**), keeps widget `z` in sync, and exposes
   descriptors for the editor. `codegen::generate_index_js` and the WinUI preview must both use
   `layers::ordered_widgets()` / `layers::describe()`, so the preview matches the exported watchface.
   **There is no background layer and no background asset.** The watchface background is a constant
   `hmUI.widget.FILL_RECT(x:0,y:0,w:466,h:466,color:0xFF05070D)` emitted before all widgets — exactly
   like the working Python reference. Do not order by vector index or `z` separately in C#.
   `rw_move_layer` saves the new order immediately.
   Widget creation/editing/removal lives in `crates/rusty-core/src/widgets.rs`
   (`rw_add_widget`, `rw_set_widget`, `rw_remove_widget`). The last remaining widget cannot be
   removed (export validation requires at least one). The left **Widgets tab is a read-only catalog**
   of the types the app supports (`rw_supported_widgets`) — it must not contain an Add button.
   Adding happens from the **+ Add widget** button on the right of the editor's Preview bar: it opens
   a Flyout with a search box and a selectable catalog; choosing an entry calls `rw_add_widget` and
   closes the Flyout. The right-hand inspector is driven by the layer selected in Layers
   (`LayersList_SelectionChanged` → `LoadWidgetSettings`), so never hard-wire it to the first time
   widget. Keep every bundled `Localization/*.json` valid JSON — an unescaped `"` silently drops the whole
   catalog and makes the UI fall back to English (also true for user catalogs in %APPDATA%).
6a. **Text widget frame (ramka) and scaling.** Every text widget is drawn in the preview as a
   transparent `Border` frame at exactly `x/y/w/h` (same geometry as the generated TEXT widget),
   with four corner resize handles. Dragging inside moves the widget; dragging a corner resizes it.
   `rw_frame_geometry` is the single source of truth for this gesture math (clamps to 466×466,
   minimum 24px, text size 8–160) and is called on every pointer move — C# only applies the returned
   values. The optional `snap` field makes moved/resized edges snap to `SNAP_GRID` (16px); C# passes the checkbox state and never snaps by itself. `alignX`/`alignY` carry candidate lines (canvas edges/center and other widgets' edges/centers); Rust returns `guideX`/`guideY` and C# only draws the dashed red guides — alignment wins over the grid. Double-clicking the frame toggles `Widget::auto_scale`: the frame turns red and resizing
   scales `text_size` proportionally to width. `text_style` (`ellipsis`/`none`/`wrap`) maps directly
   to ZeppOS `text_style`. Scaling persists the **real** `text_size`, so the preview and exported
   `watchface/index.js` stay 1:1. Never introduce a separate UI-only zoom. `v_align` (`top`/`center`/`bottom`) maps to `align_v`; text lives in a host `Grid` sized to the frame because `VerticalAlignment` on a direct Canvas child has no effect. **The `TextBlock` itself must stay content-sized** (`Height = double.NaN`, `MaxHeight` = current frame height, `Width` = current frame width): `VerticalAlignment` then anchors it to the *current* frame edges. A fixed `Height` (e.g. `Height = layer.H` set once at creation) makes the text keep a stale bounding box after scaling — with `center` the offset `(h_old − h_new)/2` goes negative and the text escapes above/outside the Canvas. Never set an explicit `Text.Height`; `ApplyFrameGeometry` must refresh `Width`/`MaxHeight` together with the host and frame. Horizontal
(`left`/`center`/`right`) and vertical (`top`/`center`/`bottom`) alignment are applied in the single
`ApplyTextLayout` helper called at the end of every `ApplyFrameGeometry` — do not set
`TextAlignment`/`VerticalAlignment` separately from geometry, otherwise the TextBlock keeps a stale
anchor after scaling or after an inspector change (regression: preview and exported JS disagreed).
`ApplyLayersPayload` only rebuilds the list/preview when layer identity or order changes — geometry edits update existing `LayerRow`/`FrameView` objects in place (`UpdateExistingLayerViews`), otherwise the list jumps and the preview flashes black.
6a. **Fonts are one shared asset store, but projects are self-contained.** Installed fonts live in `%APPDATA%\RustyWatch\Fonts` (`rusty-core::fonts`); the bundled default is `Inter-Regular.ttf` (SIL OFL 1.1, license in `native/RustyWatch/Assets/Fonts/OFL-Inter.txt`) and is copied there by `ensure_default`. `Widget.font` stores only the file name; `None` = watch system font. Assigning a font copies it into `<project>/assets/fonts/<file>` and registers an `Asset { role: Font }`, so `save as`/ZIP export include it; `prune_unused` removes old copies when the user switches fonts. The generated JS **does set `font: "fonts/font_<hash>.ttf"`** when a widget has a font — exactly like the 100% working reference export (which uses `fonts/Rajdhani-*.ttf`). The TTF is copied into `assets/fonts/` and referenced by `src:`-equivalent name; editor preview and watchface now use the same font. There are deliberately **no** offscreen "glyph warm-up" widgets - they were created before the background and could abort `build()` on a real watch; the initial text already contains the full character set. The WinUI preview renders with the selected TTF (`PreviewFonts.cs` → `ms-appx:///FontCache/...`) for editing; the exported watchface uses the same TTF, so glyph metrics match the editor preview. WinUI unpackaged apps cannot load `file:///` or `%APPDATA%` fonts directly. New projects get the default font via `rw_apply_default_font`; when **opening any project** and after `rw_add_widget`, call `rw_apply_default_font_to_missing` so widgets with `font: null` receive the bundled Inter TTF too. Without it the editor fell back to Segoe UI — identical `x/y/align_h/align_v` then produced different glyph widths/baselines in the preview. Custom fonts are never overwritten.
6b. **There is no background store.** The watchface background is always the constant `FILL_RECT`
   in generated JS (`0xFF05070D`), exactly as in the working Python reference. No `Backgrounds`
   directory, no `bg.png`, no `background` field in `project.json`, no `AssetRole::Background`, no
   background layer in Layers, no background controls in the editor and no `rw_backgrounds_*` /
   `rw_set_background` FFI. General graphics for the IMAGES widget live in
   `%APPDATA%\RustyWatch\Images` (`rusty-core::images`) and are fitted with the shared `image_fit` module.
   The **IMAGES widget** (`WidgetKind::Images`, catalog kind `images`, new `images` category) uses that same `image_fit` module: `rw_set_widget_image` copies the chosen file from `%APPDATA%\RustyWatch\Images` into `<project>/assets/images/<widget-id>/<file>`, physically resized to the widget's `w × h` with the selected `FitMode` (`cover`/`contain`/`stretch`, default `contain`). `Widget.src` points at that prepared asset path and `Widget.image_fit` stores the mode. `rw_set_widget` re-prepares the PNG from the pristine store copy whenever geometry or fit changes; `package.rs` applies `images::export_asset_bytes` at export as a safety net for older projects. Preview draws the PNG at its natural size with a movable frame; blue mode moves, double-click toggles red scale mode. Assets page has a separate **Images** section (import/remove/open folder), and the inspector has an image picker plus a fit-mode combo.
6c. **Analog time and hand sets.** The addable widget kinds come from `widgets::supported_widgets()` as `{kind, category}`. The "+ Add widget" flyout builds collapsible `Expander` groups from that list (collapsed by default; search auto-expands categories with matches and filters items inside them), while the left Widgets tab shows the same categories always expanded as plain headers with all items visible — no arrow to click. The old time widget is **"Digital time"** and both time widgets live in category `time`; the **"Date"** text widget lives in category `text` (`WidgetKind::DateText` + `DataSource::Date`, rendered with the editable `date_format` pattern). An analog widget (`WidgetKind::AnalogTime`, addable kind `analog-time`) is a single layer whose generated JS is a group of four rotating `hmUI.widget.IMG` widgets (hour/minute/second/cap), each with a full-screen frame (`x:0,y:0,w:466,h:466`), `pos_x/pos_y` placing the PNG, `center_x/center_y` at the watch centre, `auto_scale: false`, and `angle` updated through `setProperty(hmUI.prop.MORE, …)` from the shared `tick`; the cap is static. Seconds hand ⇒ 1 s timer, otherwise MINUTEEND listener. Installed hand sets live in `%APPDATA%\RustyWatch\Hands\<id>\` (`rusty-core::hands`: list/add/remove/ensure_default); a set needs square `hour.png` + `minute.png` of equal size, `second.png`/`cap.png` optional. Bundled `Classic` (**with its subtle shadow**) and `Light` (pure white hands with a dark outline and soft glow) are committed in `native/RustyWatch/Assets/Hands/` and installed from there; the scratch original in `temporary/hands` is local-only and gitignored. `hands::ensure_default` seeds **every** valid bundled set and skips sets already installed — it must not return early after `Classic`, otherwise newly bundled sets never appear in existing installations. Selecting a set copies files to `<project>/assets/hands/<id>/`, registers `AssetRole::Hand` and prunes unused sets; ZIP stays self-contained. A **new** analog widget gets its frame centred and sized from the PNGs, but changing the set on an existing widget must preserve its current `x/y/w/h` (only the look changes); `rw_set_hands` then re-scales the new PNGs to that preserved frame via `hands::scale_project_hands`. Keep hand images smaller than the full 466×466 screen (the bundled set is 400×400) — full-screen hand bitmaps are known to cause trouble on real watches. The preview rotates `Image` elements with `RotateTransform` on the same clock timer; no UI-only zoom. Frames and their four resize handles are drawn on a separate `OutlineCanvas` **outside the round clip**, so the handles sit exactly on the layer's corners and stay grabbable even when the 400×400 frame's corners fall outside the watch screen. In the blue mode image layers (hands and future graphics) can only be moved; the red mode (double-click the frame) enables scaling. Text layers resize their canvas in blue and scale the text in red. `rw_frame_geometry` receives `square: true` for analog layers and keeps `w == h` while resizing, so hands are scaled uniformly — the real `w/h` is persisted in the project and drives the PNG size and `pos_x/pos_y` (the IMG widget itself uses the full-screen frame). Scaling any widget image at runtime via `setProperty` is still unsupported; here the size is set when the widget is created. **`hmUI.widget.IMG` does not scale its PNG at runtime either** — it draws the image at its natural size and clips it to the widget's `w/h`. So the PNG files copied into `<project>/assets/hands/<id>/` must be physically resized to the analog widget's `w/h`: `hands::scale_project_hands` does this when geometry is saved (`rw_set_widget`) and when a hand is re-enabled (`rw_set_hand_visibility`), always starting from the pristine `%APPDATA%` store copy; `hands::export_asset_bytes` passes assets through unchanged; the crop is applied by `prepare_hand_assets` in both export paths. Without it a 400×400 PNG in a 175×175 widget shows only a tiny centre crop on the watch. Additionally `hands::prepare_hand_assets` follows the working Python reference: one common scale for the whole set, then a **tight alpha bounding box with an 8 px transparent margin**, and emits per-hand `x/y/w/h` (used as `pos_x/pos_y` and as the PNG size) via `generate_index_js_with_hands`. The exported IMG frame is always the full screen (`466×466`) with `center_x/center_y = 233`, so rotation happens around the watch centre and the tight crop is not clipped. This reduces decoded hand textures from ~1.4 MB to ~150 KB for four hands; a full-screen 466×466 hand texture costs ~868 KB and several of them can reboot the real firmware. Both export paths (`export_zip` and `export_zip_bytes`) must apply the same preparation. The analog inspector also has one checkbox per hand (hour, minute, second and the centre cap). A checkbox appears only when the selected set actually contains that file (hour/minute always, second/cap when installed). `rw_set_hand_visibility` → `hands::set_hand_visibility` clears the corresponding `*_src` to hide a hand and re-copies the PNG from `%APPDATA%` to re-enable it (disabled PNGs are removed from the project by `prune_unused`). Codegen emits `null` for hidden hands and skips their angle updates; validation accepts an analog widget with any subset of hands. User visibility choices are remembered across set switches for hands the new set supports.
 6d. **System widgets and STATUS icon packs.** The `system` category (`CATEGORY_SYSTEM`) holds
   `battery` (`WidgetKind::BatteryText`, `DataSource::Battery` + `hmSensor.id.BATTERY`) and four
   `status-*` layers (`WidgetKind::StatusIcon`) rendered by `hmUI.widget.IMG_STATUS` with
   `hmUI.system_status.CLOCK/DISTURB/DISCONNECT/LOCK`. ZeppOS toggles their visibility itself —
   no sensors, timers or listeners are generated for them. Installed icon packs live in
   `%APPDATA%\RustyWatch\Status\<id>\` (`rusty-core::status_icons`: list/add/remove/ensure_default,
   same pattern as `hands`), a pack is detected only when **all four** square PNGs are present:
   `alarm.png`, `dnd.png`, `bluetooth.png`, `lock.png`. Only `Classic` is bundled (the default,
   it is seeded from `native/RustyWatch/Assets/Status/` and cannot be
   removed. Selecting a pack copies
   it to `<project>/assets/status/<id>/`, registers `AssetRole::StatusIcon`, prunes unused packs and
   physically resizes the PNGs to the frame (`IMG` does not scale at runtime). Default frames keep
   all four icons inside the 233 px round cutout. A new **Battery** widget is centred on the
   watchface (`x=23,y=203,w=420,h=60`); new status icons form one centred horizontal row
   (`x=145/193/241/289,y=24,w=h=32`). Status layers use the same blue-move / red-scale model as
   other images (double-click toggles `Widget::auto_scale`, now persisted by `rw_set_widget`), with
   a scale-mode button in their inspector. The Assets page has a **Status icons** section;
   the inspector has a pack ComboBox, and all four status kinds sit directly in SYSTEM.
   Fonts, Images, Hands and Status packs are all shown as **GridView tiles,
   at most three per row**; each installed pack gets a generated `preview.png` (Hands: all hands
   at **21:37:11**; Status: 2×2 grid of the four icons), so the Assets page never renders controls
   offscreen.
   **Entering Assets must never regenerate thumbnails or block navigation.** `Navigate("assets")` shows the
   view first and refreshes the stores in `Task.Run` (results marshalled with
   `DispatcherQueue.TryEnqueue`, stale responses discarded by a version counter). Rust stores a
   `<pack>/preview.fingerprint` (names/sizes/mtimes of the source PNGs) and regenerates `preview.png`
   only when it is missing or the fingerprint changed. `ensure_default` also refreshes an installed
   built-in pack when a `.bundled-version` marker differs from the bundled source, so updated Classic
   icons reach existing installations and attached project copies. C# caches decoded `BitmapImage`s by
   path+fingerprint and shows a per-tile `ProgressRing` only for new/changed packs until that image opens.
   The built-in/default assets show a localized `AssetsDefaultBadge` under their name
   (Inter font, Classic hands and Classic status icons). Every font tile uses the
   same blue rounded tile with the sample `AaBbCc / 012345` drawn in that exact TTF. **Never set
   `FontWeight` (or any other filter) on that TextBlock** — SemiBold makes WinUI synthesize a bold face
   and the preview no longer matches the font (a dot-matrix font such as ndot57 loses its dots).
6. **Project storage:** one folder per watchface under a user-chosen projects root:
   `<root>/<sanitized watchface name>/project.json`. Never overwrite an existing folder — return
   an error. `metadata.name/appId` land in generated `app.json` as `appName`/`appId`; for byte-parity with the reference `vender` is `zepp` and `description` is empty,
   `appId`, and `i18n.*.appName`.
7. **App data lives in %APPDATA%\RustyWatch** (roaming, survives updates): `settings.json`,
   `recent.json`, the default `Projects` folder, the shared `Fonts`/`Images`/`Hands`/`Status` stores, the
   runtime `Localization` catalogs and `Emulator` work dir. `AppPaths.Initialize()` creates the
   whole structure on first launch, seeds missing bundled localization files (never overwrites
   user copies) and copies legacy files from %LOCALAPPDATA%\RustyWatch (roaming files win).
   `rw_ensure_defaults` (called from `MainWindow`'s constructor) seeds the default font and hand
   set; no installer is needed for first-run population. `AppSettingsStore` is the single writer
   for settings; `MainWindow` uses `AppPaths.RecentStorePath`. Never hardcode %LOCALAPPDATA% again.
8. **Projects root:** New project / Save as create directly in `AppSettingsStore.ProjectsDirectory`
   (default %APPDATA%\RustyWatch\Projects), no folder prompt. The App Settings field changes it and
   persists it in `settings.json`; its `TextChanged` handler must stay wired in XAML.
9. **Recent projects:** `%APPDATA%/RustyWatch/recent.json`, max 10 entries, BOM-tolerant,
   canonically a JSON **array**. PowerShell scripts can turn it into `{"value":[...],"Count":N}`
   or a single object — `recent::load` accepts both shapes and rewrites the file as an array on
   the next save, and `rw_open_project` never fails just because the recent store is broken.
   When editing this file from a script, serialize an **array explicitly** (`[...]` via string
   join), never rely on `ConvertTo-Json` input enumeration.
10. **No debug hooks in shipped code.** Temporary startup switches used for screenshot
   verification (`RW_DEBUG_*`) were removed; do not reintroduce them.

## 5. Localization (PL / EN, ready for more)

- Bundled user-visible text lives in `native/RustyWatch/Localization/en.json` and `pl.json` under
  a `strings` object; XAML binds with `{Binding Strings[Key]}`. **At runtime the catalogs are read
  only from `%APPDATA%\RustyWatch\Localization`** — `AppPaths.Initialize()` seeds missing files
  from the bundled folder on first launch and never overwrites existing ones. The bundled files
  are therefore *seeds*, not the live source; editing them does not affect an existing install
  (delete the appdata copy to restore the bundled original).
- When adding a UI string: add the same key to **every** bundled catalog, then bind it in XAML.
  When a string needs runtime formatting use `LocalizationService.Format(key, args)`; for plain
  text use `LocalizationService.Get(key)`. `StringCatalog` falls back to the English catalog for
  missing keys so partial user translations still show text instead of key names.
- `LocalizationService.SetLanguage()` + `ApplyLocalization()` / `LocalizationBridge.Refresh()`
  is the live-switch path. New languages require only dropping `<code>.json` into the **appdata**
  `Localization` folder and clicking **Reload languages** in App Settings — no code changes and no
  rebuild/reinstall. The catalog list is discovered from `*.json` filenames at runtime; malformed
  JSON is skipped. Keep `native/RustyWatch/Localization/README.md` as the user-facing format guide.
- Rust diagnostics are canonically Polish. English is produced by the pattern map in
  `LocalizationService.Error`. When adding a new Rust error message, add its pattern to that map
  and keys to all catalogs.
- Language choice persists in `%APPDATA%\RustyWatch\settings.json`; default follows the
  Windows UI culture.
- Assistant-facing docs/code comments in this repo are largely Polish; keep new comments
  consistent with the surrounding file. User-facing strings must **always** come from catalogs.

## 6. WinUI 3 / Windows App SDK gotchas (learned the hard way)

Do not "fix" these by reverting to the obvious approach:

- **Never downgrade Windows App SDK below 2.7.0.** 1.7's XamlCompiler fails silently.
- `Window` in WinUI 3 has no `Width`/`Height`/`FontFamily`/`Resources` XAML members. Set size in
  code: `AppWindow.Resize(new Windows.Graphics.SizeInt32(1280, 800))`.
- `ApplicationBackgroundThemeBrush` does not exist in 2.7 and crashes `Microsoft.UI.Xaml.dll`
  with `0xc000027b`. Root backgrounds use `#1C1C20` / `#161619`.
- A `SelectionChanged` handler that runs during `InitializeComponent()` caused a
  `0x80004003` XAML crash. Guard event handlers with `if (!_initialized) return;`.
- `TreeView.Items` / XAML `Items` don't work here; use `ItemsSource` or avoid `TreeView`.
- `NavigationView` needs `IsSettingsVisible="False"` (App Settings is a custom footer item).
- A `ContentDialog` must be a **direct child of the root `Grid`**; declaring it in
  `Grid.Resources`/`Window.Resources` makes `ShowAsync()` hang or fails compilation with
  `WMC0011` / `WMC0035`.
- `PreserveNewest` can leave a stale `rusty_ffi.dll`. The `CopyRustFfi` target in the `.csproj`
  uses `SkipUnchangedFiles="false"` — keep it.
- XAML indexer bindings (`{Binding Strings[Key]}`) work in this project; language switching is
  driven by `LocalizationBridge.Refresh()` + `StringCatalog.Refresh()`.

## 7. Current scope / placeholders

- Fresh app opens **HOME**: New project, Open project, recent list, target-device card.
- Left nav: Home, Editor + Project settings (visible after opening a project), **Widgets (always
  visible, also without a project)**, Assets, App Settings.
- **Editor:** 1:1 round 466×466 preview with a **Layers** panel showing layers from top to bottom.
  Widget rows have an **eye button** (hide/show), **up and down arrows** (both work — all row actions live in one horizontal stack so
  the remove button never covers the arrows) and a remove button. A hidden widget stays in
  `project.json` and in the Layers list (greyed out), but is skipped by the preview **and** by
  generated `watchface/index.js` (`Widget::visible`, default `true`; `rw_set_widget_visibility`).
  Double-clicking empty space in the editor or elsewhere in the window clears the layer selection
  (inspector hides). A layer selected in the Layers list gets the highest `ZIndex` on `OutlineCanvas`,
  so dragging/scaling applies to it even if another layer is visually on top. Selecting a row drives
  the right-hand **inspector** with that widget's settings
  (preset/custom format, font, font size, color, X/Y, alignment, text fit), auto-saved via `rw_set_widget`.
  The **Font** ComboBox lists system font plus every installed font; the preview re-renders with the
  selected TTF immediately (no UI-only zoom or substitute font).
  Text widgets also have a draggable/resizable frame (ramka) in the preview; double-clicking it
  toggles red scale mode (`rw_frame_geometry` + `auto_scale`). Removing the last widget is blocked.
  The preview renders the constant black background and all widgets in exactly the order exported to `watchface/index.js`.
  A **Preview options** button (ellipsis icon) sits at the right end of the preview's bottom bar;
  its flyout has two checkable `ToggleMenuFlyoutItem`s: **Preview grid** (shows the 16 px grid via
  `GridLinesCanvas` and enables snapping — `rw_frame_geometry` receives `snap: true`) and
  **Unselected widgets outline** (shows the subtle frame around widgets that are not selected;
  default on). Both persist in `AppSettingsStore` (`snapToGrid` / `showUnselectedOutline` in
  settings.json). Flyout item labels are set in code (`RefreshPreviewOptionLabels`) because a
  flyout does not inherit the page's `DataContext`. Dragging also snaps the
  widget's **left/center/right (or top/center/bottom)** to the canvas edges/center and to other
  widgets' edges/centers, showing dashed red `GuideVertical`/`GuideHorizontal` lines — including
  **center-to-center** alignment with the watchface and with other widgets. `resolve_axis` in
  `widgets.rs` checks alignment on the **pre-grid position**: the 16 px grid moves a widget by up
  to 8 px, which would otherwise exceed the 5 px `ALIGN_THRESHOLD` (e.g. a 420 px widget could
  never center on x=233 with the grid on). Alignment wins over the grid, so center snapping works
  with **Preview grid either on or off**. The **Text alignment** section has one row of arrow/circle icon buttons: left arrow,
  circle, right arrow, then up arrow, circle, down arrow. The **X and Y sliders sit side by side**,
  each with its live numeric value above it: a **single tap copies** that number to the clipboard,
  a **double tap turns it into an editable field** (Enter commits, Escape cancels, value clamped to
  `0…466−w/h`). Under the **Center on watchface** button are two side-by-side buttons — **Horizontal**
  and **Vertical** — that center only that axis (`(466−w)/2` or `(466−h)/2`). The X/Y bars and the whole
  centering group live in `WidgetInspector` and are **shared by both digital and analog widgets**;
  digital-only controls are wrapped in `DigitalSettingsPanel` and `DigitalAlignPanel`, which are
  collapsed for analog layers. The old vertical-align ComboBox is gone. The inspector's `ScrollViewer`
  reserves a 14 px right margin on its content so the overlay scrollbar never covers controls.
- **Widgets tab** is always visible and is a read-only catalog of supported widget types:
  **Digital time** and **Analog time** in the **Time** category; **Date**, **Day of week (number)**,
  **Week of year**, **Day of year**, **Lunar date**, **Festival** and **World clock** in the new
  **Date** category (`CATEGORY_DATE`); **Text (user defined)** in the **Text** category and
  **Image** (`images`) in the new **Images** category (`CATEGORY_IMAGES`);
  **Step counter** plus **Distance** in the **Distance**
  category (`CATEGORY_DISTANCE`), and **Heart rate, Stress, PAI, Skin temperature, Sleep, Calories**
  and **SpO₂** in the **Health** category (`CATEGORY_HEALTH`); **Battery** and four status icons
  (**DND**, **LOCK**, **BLUETOOTH**, **ALARM**) directly in the new **System** category
  (`CATEGORY_SYSTEM`) — there is no second-level STATUS subgroup. Categories are shown as plain headers
  with every widget visible — no arrow to click.
  Adding happens from the **+ Add widget** button at the right end of the editor's Preview bar;
  its Flyout has a search box and a list of collapsible category groups (collapsed by default,
  search auto-expands matches). The `date` catalog entry creates a `WidgetKind::DateText` with
  `DataSource::Date` and the default pattern `MMM D`; the inspector has an editable **Format** box
  (and date-specific presets/hint) just like the time widget. `crates/rusty-core/src/date_format.rs`
  is the single source of truth for the pattern (`D`/`DD`, `M`/`MM`, `MMM`/`mmm`, `MMMM`/`mmmm`,
  `DDD`/`ddd`, `DDDD`/`dddd`, `YY`/`YYYY`, `\` escapes). **Letter case controls name case**: uppercase
  tokens give capitalized names (`Jan`, `Mon`, `January`, `Monday`), lowercase tokens give lowercase
  names (`jan`, `mon`, `january`, `monday`), and five/six repeated letters give UPPERCASE names.
  Numeric tokens work in either case. Weekday names come from `locale.rs` tables indexed by
  `Date.getDay()` (0 = Sunday); Rust computes the same index with Sakamoto's algorithm so the preview
  matches the exported JS. The editor preview uses `rw_format_date` and generated JS uses
  `js_expression_localized`, so both stay 1:1 and follow the watch's runtime language via `L.months` /
  `L.monthsFull` / `L.days` / `L.daysFull`. The pattern is stored in the existing `Widget::time_format` field (the Rust parser is
  chosen by widget kind), so the project schema stays unchanged.
  The **Date** category also holds six generated text fields with no editable pattern: `weekday-number`
  (number 1–7 computed from the TIME sensor's `year`/`month`/`day` with the shared Sakamoto algorithm —
  **not** from `TIME.week`, whose first day differs between ZeppOS API versions), `week-number` (ISO week from `date_format::iso_week` /
  `JS_ISO_WEEK`), `day-of-year` (`date_format::day_of_year` / `JS_DAY_OF_YEAR`), `lunar-date`
  (`TIME.lunar_month/lunar_day`), `festival` (`TIME.solar_festival || lunar_festival ||
  lunar_solar_term` — lunar festivals are Chinese-language only) and `world-clock`
  (`hmSensor.id.WORLD_CLOCK`, `init()`/`getWorldClockInfo(0)`/`uninit()`, city + `HH:MM`). They are separate
  `WidgetKind`s and `DataSource`s; codegen updates them in the clock `tick`, and the editor preview calls
  `rw_format_date_field` so Rust and JS use the same algorithms. Sensor-only fields show `--` on the PC.
  **First day of week** is an app setting (default `monday`, alternative `sunday`) stored in
  `Project.first_day_of_week` via `rw_set_first_day_of_week` and passed to `rw_format_date_field`:
  Monday-first gives 1 = Monday … 7 = Sunday (Tuesday = **2**), Sunday-first gives 1 = Sunday …
  7 = Saturday. Changing it updates the preview and re-exports the project immediately, so the ZIP
  and the simulator show the same numbering. Do not reintroduce `t.week` here.
  The `text` catalog entry creates a
  plain `WidgetKind::Text` with **no `dataSource` and no `timeFormat`**: its content lives in
  `Widget::text`, editable in the inspector's **Text content** box (`UserTextBox`) and saved through
  `rw_set_widget` with the `text` key. The preview shows it directly and codegen emits a static
  `text: "..."` in `watchface/index.js` (no timer/listener), so the exported watchface displays the
  typed text verbatim. For this widget the inspector hides the Format preset/box and shows the
  text field instead. The `steps` catalog entry creates a `WidgetKind::StepText` with
  `DataSource::Step`, initial `text: "0"` and no `timeFormat`; `layers::describe` reports its kind as
  `steps`. Codegen emits classic `hmSensor.createSensor(hmSensor.id.STEP)`, an `hmSensor.event.CHANGE`
  listener that writes `String(step.current)` into the TEXT widget on every step change (plus a
  refresh in `tick`), and removes the listener in `onDestroy`. Sensor creation is isolated in
  `try/catch`, so a missing STEP sensor never kills the watchface — it just stays at `"0"`. The
  inspector hides Format/Text content and shows a `StepCounterHint` instead; the PC preview shows
  `"0"` because there is no step sensor. Localized keys: `LayerSteps`, `StepWidgetCardTitle/Desc`,
  `StepCounterHint`. The `distance` catalog entry creates a `WidgetKind::DistanceText` with
  `DataSource::Distance` and initial text `0.00 km` / `0.00 mi`. The watch reads metres from
  `hmSensor.id.DISTANCE`; codegen emits `(_meters / 1000).toFixed(2) + ' km'` for metric and
  `/ 1609.344 ... ' mi'` for imperial, with a `CHANGE` listener and `onDestroy` cleanup isolated in
  `try/catch` just like the step sensor. `Project.measurement_system` stores the
  **app-wide default** from App Settings (`AppSettingsStore.MeasurementSystem`); the default value is
  **`watch`** — the generated JS calls `hmSetting.getMileageUnit()` once at build (`_initMileageUnit`,
  guarded by `try/catch` and `typeof hmSetting !== 'undefined'`) and picks `/1000 + ' km'` or
  `/1609.344 + ' mi'` at runtime, so one ZIP works in both units. `metric` and `imperial` are explicit
  alternatives; apply the app default to an open project with `rw_set_measurement_system` on project
  open and whenever the setting changes. Distance widgets can override it through
  `Widget.measurement_system` (`null` = follow project default), edited by the inspector's **Use own
  unit system** checkbox plus a Watch/Metric/Imperial ComboBox; `rw_set_widget` receives
  `measurementSystem` (`watch` = follow the watch, empty string clears the override). Both codegen
  paths (sensor tick and listener) must use the same per-widget unit. The PC preview has no watch
  setting, so `Watch` uses a neutral `0.00 km` placeholder until the first sensor tick on the watch.
  The **Health** category adds seven text widgets backed by classic `hmSensor`: `HEART` (bpm),
  `STRESS` (0–100), `PAI` (`dailypai`), `BODY_TEMP` (°C), `SLEEP` (`getTotalTime()` → `H:MM`),
  `CALORIE` (kcal) and `SPO2` (%). `codegen.rs` keeps one shared `healthTick` plus one lazy
  `try/catch` getter per sensor, refreshes on each sensor's CHANGE (HEART: CURRENT) event and on
  the clock tick, and removes every listener in `onDestroy`. Missing sensors leave the neutral
  `--` preview text instead of breaking the watchface. Widgets with a unit (`HEART`, `BODY_TEMP`,
  `CALORIE`, `SPO2`) have a **Show unit** checkbox in the inspector: `Widget::show_unit` (default
  `true`) controls both the initial `-- bpm` text and the JS suffix, so disabling it emits a bare
  number. Unitless widgets (Stress, PAI, Sleep) hide that checkbox. HRV and Readiness are intentionally **not**
  in the catalog: HRV has no watchface sensor API and Readiness is only an `hmUI.data_type` value.
  More widget types will be added to the catalog only when explicitly requested.
- **Project settings** page is real: the watchface **name is shown greyed-out (read-only)** — the project
  folder and recent list depend on it, so it must never be renamed here (use Save as for a copy).
  **Author** and **version** are editable and saved explicitly with `rw_set_project_metadata`
  (`rusty-core::update_project_metadata`); version must be non-empty, ≤32 chars and whitespace-free,
  and it flows to generated `app.json` as `app.version.name`.
- **Assets page** has a **Fonts** category: list of installed fonts (with a live "Aa" sample),
  **Add font...** (.ttf picker), **Remove font** and **Open fonts folder**. The store is shared by
  all projects; the bundled default is Inter (SIL OFL 1.1, safe to commit with a project).
  It also has an **Images** category for general widget graphics and a **Hands** category for analog-watch hand sets: thumbnails, **Add hand set...** (folder picker with
  `hour.png`/`minute.png` + optional `second.png`/`cap.png`), **Remove hand set** and **Open hands folder**.
- **No background UI exists.** The background is always the fixed dark fill `FILL_RECT(0xFF05070D)`,
  matching the 100% working reference export. There is no background store, no `bg.png`, no
  background picker, no auto-scale toggle and no `rw_set_background`/`rw_backgrounds_*` FFI.
- Selecting the **analog-time layer** shows a hand-set ComboBox and one checkbox per hand (hour,
  minute, second, centre cap — only for files present in the selected set), followed by the **same
  shared X/Y bars and centering buttons as the digital widget** (one long *Wyśrodkuj na tarczy* plus
  *W poziomie* / *W pionie*). The frame can be dragged; its four corner handles sit on a separate layer outside the round clip.
  Blue mode moves the hands only — double-click toggles the red scale mode that enables resizing. Resizing keeps the frame square, so
  the hands scale uniformly and the real `w/h` is saved and emitted to the generated IMG widgets.
  The preview rotates the hand images on the clock timer so it matches `watchface/index.js` 1:1.
- **App Settings** page is real: language selector + live switching, **Open languages folder** and
  **Reload languages** buttons (catalogs live in `%APPDATA%\RustyWatch\Localization`), and a
  **Default author** field. A configured value is used as the author for new projects; an empty value falls back to
  `Environment.UserName`. Both settings live in `%APPDATA%\RustyWatch\settings.json` via
  `AppSettingsStore` (language lives there too — never overwrite one setting when saving another). It also has **Watchface preview language**
  (`auto` = Windows UI culture, `pl`, `en`) which selects the PL/EN variant rendered in the editor;
  the exported watchface always detects the watch language itself and is unaffected by this setting.
  The **Default system of units** and **First day of week** selectors are also here; both are written to
  the open project so the exported ZIP uses them (`rw_set_measurement_system` / `rw_set_first_day_of_week`).
  The app logo lives in `native/RustyWatch/Assets/`: `AppIcon.ico` (multi-size icon for the exe and the
  window title bar, regenerated from `logo/logo-{16,32,48,64,128,256}.png`) and `Logo.png` (the 256 px
  mark shown on HOME). `logo/` holds the master SVGs and the other size/transparency variants.
- App Settings (bottom) and Project settings (above it, shown only when a project is open) live in
  `NavigationView.PaneFooter`. Footer items do **not** raise `SelectionChanged`, so the constructor
  attaches `UIElement.TappedEvent` handlers with `handledEventsToo: true` to both; keep them when
  moving/renaming those items or clicks will silently stop working.
- Clicking a footer item **switches tabs**: `FooterItem_Tapped` sets `NavView.SelectedItem` to the
  hidden `BlankNavItem` (tag `none`) and calls `SetFooterHighlight`. Do **not** assign `null` to
  `SelectedItem` — the blue indicator on the previous tab stays visible in WinUI 3.
  `NavView_SelectionChanged` clears the footer highlight for every tag except `none`.
- Footer items must stay **plain `NavigationViewItem`s directly in the footer `StackPanel`** — no
  `Grid` wrappers and no hand-drawn indicator `Rectangle`s. That is what lets NavigationView apply
  its own template, compact (icon-only) state and selection visuals to them. `SetFooterHighlight`
  only sets `IsSelected` on the two items; their selected background/accent bar/bold text then come
  from the native control and are identical to the upper Home/Editor/Assets tabs.
- `OpenPaneLength="210"` is intentional: Polish labels (`Ustawienia aplikacji`, `Ustawienia projektu`)
  were clipped at the old 172px. `CompactPaneLength="52"` (not the 48px default) is a workaround for
  the WinUI 3 bug where `PaneFooter` item icons are clipped when the pane collapses
  (microsoft-ui-xaml#10415). With 52px the collapsed footer item measures the same 44×36 as the
  menu items. Do not revert either value without re-verifying EN/PL in both pane states.
- Bottom bar: Save, Save as..., **Export**, Test in emulator (all disabled until a project is
  open), Sync to Watch (informational). Save uses `rw_load_project` + `rw_save_project`; Save as
  uses `rw_save_project_as` (copies project + assets).
  **Clicking Export first generates `preview.png` + a 7-frame `preview.gif`, then opens an in-app
  `ContentDialog` over the editor** (a direct child of `RootGrid`, per the WinUI rule). While the
  frames render, the Export button shows a small `ProgressBar` (`1/7 … 7/7`); the dialog opens
  **only after** `preview.gif` exists and shows that GIF **clipped to the round watchface**
  (a `Border` with `CornerRadius="120"`) **above** the project details (name, author, version, App ID) and the
  **Export ZIP** button. The progress bar switches to indeterminate while the GIF is encoded,
  so it keeps loading for the whole generation. The GIF loops forever. Both preview files live in the
  project folder next to `project.json` and are **not** added to the ZIP. Filename comes from
  `metadata.name` via `ExportFileName()` (whitespace and forbidden characters → `.`). Preview
  rendering is **Rust-only** (`rusty-core::preview`, `rw_preview_random_series` /
  `rw_render_preview_frame` / `rw_encode_preview_gif`) — never render frames offscreen with WinUI
  controls: that previously leaked a „watchface image" artifact under the editor and swapping
  decoded images in an `Image` progressively shifted/scaled them. The editor's own preview stays
  untouched by export, and Custom Data never feeds the generated files.
- **Test in emulator** (`rw_prepare_emulator_workspace` → `zeus install`) exports the project to
  `%APPDATA%\RustyWatch\Emulator\app-<appId>`, unpacks it and installs it into the official Zepp OS
  Simulator. `ZeppSimulator` detects the install (registry/known paths), the downloaded Active 2
  image (`%APPDATA%\simulator\config.json` — `selectDeviceList` can be an object **or an array**)
  and the open watch window (`simuOpen`). If the simulator is missing, the app **only opens the
  official download page** — never bundle, mirror or silently install Zepp's proprietary binaries,
  never accept their EULA on the user's behalf (licensing: Zepp Health, own license; our GPL + Generation Exception is unaffected). `zeus` target name is
  `Amazfit Active 2 NFC (Round)` (plain `Amazfit Active 2 (Round)` is rejected by zeus 1.10.2).
  zeus can print `[ERROR]` while exiting with code 0 — treat both as failure.

## 8. Device / output verification

`deviceSource: 8913153` and the ZeppOS API versions are profile constants in
`rusty-core/src/model.rs`. Generated `app.json` is a **flat v2** manifest (`configVersion: "v2"`, no `targets`) — the watch installer rejects the v3 `targets` layout with "Failed to unzip the package". It **must** include `defaultLanguage`, `packageInfo` and all 8 Active 2 `deviceSource` values. The v3/target layout is created only inside the emulator workspace by `prepare_emulator_workspace` → `convert_export_for_zeus`.
(zeus rejects packages without it). Verify the export in the official simulator with:

```powershell
zeus install --path <unpacked-project-dir> --target "Amazfit Active 2 NFC (Round)"
```

The simulator needs the `Active 2 v1.1.0` device image downloaded inside its own download manager;
that step may require a Zepp account and is always done by the user. The emulator validates JS
runtime/build, but does not replace the final real-device check.

Generated zip layout is **flat v2**, matching the working reference export: `app.json`, `app.js`, `watchface/index.js`, `assets/*`, with explicit directory entries. Every project asset is written directly into `assets/` under a deterministic, unique hashed name (`hand_<hash>.png`, `font_<hash>.ttf`, `status_<hash>.png`, `img_<hash>.png`); `icon.png` keeps its name. There is no `bg.png`. `crates/rusty-core/src/export_paths.rs` is the single source of truth: `codegen` emits the same names in `src:`/`font:` and `package` uses them as ZIP entries, so the generated JS and the package can never disagree. `app.js` is the exact official `zeus` template (`gettextFactory`, `getPx(454)`, `getApp`/`getCurrentPage`); `app.json` has only `i18n.en-US`. Editor subfolders (`hands/<set>/…`, `status/<set>/…`) are **never** written to the ZIP; `fonts/` is the sole exception and keeps a unique hashed file name. The watch thumbnail `assets/Preview.png` stays 314 × 314 RGBA and is masked to the device screen shape (`DeviceProfile::screen_shape` in `model.rs`; `ScreenShape::Round` for Active 2 → transparent outside the circle). It is a temporary placeholder generated from the bundled `Logo.png`. `preview.gif` and the 466 × 466 `preview.png` stay local (not packaged).
ZIP directory entries are post-processed to DOS metadata (`0x41ff0010`) exactly like the reference package. Prefer performance-friendly generated JS: fewer widgets, correct AOD,
and no listener leaks.

## 9. Definition of done for a change

1. `cargo test --workspace` green (currently 138 tests: 111 core + 27 FFI), no new warnings.
2. `dotnet build .\native\RustyWatch\RustyWatch.csproj` — 0 errors, 0 warnings.
3. App launches; affected screens verified visually (English **and** Polish if UI changed).
4. Round-trip `new → export → open → export` preserves data when project model/codegen changes.
5. No hardcoded user-facing strings; no temporary debug hooks; docs (`README.md`,
   `docs/FORMAT.md`, `PLAN.md` if decisions changed) updated.
