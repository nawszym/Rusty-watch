<div align="center">

<img src="logo/github-avatar.png" alt="Rusty Watch logo" width="150" />

# Rusty Watch

**A native watchface editor for Amazfit Active 2 (Round) · Zepp OS 4.x**

Build watchfaces on Windows 11 in a true **466 × 466 preview**, arrange layers, attach
assets and export a ready-to-install **`.zip`** — without writing ZeppOS code by hand.

[![Rust](https://img.shields.io/badge/Rust-workspace-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![C#](https://img.shields.io/badge/C%23-WinUI%203-9B4F96?logo=csharp&logoColor=white)](https://dotnet.microsoft.com/)
[![Windows App SDK](https://img.shields.io/badge/Windows%20App%20SDK-2.7.0-0F6CBD?logo=windows&logoColor=white)](https://learn.microsoft.com/windows/apps/)
[![Zepp OS](https://img.shields.io/badge/Zepp%20OS-4.x-38B48B)](https://docs.zepp.com/)
[![Windows](https://img.shields.io/badge/Windows-11-0078D6?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/license-GPL--3.0--or--later-4B8BBE)](LICENSE)

**[Technical deep-dive](docs/TECHNICAL.md)** · **[Project format](docs/FORMAT.md)** · **[Plan](PLAN.md)** · **[For AI agents](AGENTS.md)**

</div>

---

## What Rusty Watch is

Rusty Watch targets exactly one device: the **Amazfit Active 2 (Round)** — a round
466 × 466 display, Zepp OS 4.x, `deviceSource: 8913153`. It is an independent
implementation inspired by the [SashaCX75 editor](https://github.com/SashaCX75/Watch-face-editor-for-Amazfit-watch-on-ZeppOS);
no source code or assets were copied from that project.

The application is a **native WinUI 3** program. The Windows UI is the only frontend;
an older Tauri/React prototype was removed and is not coming back. All watchface logic
lives in a Rust workspace and reaches C# through a small C ABI bridge. The generated
package uses the classic ZeppOS Watchface API (`hmUI`, `hmSensor`, `timer`,
`WatchFace({...})`), not the Mini Program `@zos/*` modules.

## Highlights

| | |
|---|---|
| **1:1 preview** | Round 466 × 466 canvas with a fixed black background; every widget drawn in the same order as in the exported package. |
| **Layers** | Reorder, hide and remove layers. Hiding keeps the widget in `project.json` but removes it from the preview and generated JS. |
| **Frames & snapping** | Drag to move, drag a corner to resize, 16 px grid, alignment guides to canvas/widget edges and centres, per-widget opacity (0–255). |
| **Clocks** | Digital time with an editable pattern, and analog time driven by PNG hand sets. |
| **Data widgets** | Date and derived date fields, steps, distance, heart rate, stress, PAI, sleep, SpO₂, temperature, calories and goal targets. |
| **Progress & shortcuts** | Arc and bar progress driven by watch sensors and goals; invisible tap zones that open watch apps or data screens. |
| **Assets** | Shared libraries of TTF fonts, images, hand sets, status icon packs and weather icons; selected assets are copied into each project so it stays self-contained. |
| **Languages** | English and Polish UI catalogs, plus a separate setting for the language shown in generated previews. The watchface itself detects the watch language at runtime. |
| **Export** | Deterministic `preview.png` and an 8-frame looping `preview.gif`, a flat v2 `.zip`, and a local QR code for phone installation. |

## Widget catalog

The left-hand Widgets tab is a read-only catalog. New widgets are added from the
**+ Add widget** button on the preview bar.

- **Time** — Digital time, Analog time.
- **Date** — Date (editable pattern), Day of week (number), Week of year, Day of year, Lunar date, Festival, World clock.
- **Text** — Text (user defined).
- **Image** — a picture fitted into a blue frame; double-click the frame to stretch it while resizing.
- **Distance** — Step counter, Distance (km or mi, following the watch's unit setting).
- **Goals** — Step target, Calorie target, Stand target, Fat burning target.
- **Health** — Heart rate (current and last valid), minimum/maximum/average heart rate, Stress, PAI (today, 7 days, week), Skin temperature, Calories, SpO₂ (current and hourly average), Stand hours, Fat burning.
- **Sleep** — Sleep duration, Sleep score, Deep/Light/REM sleep, Time awake, Sleep start/end, Sleep target.
- **Progress** — Arc (square, one size slider) and Bar (editable width and height), with sensor-backed sources, optional track and rounded ends.
- **Shortcuts** — Invisible tap zones for battery, heart rate, stress, sleep, PAI, SpO₂, weather, alarm, stopwatch, countdown, sports, breathing and moon phase. They are drawn as a sea-breeze hatch in the editor only.
- **Weather** — Condition, city, firmware-provided current temperature, high/low, sunrise/sunset, time to sunrise/sunset and 29 weather icons. The icon has a 24–256 px SIZE slider (square, centred); resizing re-scales all 29 icons of the selected pack. Forecast-backed fields can select a day from today to +4.
- **System** — Battery, and four status icons (DND, LOCK, BLUETOOTH, ALARM) rendered by the watch firmware itself.

Heart-rate text and heart-rate progress always read the **last registered measurement**
(`HEART.last` on the `LAST` event) rather than continuous measurement. Goal-based
progress divides by the watch's own `.target`, so 2500 of 5000 steps draws half the arc.
Some sensors (Stand, Fat burning) may be absent on this watch; in that case the
watchface keeps the project's placeholder value and the editor shows a warning.

## Asset libraries

Application data lives in `%APPDATA%\RustyWatch`, so it survives updates and moving
the program folder.

| Library | Contents |
|---|---|
| `Fonts` | TTF fonts. New projects and widgets use the watch system font (Noto Sans) by default; Inter and other TTFs can be selected manually and are exported into the project. |
| `Images` | Graphics for the Image widget. |
| `Hands` | Analog hand sets; Classic and Light are bundled. |
| `Status` | STATUS icon packs; Classic is bundled. |
| `Weather` | Weather icon packs; `system-like` (default) and Not Something are bundled with 29 icons each. |
| `Localization` | UI language catalogs. Drop in another `<code>.json` and use **Reload languages** — no rebuild needed. |

Selecting an asset copies it into the project folder and registers it as a project
asset, so **Save as…** and ZIP export never depend on the shared library. Hand PNGs
are cropped to a tight alpha bounding box with an 8 px margin and resampled in
premultiplied-alpha space: ZeppOS cannot scale an `IMG` at runtime, and straight-alpha
resampling produced white fringes on the real watch.

Choosing **System font** in a text widget leaves `font` unset: the exported watchface omits
the font property and the watch renders with its own default font, **Noto Sans**. The editor
previews that choice with a bundled subset of the official Noto Sans TTF, so the preview and
the watch use identical glyph metrics. Custom fonts are exported as project assets. The Assets
page shows the watch system font as its first, marked-as-default tile (preview in Noto Sans);
Inter has no badge, though it remains a protected bundled font.

## Getting started

### Requirements

- Windows 11
- Rust (workspace in `Cargo.toml`)
- .NET SDK 10 to build and .NET Runtime 8 to run
- Windows App SDK 2.7.0 (restored by NuGet; the app is self-contained)
- Optional: the official Zepp OS Simulator with the *Active 2 v1.1.0* device image and `zeus`

### Build and run

PowerShell needs `dotnet` on `PATH` in this shell:

```powershell
cargo test --workspace

$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH
dotnet build .\native\RustyWatch\RustyWatch.csproj
.\native\RustyWatch\bin\Debug\net8.0-windows10.0.26100.0\win-x64\RustyWatch.exe
```

`dotnet build` first runs `cargo build -p rusty-ffi` and copies `rusty_ffi.dll` into the
output directory; `SkipUnchangedFiles="false"` prevents a stale DLL from surviving.
Both toolchains should report **0 errors / 0 warnings**.

### Command line

```powershell
cargo run -p rusty-cli -- new .\my-watch --name "My Watch" --author "Jane Doe" --app-id 12345678
cargo run -p rusty-cli -- validate .\my-watch\project.json
cargo run -p rusty-cli -- export .\my-watch\project.json
cargo run -p rusty-cli -- inspect .\my-watch\My_Watch.zip
```

`--author` and `--app-id` are optional. The name, author and App ID end up in
`app.json` as `appName`, `appId` and `i18n.*.appName`; `vender` is always `zepp` for
parity with working reference exports.

## The edit → export → watch workflow

1. **Create** a project on the Home page. Its folder is created directly under the
   projects root configured in App Settings (by default `%APPDATA%\RustyWatch\Projects`).
2. **Add widgets** from the flyout above the preview and tune them in the inspector.
   Edits autosave to `project.json`.
3. **Add assets** in the Assets page. Installed fonts, images, hands, status icons and
   weather icons are shared; project copies make each watchface portable.
4. **Export** from the bottom bar. The button shows `1/8 … 8/8` while the preview is
   rendered, then the export dialog opens with a round-cropped animated GIF, the
   project details and two actions: **Export ZIP** and **QR code**.
5. **Install** the ZIP in the simulator or on the watch.

`preview.png` is a branded composition: the 21:37:40 frame inside the gear-outlayer
logo frame with black safe areas. `preview.gif` has eight infinite-looping frames
(~1.7 s each); the composed image is the first frame, followed by seven raw frames.
Both files are written next to `project.json` and are **not** included in the ZIP.
They are rendered entirely in Rust — never with offscreen WinUI controls — and ignore
the editor's Custom Data so a project always produces the same preview.

The ZIP is written straight into the project folder. Its layout is a **flat v2**
manifest:

```text
app.json                       # flat v2, defaultLanguage, packageInfo, Active 2 deviceSources
app.js                         # entry point
watchface/index.js             # generated watchface code (hmUI / hmSensor / timer)
assets/Preview.png             # branded preview rescaled to 314 × 314, round-masked
assets/fonts/font_<hash>.ttf    # fonts actually used by the project
assets/hand_<hash>.png
assets/status_<hash>.png
assets/img_<hash>.png
assets/weather_<hash>.png
```

### Install on the phone with the QR code

Clicking **Export** prepares a fresh ZIP, starts a small local HTTP server on a free
port and generates the code. The QR image stays hidden until you click **QR code**.
The URL has the form `zpkd1://<LAN-IPv4>:<port>/<name>.zip`, the same scheme used by
`zeus preview --local`. No Zepp account, cloud service, token or third-party QR
service is involved; the phone and PC must be on the same LAN and the export window
must stay open. The server accepts query strings and URL-encoded names.

Scan it from **Zepp App → Developer Mode → Scan**. Whether the flat v2 ZIP is accepted
directly by the app, or needs a `zeus`-built artifact first, still needs a final
real-device check.

### Test in the simulator

Rust exports to `%APPDATA%\RustyWatch\Emulator\app-<appId>`, unpacks the package and
installs it with `zeus install --target "Amazfit Active 2 NFC (Round)"`. The simulator
is Zepp Health software under its own licence: Rusty Watch never bundles, mirrors or
silently installs it, and only opens the official download page when it is missing.
Simulator testing validates the JS build/runtime but does not replace a real-device test.

## Languages and preview data

The editor ships English and Polish UI catalogs and switches between them instantly.
English is the default on first launch; the app never inherits the Windows UI language.
A separate **Watchface preview language** setting (`en` or `pl`) selects the variant
shown in the editor and baked into generated `preview.png` / `preview.gif`. On the
watch, the language is detected at runtime, so one ZIP works for both.

By default the editor shows predefined sample values (31 July 2022, 21:37:40).
**Live preview** switches only time and date to the real system clock; weather and
sensor values remain predefined. Custom values edited in the gear dialog are stored
in App Settings, never in `project.json`, and never affect generated preview files
or the exported watchface.

## Repository layout

```text
crates/
  rusty-core/src/     model, validation, codegen, ZIP, asset stores, time/date engine, preview, QR
  rusty-cli/src/      new | validate | export | inspect
  rusty-ffi/src/      the C ABI bridge (66 rw_* exports)
native/RustyWatch/    the only frontend: WinUI 3 / C# / XAML
  MainWindow.xaml(.cs), RustNative.cs, PreviewFonts.cs, AppPaths.cs, Localization/, Assets/
docs/
  TECHNICAL.md        how the code actually works
  FORMAT.md           project.json and package format
logo/                 vector masters and generated icons
PLAN.md               goals and confirmed decisions (historical)
AGENTS.md             rules for AI agents
LICENSE               GPL-3.0-or-later + Generation Exception
```

`rusty-core` has no UI dependency. C# contains no business logic: it calls `rw_*`,
parses JSON and updates the UI. Each FFI call returns `{"ok":true,"data":…}` or
`{"ok":false,"error":"…"}`, and the caller releases the string with `rw_free_string`.

## Verification

1. `cargo test --workspace` — 167 core + 38 FFI tests, no warnings.
2. `dotnet build .\native\RustyWatch\RustyWatch.csproj` — 0 errors, 0 warnings.
3. Launch the app and check affected screens in both English and Polish.
4. Round-trip `new → export → open → export` without data loss.
5. Verify in the simulator, then on a physical watch.

## Documentation

- [Technical deep-dive](docs/TECHNICAL.md) — module-by-module explanation of the pipeline.
- [Project format](docs/FORMAT.md) — `project.json` and the exported package.
- [Plan](PLAN.md) — goals, scope and confirmed decisions.
- [AI agent rules](AGENTS.md) — architecture, commands and hard-won workarounds.

## License

**GPL-3.0-or-later with a Generation Exception** — full text in [LICENSE](LICENSE).

The editor source and its derivatives remain open source. The exception applies only
to generated `.zip` watchfaces: you may publish those freely without releasing your
project files. The bundled Inter and Noto Sans fonts are licensed under
[SIL OFL 1.1](native/RustyWatch/Assets/Fonts/OFL-Inter.txt) and
[SIL OFL 1.1](native/RustyWatch/Assets/Fonts/OFL-NotoSans.txt); see
[font credits](native/RustyWatch/Assets/Fonts/CREDITS.md).
