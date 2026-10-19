# Daylight Technical Documentation

## 1. Project overview

Daylight is a private journal application for desktop and mobile. Its user
interface is built with Svelte 5 and Vite; a Tauri 2 shell exposes a Rust
backend for encrypted storage, vault lifecycle, biometric unlock, and Google
Drive backup.

The frontend never reads or writes the vault file directly. It sends typed
Tauri commands through `src/lib/api.ts`. Rust owns the decrypted session,
encryption keys, filesystem access, and Drive requests.

The application includes:

- Password-based vault creation and unlock.
- An always-ready blank journal draft, with debounced autosave.
- A reverse-chronological timeline, calendar, writing statistics, and settings.
- Markdown editing with a live-preview mode.
- Optional biometric quick unlock on Windows, iOS, and Android.
- Optional encrypted, two-way Google Drive synchronization and fresh-install
  restore.
- Idle and background auto-lock.

This source combines the componentized Svelte UI from an earlier project with
the Tauri/Rust backend and packaging work from another project.

## 2. Technology and repository layout

### Main technologies

| Layer | Technology |
| --- | --- |
| UI | Svelte 5, TypeScript, Vite |
| Styling | Tailwind CSS v4, CSS custom properties, `tw-animate-css` |
| UI primitives | shadcn-svelte-style components built on Bits UI |
| Icons and notifications | `@lucide/svelte`, `svelte-sonner` |
| Desktop/mobile shell | Tauri 2 |
| Native application logic | Rust 2021 |
| Encryption | Argon2id, XChaCha20-Poly1305 |
| Google Drive HTTP | `reqwest` with Rustls TLS |

### Important paths

```text
src/
  App.svelte                       Application lifecycle and view selection
  app.css                          Theme tokens, base styles, component styles
  main.ts                          Svelte application entry point
  lib/
    api.ts                         Typed frontend-to-Rust command facade
    journal.svelte.ts              Reactive entries, autosave, sync lifecycle
    appearance.svelte.ts            Theme/accent/font preferences
    dates.ts                        Local date grouping and labels
    stats.ts                        Derived writing statistics
    markdown.js                     Escaping Markdown renderer
    live-render.ts                  Markdown source/backdrop formatting
    components/
      entry.svelte                  Entry editor, swipe delete, context menu
      live-editor.svelte             Line-based live-preview Markdown editor
      lock-screen.svelte             Setup, password unlock, restore UI
      nav.svelte                     Main navigation
      views/
        timeline.svelte              Day-grouped journal
        calendar.svelte              Year/month calendar
        stats.svelte                 Writing statistics display
        settings.svelte              Appearance, security, backup settings
      ui/                            Reusable button, input, dialog, etc.
src-tauri/
  tauri.conf.json                   Tauri window, CSP, build and bundle config
  capabilities/default.json         Tauri window capability permissions
  Info.ios.plist                    iOS Face ID usage explanation
  src/
    lib.rs                          Commands, state, persistence, migration
    crypto.rs                       Vault format and cryptographic primitives
    biometric.rs                    Native consent/biometric verification
    keystore.rs                     Platform-specific quick-unlock key storage
    drive.rs                        OAuth PKCE and Drive REST client
    sync.rs                         Encrypted backup and conflict resolution
```

## 3. Runtime architecture

### Startup and lock lifecycle

`App.svelte` begins in a boot state and calls `vault_status`. The UI then shows
the setup screen when no vault exists, or the unlock screen when one does. A
successful setup, password unlock, biometric unlock, or restore opens the
journal by loading entries through the API facade.

While the journal is unlocked:

- A blank draft is created at the top of the current day and focused.
- The application locks after five minutes without activity.
- Returning to the app after it was hidden for more than one minute locks it.
- `Ctrl/Cmd + L` locks immediately; `Ctrl/Cmd + N` returns to a new draft.
- Drive sync is checked at app start, on focus/visibility changes, and by a
  two-minute poll.
- Before locking, pending entry writes are flushed. A final sync is attempted
  with a bounded four-second wait when Drive is connected.

`Journal` in `src/lib/journal.svelte.ts` holds UI-facing reactive state. The
authoritative unlocked payload and its data-encryption key live in Rust's
`AppState`; the frontend only sees entries returned by commands.

### Backend state and command boundary

The Rust application manages a single `Mutex<AppState>`. It contains:

- The Tauri application data directory.
- An optional unlocked `Session` containing the data key, vault header, and
  decrypted payload.
- A flag preventing overlapping Drive sync operations.

`lock_state` turns a poisoned mutex into an explicit error. Commands that do
asynchronous work generally copy the key/session data they need, release the
mutex while waiting on external work, and verify the session again before
committing results. This avoids holding the application-state lock across
network or biometric awaits and detects a session that changed during that
work.

The frontend API facade maps JavaScript method names to these registered Rust
commands:

| Frontend method | Tauri command | Purpose |
| --- | --- | --- |
| `status` | `vault_status` | Vault, biometric, and backup status |
| `setup` | `setup_vault` | Create a password-protected vault |
| `unlock` | `unlock` | Unlock with the password |
| `unlockBiometric` | `biometric_unlock` | Verify native consent and unlock |
| `lock` | `lock` | Clear the active Rust session |
| `setBiometric` | `enable_biometric_unlock` / `disable_biometric_unlock` | Configure quick unlock |
| `changePassword` | `change_password` | Re-wrap the data key |
| `list` | `list_entries` | Read entries from the unlocked payload |
| `create` | `create_entry` | Add an entry |
| `update` | `update_entry` | Change title/body |
| `remove` | `delete_entry` | Delete an entry |
| `driveConnect` | `drive_connect` | Connect a Google OAuth client |
| `driveDisconnect` | `drive_disconnect` | Remove Drive configuration |
| `sync` | `sync_now` | Synchronize the local vault |
| `restore` | `restore_from_drive` | Restore when no local vault exists |

All vault-dependent commands reject calls made while locked.

## 4. Frontend behavior

### Journal model and autosave

An entry has an ID, title, Markdown body, creation timestamp, and update
timestamp. Timestamps are Unix seconds. Entries are sorted newest first in
storage/API results; the timeline groups them by the device's local calendar
day. Older days are displayed oldest-first within each day, while today's
entries are newest-first.

The draft is a frontend-only entry until its title or body contains
non-whitespace text. On the first edit it becomes a normal entry and receives
its creation time. Each normal-entry update is debounced for 400 ms. Writes
for the same entry are chained so that a create operation completes before a
subsequent update. Save/delete errors are surfaced with toast notifications.

Deleting an entry removes it from the visible list immediately and offers an
Undo action. The Rust backend records a tombstone when Drive is configured so
the deletion can propagate to other devices. Empty saved entries are removed
when the editor loses focus or the app locks.

### Markdown editor and rendering

`live-editor.svelte` splits content into line blocks; fenced code blocks are
kept together. The block containing the caret is an editable textarea layered
over a visually formatted backdrop. Other blocks are rendered as Markdown.
Keyboard handling supports line splitting/joining, list and quote continuation,
and vertical movement between blocks.

`markdown.js` is a deliberately small renderer, not a full CommonMark
implementation. It handles headings, paragraphs and line breaks, fenced code,
quotes, ordered/unordered lists, rules, emphasis, strike-through, links, images,
and inline code. It escapes HTML input before generating markup, so journal
text cannot inject raw HTML. Link and image URLs are restricted to `http:`,
`https:`, or `mailto:` schemes by the renderer.

The Tauri Content Security Policy further limits webview resources. In
particular, the configured `img-src` permits local assets and `data:` images
but does not permit remote `http:` or `https:` images. Remote Markdown image
references therefore do not display. Network access for Drive is performed by
the Rust HTTP client, not by arbitrary webview scripts.

### Views and preferences

- **Timeline:** loads the newest 30 day groups initially and reveals earlier
  days incrementally; it supports jumping to a selected calendar day.
- **Calendar:** shows all years from the earliest recorded entry through the
  current year, with entry counts by day.
- **Statistics:** derives word totals, average words per entry, active days,
  current/longest streak, a 20-week activity grid, six months of word counts,
  writing hour distribution, and weekday distribution.
- **Settings:** exposes theme, accent and font preferences; biometric toggle;
  lock-now and password-change actions; and Drive connection/sync status.

Appearance preferences are stored separately in browser `localStorage` under
`daylight.appearance`. They are not journal payload data and are not synced
through Google Drive. Supported themes are dark, light, and system; accents
include blue, purple, pink, red, orange, and green; writing fonts are system,
serif, and monospace.

The app suppresses text selection for interface chrome to avoid accidental
highlighting, while leaving form controls, editable areas, and journal-entry
content selectable.

## 5. Vault storage and cryptography

### On-disk representation

The vault is stored as `vault.dat` under Tauri's platform-specific application
data directory. It is a JSON object with:

- `header`: version, KDF salt and parameters, and an encrypted/wrapped data key.
- `data`: base64-encoded encrypted JSON payload.

The decrypted payload contains entries, Drive credentials, the last-backup
time, deletion tombstones, and the last header update time. The only intended
plaintext metadata in the local vault file is the cryptographic header needed
to derive and unwrap the data key; entry content and Drive credentials are
inside the encrypted payload.

### Key hierarchy and algorithms

Vault creation generates:

1. A random 256-bit data-encryption key (DEK).
2. A random 128-bit Argon2 salt.
3. A key-encryption key (KEK) derived from the password using Argon2id,
   version 1.3, with 65,536 KiB (64 MiB) memory, three passes, one lane, and a
   256-bit output.
4. The DEK wrapped by the KEK with XChaCha20-Poly1305.
5. The serialized payload encrypted with the DEK using XChaCha20-Poly1305.

Every XChaCha20-Poly1305 operation uses a fresh 192-bit (24-byte) random
nonce, prepended to its ciphertext. Authentication additional data separates
key-wrapping ciphertext (`daylight-wrap-v1`) from vault payload ciphertext
(`daylight-vault-v1`). Ciphertext and salts are base64 encoded for JSON storage.

The header records the Argon2 parameters so an unlock can derive the KEK. Before
running Argon2, the backend validates the format version and rejects unsafe
parameters: memory must be at least 8 KiB and at most 1,048,576 KiB, passes
must be 1–16, and lanes must be 1–16. Passwords must contain at least eight
characters.

Password changes verify the old password and confirm it unwraps the same DEK
as the active session. They then generate a new salt and wrap that same DEK
under the new password. Entry payload ciphertext does not need to be re-encrypted
solely for a password change.

Sensitive key buffers use `zeroize`/`Zeroizing` where implemented. Password
derivation runs in a blocking worker rather than occupying an async executor
thread.

### Persistence, migration, and recovery

Vault mutations serialize the payload, encrypt it, write a temporary file in
the vault directory, call `sync_all`, and replace the vault file. This avoids
leaving a partially written vault if an operation is interrupted during file
replacement. Entry create/update/delete handlers restore their in-memory
payload state if persistence fails.

On first setup, the backend can import the legacy plaintext `diary.json` file.
It checks for duplicate IDs, creates missing IDs, and fills a missing
`updated_at` from `created_at`. The source file is renamed to
`diary.json.plain-backup` after the encrypted vault has been prepared. If a
backup already exists alongside a source file, import stops and asks the user
to move the backup rather than overwrite it. The plaintext backup should be
removed manually after verifying the encrypted import.

There is no password reset or server-side recovery. Losing the password without
a working device-local biometric key can make the vault unrecoverable. The
Drive backup uses the same password-wrapped DEK and is not a password-reset
service.

## 6. Biometric quick unlock

Biometrics are an optional, device-local convenience path. The app always
retains password unlock as a fallback. The frontend invokes Rust; native
verification is performed in the backend before the saved DEK is loaded. After
an asynchronous prompt, the command checks that the vault was not unlocked by
another operation in the meantime.

Platform implementation:

- **Windows:** Rust calls Windows Hello through the Windows Runtime
  `UserConsentVerifier`, initializes/uninitializes a WinRT apartment, checks
  availability, and requires a `Verified` result.
- **iOS/Android:** Rust calls the Tauri biometric plugin. iOS includes a
  `NSFaceIDUsageDescription` in `Info.ios.plist`.
- **Windows/iOS key storage:** the DEK is stored in the OS keyring.
- **Android key storage:** the DEK is written to an app-private `quick.key`
  file using a temporary file, flush, and replacement.
- **macOS/Linux desktop:** biometric unlock is not implemented.

The Android file is protected by the application sandbox, not by an OS keyring
or hardware-backed encryption in this implementation. Biometric unlock is not
included in Drive backups and does not replace the vault password for restoring
on a new device.

If the stored biometric key no longer decrypts the current vault payload, the
backend removes the stale quick-unlock key and asks the user to use the
password.

## 7. Google Drive authorization, backup, and restore

### Authorization and credentials

The user supplies a Google OAuth Desktop client ID and secret. The native
backend:

1. Binds a loopback listener to `127.0.0.1` on an ephemeral port.
2. Generates a random PKCE verifier and OAuth state, then computes an S256
   challenge.
3. Opens the system browser for Google authorization, requesting only
   `https://www.googleapis.com/auth/drive.appdata`.
4. Validates the returned state and exchanges the code at Google's token
   endpoint.
5. Waits at most 180 seconds for the callback.

HTTP operations use a 60-second client timeout. The refresh token, client ID,
and client secret are stored in the encrypted vault payload. Access tokens are
obtained when constructing a Drive client and are not written into the vault.

### Remote data format

Drive files are stored in the app-private `appDataFolder`. The implementation
uses:

- `vault.chk`: an encrypted marker used to detect a different Daylight vault.
- `header.json`: the vault header, including KDF parameters and the
  password-wrapped DEK. This is not the decrypted data key.
- `e-<entry-id>`: one individually encrypted entry per file.
- `t-<entry-id>`: one individually encrypted deletion tombstone per file.

Entry ciphertext uses associated data `daylight-entry-v1:<id>`; tombstones use
`daylight-tomb-v1:<id>`; the marker uses `daylight-check-v1`. Google Drive
`appProperties` carry an update/deletion time (`u`) and a content stamp (`h`)
used for synchronization and consistency checks. These metadata can reveal
entry IDs and change times even though entry titles and bodies are encrypted.

The sync process paginates the app-data file listing, removes duplicate remote
versions, verifies the vault marker, validates decrypted records against their
IDs/timestamps/stamps, and reconciles local and remote items. Deletes are
represented as tombstones so an old copy cannot silently reappear.

Conflict policy:

- A later timestamp wins.
- On a live-entry versus tombstone conflict, a tombstone wins when its time is
  equal to or later than the entry update.
- A strictly newer live edit can supersede an older tombstone.
- When two live entries have different stamps but equal update times, the
  content stamps provide a deterministic lexical tie-break.
- Matching live stamps and matching tombstones are unchanged.

After sync completes, Rust merges the synchronized snapshot back with any local
edits made during network activity and persists the merged encrypted payload.
The UI schedules a sync about ten seconds after edits, avoids retries within a
20-second minimum gap, syncs on startup/focus/polling when stale, and attempts
to sync before locking.

### Restore

Restore is available only when no local vault exists. The app authorizes the
user, fetches `header.json`, unwraps the data key with the supplied password,
then decrypts and validates the latest entry/tombstone files. Tombstones are
applied before the restored vault is written. The result is atomically created
as the local vault and its Drive configuration is saved in the encrypted
payload.

**Operational caveat:** the implementation documentation notes that Google
Drive synchronization and fresh-install restore have not yet been validated
against a real Google account. Test with nonessential data before relying on
Drive as the only backup.

## 8. Tauri configuration and security boundaries

`src-tauri/tauri.conf.json` configures a 1080×720 resizable window with a
720×480 minimum, a dark background, Vite development server at
`http://localhost:1420`, and `dist` as the packaged frontend. The app identifier
is `com.daylight.diary`.

The webview Content Security Policy is:

```text
default-src 'self' ipc: http://ipc.localhost;
style-src 'self' 'unsafe-inline';
img-src 'self' asset: http://asset.localhost data:;
connect-src ipc: http://ipc.localhost
```

The default Tauri capability grants `core:default` to the main window. Native
Drive networking is performed by Rust's HTTP client. Treat the webview as an
untrusted presentation boundary: sensitive vault operations and biometric
verification belong in registered Rust commands, not frontend-only checks.

The UI defaults to preventing selection of interface chrome, with text
selection retained for inputs, textareas, editable fields, and journal entries.
This is a usability choice, not a confidentiality control.

## 9. Build, run, and test

### Requirements

- Bun (package installation and frontend scripts).
- Rust stable toolchain and Cargo.
- Platform prerequisites documented by Tauri 2.
- For Android: Android SDK, NDK, and JDK.
- For iOS: macOS and Xcode.

### Desktop development and production

Run these commands from the project root:

```sh
bun install
bun run tauri dev
```

Useful validation/build commands:

```sh
bun run check
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
bun run tauri build
```

`bun run check` runs `svelte-check`; `bun run build` emits frontend assets to
`dist`; `bun run tauri build` first builds the frontend, then compiles and
packages the Rust/Tauri app.

On Windows, the release build produces `daylight.exe` and, with the configured
all-target bundle settings, an MSI and an NSIS installer under:

```text
src-tauri/target/release/
src-tauri/target/release/bundle/msi/
src-tauri/target/release/bundle/nsis/
```

### Icons and mobile scaffolding

The repository includes `32x32.png`, `128x128.png`, and `icon.ico`. Generate
additional platform icon formats before macOS/mobile packaging:

```sh
bun run tauri icon icon.png
```

Initialize mobile projects with:

```sh
bun run tauri android init
bun run tauri ios init
```

The iOS command must be run on macOS.

### Test coverage notes

Rust unit tests cover cryptographic round-tripping and rejection of a wrong
password, backwards-compatible payload defaults, sync reconciliation/conflict
behavior, restore filtering, and legacy import handling. Windows-specific
tests cover Windows Hello availability and keyring operations. These unit tests
do not substitute for a real Google OAuth/Drive end-to-end test or validation
on each mobile device and OS version.

## 10. Known limitations and operational guidance

- A forgotten password cannot be reset by Daylight. Keep it in a trusted
  password manager.
- Keep a verified backup; Drive is optional and has not been confirmed against
  a real account in this project's documented test status.
- Drive file names and app properties expose record IDs and timestamps, though
  entry content is encrypted.
- Android biometric quick unlock stores the DEK in an app-private file; this is
  weaker than hardware-backed key storage.
- macOS and Linux desktop biometric unlock is unavailable.
- Remote Markdown images are blocked by the webview CSP. Links may target
  external sites.
- Appearance preferences are device-local and not included in the encrypted
  journal backup.
- The renderer implements a practical Markdown subset and should not be
  treated as a full Markdown specification.
- Verify the legacy plaintext backup after import and remove it when no longer
  needed, since it remains unencrypted.
