# Roans Terminal

A free, fully-local alternative to Termius. SSH + serial terminal with tabs,
saved hosts, and light/dark mode — built once, runs on macOS, Windows and Linux.

No account, no cloud. Everything stays on your machine.

## Features

- **SSH** (password or private-key auth) and **serial** connections.
- **Tabs** — multiple sessions side by side.
- **Host manager** — save hosts with credentials (stored in the OS keychain).
- **Light / dark theme**.
- Fully local — no server, no telemetry.

## Tech stack

| Layer | Tech |
|-------|------|
| UI | xterm.js + TypeScript (Vite) |
| Shell | [Tauri v2](https://tauri.app) |
| SSH | [russh](https://github.com/warp-tech/russh) (pure Rust) |
| Serial | [serialport](https://crates.io/crates/serialport) |
| Secrets | OS keychain via [keyring](https://crates.io/crates/keyring) |

## Build

```sh
npm install
npm run build          # frontend
cd src-tauri && cargo build --release
```

Or via the bundled Tauri CLI:

```sh
npm run tauri build
```

### Platform prerequisites

- **Linux:** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
- **macOS / Windows:** Xcode / MSVC toolchain (handled by CI).

## Releases

GitHub Actions builds installers for all three platforms on every `v*` tag
(or manual dispatch). Artifacts land on the draft release.

## Notes / honest limits

- Host keys are trusted on first connect (no pinned-host-key verification yet).
- Secrets live in the OS keychain (Keychain / Credential Manager / Secret Service).
- Serial resize is a no-op (serial has no PTY); SSH supports live resize.
