# GitHub Copilot instructions — Rusty Watch

Full instructions live in [`AGENTS.md`](../AGENTS.md) at the repository root. Read it before
making changes; it is the only complete agent guide. This file lists the key points.

- Native Windows 11 watchface editor for **Amazfit Active 2 (Round)** — 466 × 466, Zepp OS 4.x,
  `deviceSource: 8913153`. Export **`.zip` only**, never `.zpk`.
- The only frontend is **WinUI 3 / Windows App SDK 2.7.0** in `native/RustyWatch/` (C#/XAML).
  The Tauri/React prototype was removed — do not reintroduce a web UI.
- Rust workspace: `rusty-core` (pure logic/codegen, no UI dependencies), `rusty-cli`,
  `rusty-ffi` (C ABI with 66 `rw_*` exports; JSON UTF-8, free results with `rw_free_string`).
  The C# bridge is `native/RustyWatch/RustNative.cs`; update both sides when adding an export.
- Feature flow: core → FFI → native UI. Verify with `cargo test --workspace` and
  `dotnet build .\native\RustyWatch\RustyWatch.csproj`. Prefix .NET commands in PowerShell with
  `$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH`. Expect 0 errors / 0 warnings.
- All user-facing strings come from bundled `native/RustyWatch/Localization/en.json` and
  `pl.json` (keep keys in sync). They are first-launch seeds; runtime catalogs are loaded from
  `%APPDATA%\RustyWatch\Localization`. Route Rust error patterns through `LocalizationService.Error`.
- Generated JS uses the classic Watchface API (`hmUI`, `hmSensor`, `timer`, `WatchFace({...})`),
  never `@zos/ui` / `@zos/sensor`. It emits `hmUI.show_level.ONLY_NORMAL` always; AOD comes from
  `WIDGET_DELEGATE` + `hmSetting.getScreenType()`. No `Date` API anywhere in generated JS.
- The editor preview and generated `preview.png` / `preview.gif` are rendered in Rust only, with
  deterministic data. Custom editor data never changes generated previews or the export.
- Licence: GPL-3.0-or-later + Generation Exception. Do not copy SashaCX75 code or assets.
  Do not add `RW_DEBUG_*` hooks, downgrade Windows App SDK, or change the `CopyRustFfi` target
  without reading `AGENTS.md` section 6.
