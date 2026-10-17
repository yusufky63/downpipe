# Third-party notices

Downpipe is licensed under Apache-2.0. It depends on the open-source projects below, each under its own license. This file covers the dependencies of the code in this repository; it was generated from `cargo metadata` and `pnpm licenses list --prod` on 2026-10-17 and must be regenerated before each release (a release build will also ship an SPDX SBOM).

No external programs (yt-dlp, FFmpeg, aria2) are bundled.

## Rust: direct dependencies

| Crate | Version | License |
|---|---|---|
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| axum (test server only) | 0.8.9 | MIT |
| bytes | 1.12.1 | MIT |
| clap | 4.6.7 | MIT OR Apache-2.0 |
| dirs | 7.0.0 | MIT OR Apache-2.0 |
| fastrand | 2.5.0 | Apache-2.0 OR MIT |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| hex | 0.4.3 | MIT OR Apache-2.0 |
| httpdate | 1.0.3 | MIT OR Apache-2.0 |
| indicatif | 0.18.6 | MIT |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| reqwest | 0.13.5 | MIT OR Apache-2.0 |
| rusqlite (bundles SQLite, public domain) | 0.40.2 | MIT |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha2 | 0.11.1 | MIT OR Apache-2.0 |
| tauri | 2.12.4 | Apache-2.0 OR MIT |
| tauri-build | 2.7.3 | Apache-2.0 OR MIT |
| tauri-plugin-dialog | 2.8.3 | Apache-2.0 OR MIT |
| tauri-plugin-opener | 2.7.2 | Apache-2.0 OR MIT |
| tauri-plugin-single-instance | 2.5.4 | Apache-2.0 OR MIT |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| tokio | 1.53.3 | MIT |
| tokio-util | 0.7.20 | MIT |
| tracing | 0.1.44 | MIT |
| tracing-subscriber | 0.3.23 | MIT |
| url | 2.5.8 | MIT OR Apache-2.0 |
| uuid | 1.28.0 | Apache-2.0 OR MIT |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |

## Rust: all transitive dependencies by license

563 crates in total. Most are MIT and/or Apache-2.0; the others are permissive (BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, 0BSD, CC0-1.0, BSL-1.0, CDLA-Permissive-2.0) except for these weak-copyleft crates, all pulled in by Tauri's HTML tooling or `dirs`:

| Crate | Version | License |
|---|---|---|
| cssparser | 0.37.0 | MPL-2.0 |
| cssparser-macros | 0.7.1 | MPL-2.0 |
| dtoa-short | 0.3.5 | MPL-2.0 |
| option-ext | 0.2.0 | MPL-2.0 |
| selectors | 0.38.0 | MPL-2.0 |
| webpki-root-certs (Mozilla CA list) | 1.0.9 | CDLA-Permissive-2.0 |

MPL-2.0 is file-level copyleft: these crates are used unmodified, and their source is available from crates.io.

## JavaScript (desktop UI, shipped)

| Package | Version | License |
|---|---|---|
| react | 19.3.1 | MIT |
| react-dom | 19.3.1 | MIT |
| scheduler | 0.28.0 | MIT |
| @tauri-apps/api | 2.12.4 | Apache-2.0 OR MIT |
| lucide-react | 1.50.0 | ISC |

## JavaScript (browser extension)

The extension bundle contains only Downpipe's own code. Its build uses esbuild 0.28.2 (MIT) and the type definitions in @types/chrome 0.3.4 (MIT); neither is shipped.

## Fonts (bundled with the desktop UI and the extension popup)

| Font | Package | License |
|---|---|---|
| DM Sans | @fontsource/dm-sans 5.3.0 | SIL Open Font License 1.1 |
| Space Grotesk | @fontsource/space-grotesk 5.3.0 | SIL Open Font License 1.1 |
| JetBrains Mono | @fontsource/jetbrains-mono 5.3.0 | SIL Open Font License 1.1 |
