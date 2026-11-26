# CLAUDE.md

**Read [`AGENTS.md`](AGENTS.md) first.** It is the single source of truth for architecture,
build commands, invariants and WinUI 3 workarounds. Everything below is Claude Code specific.

## Environment

- Use PowerShell on Windows. In this shell, prefix .NET commands with:
  `$env:PATH = 'C:\Program Files\dotnet;' + $env:PATH`
- The workspace may not be initialized as a git repository. If `git status` / `git diff` are
  unavailable, inspect files directly. **Do not commit** — the owner handles git operations.

## Verification before reporting completion

Run both toolchains, even for a small change:

```powershell
cargo test --workspace
dotnet build .\native\RustyWatch\RustyWatch.csproj
```

`dotnet build` already runs `cargo build -p rusty-ffi`. Expected result: 0 errors and
0 warnings on both. Current test count: 168 core + 38 FFI.

## Repository-specific habits

- Treat `native/RustyWatch/Localization/en.json` and `pl.json` as a pair: every new key must
  exist in both. They are only first-launch seeds; runtime catalogs are loaded from
  `%APPDATA%\RustyWatch\Localization`. Rust error patterns belong in `LocalizationService.Error`.
- When adding a `rw_*` export, change `crates/rusty-ffi/src/lib.rs` **and**
  `native/RustyWatch/RustNative.cs` in the same change.
- WinUI 3 is the only frontend — the old Tauri/React prototype was removed, do not reintroduce
  it. Export `.zip` only, never `.zpk`. Do not copy SashaCX75 source code or assets.
- Do not leave `RW_DEBUG_*` startup switches in shipped code. Use throwaway scripts outside the
  repository for visual verification, then remove them.
- Documentation lives in `README.md` (English only), `docs/TECHNICAL.md`, `docs/FORMAT.md`,
  `PLAN.md` and `AGENTS.md`. There is no Polish README and no `Readme.base` file.
