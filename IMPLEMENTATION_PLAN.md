# Daylight Implementation Plan

## Project goal

Build and harden the Daylight desktop/mobile journal app into a reliable, secure, production-ready release candidate. The app combines a Svelte 5 frontend with a Rust/Tauri backend for encrypted vault management, biometric quick unlock, and Google Drive synchronization.

## Current repository baseline

- Frontend: Svelte 5 + Vite + TypeScript + Tailwind v4 + shadcn-svelte-style UI
- Desktop/mobile shell: Tauri 2
- Backend: Rust 2021, structured around command handlers in `src-tauri/src/`
- Core security model: Argon2id + XChaCha20-Poly1305 encrypted vault with password-based unlock
- Secondary features: biometric unlock, Drive sync/restore, timeline/calendar/stats/settings views
- Current status: architecture and core features are mostly present, but they need validation, cleanup, and production hardening before release

## Success criteria

The app is considered ready when:

- vault creation, unlock, lock, and password change are stable across desktop platforms
- all critical journaling flows work without data loss or corruption
- Drive sync and restore are validated against real accounts and recovery flows
- biometric unlock works reliably on supported platforms
- CI builds succeed for all target platforms
- user-facing edge cases are handled gracefully and tested
- release candidates can be built and distributed without manual workarounds

## Scope

### In scope

- security and data integrity
- journaling workflow and persistence
- vault lifecycle and background lock
- UI/UX polish and reliability
- Drive backup/restore flow
- desktop/mobile packaging
- CI and build validation

### Out of scope

- full general-purpose Markdown spec compliance beyond current app needs
- broad feature expansions beyond journaling and sync
- changing the overall product direction without product sign-off

## Workstreams

### 1. Stabilize the core app lifecycle

#### Objectives

- Ensure the app behaves correctly during startup, unlock, lock, background idle, and quit/reopen cycles.
- Remove ambiguity around which state is authoritative.

#### Tasks

- Confirm `App.svelte` startup flow and status gating for setup/unlock/journal screen.
- Validate all entry lifecycle flows: create, update, autosave, delete, undo, and empty-entry cleanup.
- Verify lock flow flushes pending writes and prevents locked commands from proceeding.
- Add explicit error handling for state desynchronization and failed persistence.
- Review invariants around the Rust `Mutex<AppState>` and the frontend `Journal` store.

#### Deliverables

- stable unlock/setup lifecycle
- deterministic save semantics
- clear user-visible errors on persistence failures

#### Exit criteria

- app opens, locks, and unlocks reliably
- no corrupted entry data after rapid edits or forced lock
- test cases cover lifecycle edge cases

### 2. Harden vault security and persistence

#### Objectives

- Protect the encrypted vault against write corruption and invalid state.
- Make password and data-key handling resilient and auditable.

#### Tasks

- Validate Argon2id parameters and password validation against documented constraints.
- Review atomic write pattern for `vault.dat` and temp-file replacement.
- Ensure key-zeroization is applied consistently to sensitive buffers.
- Test legacy import behavior and plaintext backup handling.
- Confirm migration rules and failure recovery for older vault formats.

#### Deliverables

- secure vault format validation
- atomic persistence model
- verified import/migration path

#### Exit criteria

- vault writes are atomic and recoverable
- wrong-password and malformed-vault tests pass
- legacy migration path is documented and tested

### 3. Improve the editor and journaling UX

#### Objectives

- Make the writing experience reliable and pleasant under real editing conditions.
- Reduce frustration from autosave, undo, and rich-text-like editing behavior.

#### Tasks

- Verify live-editor behavior for long text, code blocks, and list/quote continuation.
- Test cursor movement and block splitting/joining under common editing patterns.
- Review autosave timing and debouncing so updates are not dropped or duplicated.
- Ensure deleted entries and empty drafts behave predictably.
- Validate title/body update ordering for same-entry rapid edits.

#### Deliverables

- stable editor interactions
- consistent autosave and deletion UX
- predictable draft lifecycle

#### Exit criteria

- no duplicate/ghost entry writes
- clear undo/delete behavior
- editing remains smooth during long sessions

### 4. Validate Drive sync and restore

#### Objectives

- Confirm the sync protocol is safe, deterministic, and recoverable.
- Make Drive support production-usable with realistic testing.

#### Tasks

- Test OAuth flow, PKCE, state validation, and token refresh handling.
- Validate sync reconciliation against local state, tombstones, timestamps, and stamps.
- Run end-to-end tests with a real Google account and non-critical data.
- Verify restore-only-on-empty-vault flow and recovery from partial failure.
- Review app-properties metadata and check if any data leakage is acceptable.

#### Deliverables

- drive sync test matrix
- verified restore procedure
- documented operational caveats

#### Exit criteria

- sync works on representative real-world data
- restore from Drive succeeds under supported conditions
- no silent data loss during conflict resolution

### 5. Prune and verify biometric unlock

#### Objectives

- Ensure biometric quick unlock is reliable and safe on supported devices.
- Keep password fallback as the true authority.

#### Tasks

- Validate platform-specific quick-unlock flow for Windows, iOS, Android.
- Review key storage and stale-key cleanup behavior.
- Ensure biometric unlock cannot bypass a changed or invalid vault state.
- Add explicit messaging for unsupported or unavailable biometric surfaces.

#### Deliverables

- functioning biometric quick unlock on supported devices
- graceful fallback to password unlock

#### Exit criteria

- no stale biometric key locks an app into an unusable state
- supported platforms unlock correctly with either password or biometrics

### 6. Strengthen default UI and visual quality

#### Objectives

- Make the app feel polished, consistent, and trustworthy.
- Keep platform theming and glass-like styling aligned with the app’s identity.

#### Tasks

- Review `src/app.css` theme tokens and component consistency across screens.
- Normalize spacing, cards, modal states, and focus styles.
- Check visual accessibility across dark/light/system themes.
- Audit selection behavior, text readability, and hover/focus states.
- Optimize for desktop and mobile density without sacrificing clarity.

#### Deliverables

- consistent theme system
- polished interface states
- improved accessibility and readability

#### Exit criteria

- no broken or inconsistent styling in core flows
- key screens are usable in all supported themes

### 7. Create release-quality build and packaging pipeline

#### Objectives

- Make CI and packaging deterministic and repeatable.
- Reduce manual setup burden for building on all supported targets.

#### Tasks

- Run and fix the existing CI workflow for Windows/Linux/macOS/Android/iOS targets.
- Add or tighten checks for `bun run check`, `bun run build`, and Rust tests.
- Verify required icon generation and mobile initialization steps.
- Document platform prerequisites and signing expectations.
- Create a release checklist for artifact validation before distribution.

#### Deliverables

- validated build pipeline
- release checklist
- CI stability for all target matrix entries

#### Exit criteria

- apps build cleanly from a fresh checkout
- packaging artifacts are generated without manual editing
- release process is repeatable

## Release milestones

### Milestone 1: Stability pass

- core journal lifecycle is stable
- vault persistence is atomic and tested
- startup/lock/unlock flows validated

### Milestone 2: Security + sync pass

- password and vault validation hardened
- Drive sync and restore tested with real data
- biometric quick unlock validated on supported platforms

### Milestone 3: Product polish + release readiness

- UI polish and accessibility review complete
- packaging pipeline validated
- release checklist complete
- beta or production candidate can be built from main reliably

## Risk register

### High-risk items

- Drive sync conflict handling with real-world data
- data corruption during vault write or app interruption
- stale biometric keys causing lockout or inconsistent unlock state
- platform-specific build issues on macOS/iOS/Android

### Mitigations

- enforce atomic file replacement and backup validation
- run end-to-end sync tests on noncritical data before release
- keep password authentication as the guaranteed fallback
- maintain a platform-specific build matrix in CI and verify all artifacts before release

## Suggested execution order

1. Fix lifecycle and persistence correctness
2. Validate vault security and migration behavior
3. Harden journal editor and autosave
4. Test real Drive sync/restore flows
5. Validate biometrics and fallback behavior
6. Polish UI and accessibility
7. Release packaging and CI stabilization

## Recommended immediate next steps

- Review the current `src-tauri/src` command contracts and refine the security boundaries.
- Add a focused test matrix for vault creation, wrong-password rejection, and persistence failure handling.
- Run real Drive sync against a nonessential Google account before turning it into a release promise.
- Confirm the build matrix in CI matches the supported OS/mobile targets and fixes any blockers.

## Documentation to maintain alongside the plan

- security overview
- vault format and recovery guide
- Drive sync operational notes
- platform build prerequisites
- release checklist

This implementation plan should be treated as the working roadmap for shipping Daylight as a trustworthy encrypted journal app rather than a feature prototype.

## Proposed file additions

- `IMPLEMENTATION_PLAN.md` — overall roadmap
- `docs/release-checklist.md` — release validation steps
- `docs/security-model.md` — vault and key handling overview
- `docs/drive-sync-validation.md` — functional validation checklist for Google Drive

## Ownership and cadence

- Product/engineering review: weekly
- Risk review: after each milestone
- Decision gate before release: all critical flows must be validated on at least one clean build and one real-data test cycle

This document should evolve with the codebase; when a task is completed, mark it as shipped and record the validation evidence in the project issues or release notes.

## Final recommendation

Prioritize correctness and data safety before feature polish. The app’s core promise is secure personal journaling, and that requires trustworthy vault lifecycle behavior and a validated backup/sync story before the product is considered ready for wider use.

