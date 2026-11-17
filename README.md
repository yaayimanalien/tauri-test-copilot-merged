# Daylight

A private encrypted journal for desktop and mobile. Svelte 5 + Vite + Tailwind v4 + shadcn-svelte for the UI, Tauri 2 + Rust for the vault, sync and biometrics.

This project combines two earlier repos: the UI from `tauri-test` and the backend and packaging from `daylight-copilot-source`.

## Run

```
bun install
bun run tauri dev
```

`bun run check` runs Svelte and TypeScript diagnostics, `bun run build` builds the frontend, `bun run tauri build` packages the app, and `cargo test --manifest-path src-tauri/Cargo.toml` runs the Rust tests.

## Continuous integration

The `Build Daylight apps` workflow runs on pushes and pull requests targeting `main`, version tags (`v*`), and manual dispatch. It checks and builds Windows, Linux, macOS, Android, and iOS on their native runners. Version-tag releases publish the Windows executable and installers, Linux DEB, and macOS DMG. Android APKs are retained as workflow artifacts for 14 days; the iOS build is unsigned and validated in CI only because Apple signing credentials are not configured.

The macOS and mobile jobs generate platform-specific icons from the committed `src-tauri/icons/128x128.png` before building.

Mobile projects are initialized by CI with the Tauri CLI. Local device builds still need the Android SDK/NDK or Xcode and appropriate signing configuration for distribution.

## Structure

- `src/app.css`: every colour, glass material, typeface and accent lives in tokens at the top. New theme = one `:root[data-theme="..."]` block.
- `src/lib/components/`: UI. `ui/` holds shadcn-svelte primitives; `views/` holds timeline, calendar, stats and settings; `live-editor.svelte` is the Obsidian-style editor.
- `src/lib/api.ts`: the only place that talks to Rust commands.
- `src/lib/journal.svelte.ts`: store with autosave, Drive sync and the draft lifecycle.
- `src-tauri/src/`: `crypto.rs` (Argon2id + XChaCha20-Poly1305), `sync.rs` + `drive.rs` (two-way Drive sync), `keystore.rs` + `biometric.rs` (quick unlock), `lib.rs` (commands).

## Security

Argon2id derives a key that wraps a random 256-bit data key; XChaCha20-Poly1305 encrypts the vault, which is rewritten through a temp file on every change. Biometric unlock (Windows Hello, Face ID/Touch ID, Android biometrics) is verified in Rust before the stored data key is read; the webview has no access to the biometric plugin. On Android the data key sits in an app-private file (`quick.key`) protected only by the app sandbox. Password unlock always remains available. Biometric unlock is local to the device and is not included in Drive backups.

The CSP allows only local resources and IPC. Images from the web are blocked, so Markdown images that point at `http(s)` URLs will not display. To allow them, add `http: https:` to `img-src` in `src-tauri/tauri.conf.json`.

## Google Drive backup

Create a Google Cloud project, enable the Drive API and create a Desktop OAuth client. Connect it from Settings; Daylight requests only the `drive.appdata` scope and stores the credentials in the encrypted vault. On a new device choose **Restore journal** on the first-run screen and use the same OAuth client and vault password. Sync and restore have not been validated against a real Google account.

macOS and Linux desktop biometric unlock is not available.
