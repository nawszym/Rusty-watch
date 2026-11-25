# GitHub Copilot instructions — Rusty Watch

Full instructions live in [`AGENTS.md`](../AGENTS.md) at the repository root. Read it before
making changes. Key points:

- Native Windows 11 watchface editor for **Amazfit Active 2 (Round)** (466×466, ZeppOS 4.x,
  `deviceSource: 8913153`). Export **`.zip` only**, never `.zpk`.
- The only UI is **WinUI 3 / Windows App SDK 2.7.0** in `native/RustyWatch/` (C#/XAML).
  The old Tauri prototype was removed — do not reintroduce a web-based frontend.
- Rust workspace: `crates/rusty-core` (logic/codegen, no UI deps), `crates/rusty-cli`,
  `crates/rusty-ffi` (C ABI `rw_*`, JSON UTF-8, free via `rw_free_string`).
  C# bridge: `native/RustyWatch/RustNative.cs` — update both when adding an export.
- Feature flow: core → ffi → native UI, then verify with
  `cargo test --workspace` and `dotnet build .\native\RustyWatch\RustyWatch.csproj`
  (prefix `$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH`). Expected: 0 errors / 0 warnings.
- All user-facing strings must come from bundled
  `native/RustyWatch/Localization/en.json` + `pl.json` (keep keys in sync). These are first-launch
  seeds; runtime catalogs are read from `%APPDATA%\RustyWatch\Localization`. Rust error patterns go
  through `LocalizationService.Error`.
- Generated JS uses the classic Watchface API (`hmUI`, `hmSensor`, `timer`, `WatchFace({...})`) —
  NOT `@zos/ui` / `@zos/sensor` (Mini Programs). Export always emits `hmUI.show_level.ONLY_NORMAL`;
  AOD is driven by `WIDGET_DELEGATE` + `hmSetting.getScreenType()`, not `ONAL_AOD`.
- Licence GPL-3.0-or-later + Generation Exception; do not copy SashaCX75 code/assets.
- Do not reintroduce temporary `RW_DEBUG_*` hooks, downgrade Windows App SDK, or change the
  `CopyRustFfi` target without reading `AGENTS.md` section 6.
