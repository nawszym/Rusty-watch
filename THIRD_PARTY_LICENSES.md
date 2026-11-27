# Third-party components

Rusty Watch itself is licensed under **GPL-3.0-or-later with the Generation Exception**
(see `LICENSE`). Generated watchface packages are not subject to GPL.

The distributed Windows build contains or uses the following third-party components:

| Component | Licence | Notes |
|---|---|---|
| Microsoft Windows App SDK 2.7.0 (WinUI 3, WindowsAppRuntime, WebView2 loader) | MIT | Redistributed as part of the self-contained app; binaries are signed by Microsoft. |
| .NET 8 runtime | MIT | Included by `dotnet publish --self-contained`. |
| Rust standard library and crates (`serde`, `serde_json`, `zip`, `image`, `gif`, `ab_glyph`, `fastrand`, `anyhow`, `qrcode`, `base64`, `clap`) | MIT / Apache-2.0 (per crate) | Static Rust build; see each crate's `Cargo.toml` for exact terms. |
| Inter font | SIL Open Font License 1.1 | `native/RustyWatch/Assets/Fonts/OFL-Inter.txt`. |
| Noto Sans font (Latin/PL subset from the official Zepp CDN) | SIL Open Font License 1.1 | `native/RustyWatch/Assets/Fonts/OFL-NotoSans.txt`. |
| ZeppOS-style weather icons (`system-like`) | SIL Open Font License 1.1 | Reconstructed assets, see `native/RustyWatch/Assets/Weather/CREDITS.md`. |
| `Not Something` weather icons | SIL Open Font License 1.1 | Same credits file. |

Source repository for the corresponding source of this application:
https://github.com/nawszym/rusty-watch
