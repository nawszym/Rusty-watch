# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- The workspace may not be initialized as a git repo yet. If `git status`/`git diff` are unavailable,
  inspect files directly. Do not commit with this session; the owner handles git operations.
- Use PowerShell on Windows. Prefix .NET commands with
  `$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH`.
- Verify changes with both `cargo test --workspace` and `dotnet build .\native\RustyWatch\RustyWatch.csproj`
  before reporting completion. The build runs `cargo build -p rusty-ffi` automatically.
- Treat bundled `native/RustyWatch/Localization/en.json` and `pl.json` as a pair: any new key must
  exist in both. They are only *seeds* — runtime catalogs are loaded from
  `%APPDATA%\RustyWatch\Localization`, seeded on first launch and editable by the user. Rust error
  patterns belong in `LocalizationService.Error`.
- When adding a `rw_*` export, update `crates/rusty-ffi/src/lib.rs` **and**
  `native/RustyWatch/RustNative.cs` in the same change.
- WinUI 3 is the only frontend; the old Tauri prototype was removed — do not reintroduce it.
  Do not emit `.zpk`, and do not copy SashaCX75 source or assets.
- Avoid temporary `RW_DEBUG_*` startup hooks in shipped code — use them only in throwaway
  verification scripts outside the repo, then remove them.
