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
| `crates/rusty-core/src/` | `model.rs` (Project/Widget/metadata, device constants, starter template), `project.rs` (create/load/save/save-as/folder sanitization/random App ID), `codegen.rs` (generated `app.json`, `app.js`, `watchface/index.js`), `time_format.rs` (shared pattern parser + JS expression builder), `layers.rs` (layer order + editor layer descriptors), `widgets.rs` (add/edit/remove widget), `package.rs` (zip export/inspect), `fonts.rs` (font store, TTF name parser, project attachment/pruning), `recent.rs` (recent-project store), `locale.rs` (watchface language: PL/EN tables + runtime JS detection), `images.rs` (shared image store + background preparation), `image_fit.rs` (reusable Cover/Contain/Stretch scaling), `hands.rs` (shared hand-set store + project attachment), `validate.rs` |
| `crates/rusty-cli/src/main.rs` | CLI: `new`, `validate`, `export`, `inspect` |
| `crates/rusty-ffi/src/lib.rs` | C ABI, 43 `rw_*` exports (incl. `rw_free_string`, `rw_ensure_defaults`, `rw_set_hand_visibility`); all take/return UTF-8 JSON; caller frees with `rw_free_string` |
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
   Correct AOD constant is `hmUI.show_level.ONAL_AOD` (typo is intentional, from ZeppOS).
   Align / text-style / show-level JS expressions live in `model.rs::as_js()` with an `hmUI.` prefix.
 4a. **The watchface detects its own language at runtime.** `crates/rusty-core/src/locale.rs` owns the PL/EN tables (Polish month abbreviations and AM/PM markers) and emits a JS block calling classic `hmSetting.getLanguage()` — `9` = `pl-PL`, **every other value = English**. `codegen` uses `L.months` for Date widgets and `time_format::js_expression_localized` (`L.am`/`L.pm`) for time widgets; the glyph warm-up already includes Polish diacritics. Do not bake one language into the ZIP. The editor has a **watchface preview language** setting (`auto` = Windows UI culture, `pl`, `en`) in `AppSettingsStore`, passed through `rw_format_time`/`rw_month_name`, so the preview matches whichever variant the watch will show.
5. **Layers are the single source of draw order.** `crates/rusty-core/src/layers.rs` resolves
   `Project::layer_order` (bottom → top, reserved id `"background"` plus widget ids), keeps widget
   `z` in sync, and exposes descriptors for the editor. `codegen::generate_index_js` and the WinUI
   preview must both use `layers::ordered_widgets()` / `layers::describe()`, so the preview matches
   the exported watchface. The background asset is rendered as a synthetic `widget.IMAGE` covering
   466×466. **The background is permanently locked to index 0:** `move_layer("background", …)` and
   `remove_widget("background", …)` must always fail, widgets cannot be moved below it, and the
   editor must show a native lock icon instead of arrows/remove for that row. Do not order by vector
   index or `z` separately in C#. `rw_move_layer` saves the new order immediately.
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
6a. **Fonts are one shared asset store, but projects are self-contained.** Installed fonts live in `%APPDATA%\RustyWatch\Fonts` (`rusty-core::fonts`); the bundled default is `Inter-Regular.ttf` (SIL OFL 1.1, license in `native/RustyWatch/Assets/Fonts/OFL-Inter.txt`) and is copied there by `ensure_default`. `Widget.font` stores only the file name; `None` = watch system font. Assigning a font copies it into `<project>/assets/fonts/<file>` and registers an `Asset { role: Font }`, so `save as`/ZIP export include it; `prune_unused` removes old copies when the user switches fonts. Generated JS sets `font: "fonts/<file>"` on TEXT widgets and also emits an offscreen "glyph warm-up" TEXT widget per font/size — ZeppOS otherwise fails to draw characters that appear only after a `setProperty(TEXT)` update. The WinUI preview **must** render with the same TTF (`PreviewFonts.cs` → `ms-appx:///FontCache/...`); a theme-font preview is a bug. WinUI unpackaged apps cannot load `file:///` or `%APPDATA%` fonts directly. New projects get the default font via `rw_apply_default_font`; when **opening any project** and after `rw_add_widget`, call `rw_apply_default_font_to_missing` so widgets with `font: null` receive the bundled Inter TTF too. Without it the editor fell back to Segoe UI while the watch used its system font — identical `x/y/align_h/align_v` then produced different glyph widths/baselines, which looked like broken alignment in the preview. Custom fonts are never overwritten.
6b. **Two separate graphic stores; projects stay self-contained.** Backgrounds live *only* in `%APPDATA%\RustyWatch\Backgrounds` (`rusty-core::backgrounds`: list/add/remove/set_background/clear_background); general graphics for the IMAGES widget live in `%APPDATA%\RustyWatch\Images` (`rusty-core::images`). On first launch, `backgrounds::ensure_migrated` copies images from the old shared `Images` store into `Backgrounds` once (marker file), so existing backgrounds stay available — new IMAGES imports must never leak into the background store. Selecting a background copies the original to `<project>/assets/backgrounds/<file>` (`AssetRole::Image`) and writes `<project>/assets/bg.png` (`AssetRole::Background`); `Project.background` stores `{ image, autoScale }`. **ZeppOS cannot scale an `IMG` resource at runtime** — its displayed size is the PNG's own size — so `bg.png` must be prepared to exactly 466×466 before export. With auto-scaling on, `image_fit::FitMode::Cover` centre-crops and resizes to 466×466 (opaque RGBA, no white/empty edges); off keeps native dimensions and the editor preview draws at native size too. `image_fit` is the shared scaling module — do not implement crop math in C#. The background layer stays locked in Layers, but its image and auto-scale toggle are editable in the inspector; `rw_set_background` saves immediately and the preview must be forcibly reloaded (`LoadFreshBitmap`, URI caching otherwise shows the old PNG).
   Two built-in backgrounds are generated in Rust: **black** (`generate_black_background`, the default for new projects and for "restore") and **Starry** (`generate_starry_background`, the old gradient). They are **ordinary files** in the Backgrounds store (`black.png`, `starry.png`), seeded by `ensure_builtins` at startup, listed in Assets and selectable exactly like imported backgrounds; they cannot be removed. `create_project_with` writes `assets/backgrounds/black.png` and records `background.image = "black.png"`. Reserved values `black`/`starry` and the editor's `""` are still accepted by `rw_set_background` for backward compatibility. Keep the localized labels `BackgroundNone` / `BackgroundStarry` in sync.
   The **IMAGES widget** (`WidgetKind::Images`, catalog kind `images`, new `images` category) uses that same `image_fit` module: `rw_set_widget_image` copies the chosen file from `%APPDATA%\RustyWatch\Images` into `<project>/assets/images/<widget-id>/<file>`, physically resized to the widget's `w × h` with the selected `FitMode` (`cover`/`contain`/`stretch`, default `contain`). `Widget.src` points at that prepared asset path and `Widget.image_fit` stores the mode. `rw_set_widget` re-prepares the PNG from the pristine store copy whenever geometry or fit changes; `package.rs` applies `images::export_asset_bytes` at export as a safety net for older projects. Preview draws the PNG at its natural size with a movable frame; blue mode moves, double-click toggles red scale mode. Assets page has a separate **Images** section (import/remove/open folder), and the inspector has an image picker plus a fit-mode combo.
6c. **Analog time and hand sets.** The addable widget kinds come from `widgets::supported_widgets()` as `{kind, category}`. The "+ Add widget" flyout builds collapsible `Expander` groups from that list (collapsed by default; search auto-expands categories with matches and filters items inside them), while the left Widgets tab shows the same categories always expanded as plain headers with all items visible — no arrow to click. The old time widget is **"Digital time"** and both time widgets live in category `time`; the **"Date"** text widget lives in category `text` (`WidgetKind::DateText` + `DataSource::Date`, rendered with the editable `date_format` pattern). An analog widget (`WidgetKind::AnalogTime`, addable kind `analog-time`) is a single layer whose generated JS is a group of four rotating `hmUI.widget.IMG` widgets (hour/minute/second/cap), each with `center_x/center_y` at the image centre and `angle` updated through `setProperty(hmUI.prop.MORE, …)` from the shared `tick`; the cap is static. Seconds hand ⇒ 1 s timer, otherwise MINUTEEND listener. Installed hand sets live in `%APPDATA%\RustyWatch\Hands\<id>\` (`rusty-core::hands`: list/add/remove/ensure_default); a set needs square `hour.png` + `minute.png` of equal size, `second.png`/`cap.png` optional. Bundled `Classic` (**with its subtle shadow**) and `Light` (pure white hands with a dark outline and soft glow) are committed in `native/RustyWatch/Assets/Hands/` and installed from there; the scratch original in `temporary/hands` is local-only and gitignored. `hands::ensure_default` seeds **every** valid bundled set and skips sets already installed — it must not return early after `Classic`, otherwise newly bundled sets never appear in existing installations. Selecting a set copies files to `<project>/assets/hands/<id>/`, registers `AssetRole::Hand` and prunes unused sets; ZIP stays self-contained. A **new** analog widget gets its frame centred and sized from the PNGs, but changing the set on an existing widget must preserve its current `x/y/w/h` (only the look changes); `rw_set_hands` then re-scales the new PNGs to that preserved frame via `hands::scale_project_hands`. Keep hand images smaller than the full 466×466 screen (the bundled set is 400×400) — full-screen hand bitmaps are known to cause trouble on real watches. The preview rotates `Image` elements with `RotateTransform` on the same clock timer; no UI-only zoom. Frames and their four resize handles are drawn on a separate `OutlineCanvas` **outside the round clip**, so the handles sit exactly on the layer's corners and stay grabbable even when the 400×400 frame's corners fall outside the watch screen. In the blue mode image layers (hands and future graphics) can only be moved; the red mode (double-click the frame) enables scaling. Text layers resize their canvas in blue and scale the text in red. `rw_frame_geometry` receives `square: true` for analog layers and keeps `w == h` while resizing, so hands are scaled uniformly — the real `w/h` is persisted and emitted as the IMG widgets' `w/h`/`center_x`/`center_y`. Scaling any widget image at runtime via `setProperty` is still unsupported; here the size is set when the widget is created. **`hmUI.widget.IMG` does not scale its PNG at runtime either** — it draws the image at its natural size and clips it to the widget's `w/h`. So the PNG files copied into `<project>/assets/hands/<id>/` must be physically resized to the analog widget's `w/h`: `hands::scale_project_hands` does this when geometry is saved (`rw_set_widget`) and when a hand is re-enabled (`rw_set_hand_visibility`), always starting from the pristine `%APPDATA%` store copy; `hands::export_asset_bytes` resizes in memory at export as a safety net for older projects. Without it a 400×400 PNG in a 175×175 widget shows only a tiny centre crop on the watch. Additionally `hands::prepare_hand_assets` trims transparent margins at export, but **only to a SQUARE centred on the rotation pivot** (radius = farthest opaque pixel from the centre + 1 px margin), and emits per-hand `x/y/w/h/center_x/center_y` via `generate_index_js_with_hands`. ZeppOS clips the rotated IMG to the widget's `w/h`, so cropping to a tight alpha **bounding box** is a bug: the hand swings outside that narrow rectangle when rotated and the watch shows only stubs near the pivot (verified in the simulator). With a centred square, `center_x = w/2`, `center_y = h/2`, `x + center_x == widget.x + w/2`, and the preview (which rotates the full frame around its centre) stays 1:1. Bundled hands are radial, so this still cuts decoded bitmaps ~2× for a 167 px frame (and keeps PNG files small); hands reaching the frame edge are exported uncropped as a safe fallback. Both export paths (`export_zip` and `export_zip_bytes`) must apply the same preparation. The analog inspector also has one checkbox per hand (hour, minute, second and the centre cap). A checkbox appears only when the selected set actually contains that file (hour/minute always, second/cap when installed). `rw_set_hand_visibility` → `hands::set_hand_visibility` clears the corresponding `*_src` to hide a hand and re-copies the PNG from `%APPDATA%` to re-enable it (disabled PNGs are removed from the project by `prune_unused`). Codegen emits `null` for hidden hands and skips their angle updates; validation accepts an analog widget with any subset of hands. User visibility choices are remembered across set switches for hands the new set supports.
6. **Project storage:** one folder per watchface under a user-chosen projects root:
   `<root>/<sanitized watchface name>/project.json`. Never overwrite an existing folder — return
   an error. `metadata.name/author/appId` must land in generated `app.json` as `appName`, `vender`,
   `appId`, and `i18n.*.appName`.
7. **App data lives in %APPDATA%\RustyWatch** (roaming, survives updates): `settings.json`,
   `recent.json`, the default `Projects` folder, the shared `Fonts`/`Images`/`Hands` stores, the
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
  Background is locked (native lock icon, no eye/arrows/remove); widget rows have an **eye button**
  (hide/show), **up and down arrows** (both work — all row actions live in one horizontal stack so
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
  The preview renders `bg.png` and all widgets in exactly the order exported to `watchface/index.js`.
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
  and **SpO₂** in the **Health** category (`CATEGORY_HEALTH`). Categories are shown as plain headers
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
  It also has a **Background** category: list of images with thumbnails, **Add image...**
  (PNG/JPEG picker), **Remove image**, **Use as background** and **Open images folder**, plus a
  **Hands** category for analog-watch hand sets: thumbnails, **Add hand set...** (folder picker with
  `hour.png`/`minute.png` + optional `second.png`/`cap.png`), **Remove hand set** and **Open hands folder**.
- The **background layer** remains locked in Layers (native lock icon), but selecting it shows an
  inspector with a background-image ComboBox listing every file from the Backgrounds store —
  including the seeded **Black (default)** (a fully black 466×466 PNG) and **Starry** (the former
  generated gradient) —, an **Auto-scale to
  466 × 466** toggle and **Restore black background**. Changes save immediately via
  `rw_set_background`; the preview reloads from disk because the `bg.png` path never changes.
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
  **Clicking Export simply opens an in-app `ContentDialog` over the editor** (a direct child of
  `RootGrid`, per the WinUI rule) — no background job, no progress bar, no animation and no
  `preview.gif`. The dialog has **no watchface preview at all**: it only shows the project details
  (name, author, version, App ID) loaded from `project.json` and a single **Export ZIP** button.
  The ZIP goes straight into the project folder next to `project.json` — no save picker. Filename
  comes from `metadata.name` via `ExportFileName()` (whitespace and forbidden characters → `.`,
  e.g. `Moja tarcza` → `Moja.tarcza.zip`). Do **not** reintroduce GIF/BMP generation or playback:
  rendering frames offscreen leaked a „watchface image" artifact under the editor, swapping decoded
  images in an `Image` makes WinUI progressively shift and scale them, and the extra Rust FFI only
  added dead code. The editor's own preview stays untouched by export.
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
`rusty-core/src/model.rs`. Generated `app.json` is v3 and **must** include `defaultLanguage`
(zeus rejects packages without it). Verify the export in the official simulator with:

```powershell
zeus install --path <unpacked-project-dir> --target "Amazfit Active 2 NFC (Round)"
```

The simulator needs the `Active 2 v1.1.0` device image downloaded inside its own download manager;
that step may require a Zepp account and is always done by the user. The emulator validates JS
runtime/build, but does not replace the final real-device check.

Generated zip layout: `app.json`, `app.js`, `watchface/index.js`, `assets/active2round/*`.
Prefer performance-friendly generated JS: fewer widgets, pre-rendered backgrounds, correct AOD,
and no listener leaks.

## 9. Definition of done for a change

1. `cargo test --workspace` green (currently 82 tests: 65 core + 17 FFI), no new warnings.
2. `dotnet build .\native\RustyWatch\RustyWatch.csproj` — 0 errors, 0 warnings.
3. App launches; affected screens verified visually (English **and** Polish if UI changed).
4. Round-trip `new → export → open → export` preserves data when project model/codegen changes.
5. No hardcoded user-facing strings; no temporary debug hooks; docs (`README.md`,
   `docs/FORMAT.md`, `PLAN.md` if decisions changed) updated.
