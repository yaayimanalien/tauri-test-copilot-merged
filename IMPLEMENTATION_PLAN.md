# Liquid Glass Effect Implementation Plan

## Goal

Implement the visual effect discussed earlier as a layered, cross-platform glass treatment for the app UI:

- base frosted glass using CSS blur + transparency
- optional refraction/bending effect using SVG displacement filter on Chromium
- platform-specific enhancements for Windows and macOS where available
- graceful fallbacks for non-supporting environments

This plan covers only the visual effect itself. It does not include journal logic, vault security, Drive sync, or other product features.

## Scope

### In scope

- CSS glass panel styling
- backdrop blur and translucent surfaces
- subtle tint, highlight, and border treatment
- optional SVG displacement for lens-like bending
- hover/focus/pressed motion states
- Windows Mica/Acrylic integration where appropriate
- macOS native glass treatment when targeting macOS 26+
- performance tuning and fallback behavior

### Out of scope

- encryption, journaling, authentication, or data architecture
- app feature work unrelated to the glass treatment
- full-page shader rendering for all app content
- replacing the app layout or redesigning unrelated UI

## Design objective

The effect should feel like:

- glassy and lightweight
- readable without harming text contrast
- smooth under motion and hover
- visually premium but not overly expensive
- robust across Windows and webview environments

## Primary approach

### 1. Baseline glass layer

Implement the default look first using standard CSS:

- translucent fills
- `backdrop-filter: blur(...)`
- soft borders and highlights
- tinted overlays and inner shadow
- subtle noise/gradient layer for depth
- safe contrast management for text and controls

This is the fallback path for all platforms and should be the default baseline.

### 2. Real lens/refraction layer

Add the SVG displacement filter as an enhancement when running in Chromium/WebView2-based environments:

- apply displacement only to targeted glass surfaces
- keep effect subtle and not overused
- preserve readability and interaction usability
- experiment on Windows first, where the target environment is currently strongest

This step adds the “bending” or “liquid lens” feel without requiring full shader work.

### 3. Native platform refinements

Support native material layers where the platform supports them:

- Windows 11: Mica/Acrylic backdrop treatment
- macOS 26+: native glass material where available

These are additive improvements, not required for the base effect.

### 4. Advanced shader only if needed

Use a custom GLSL/WebGL approach only for isolated cases where a stronger effect is required, such as:

- animated hero background
- custom glass panel with motion
- a premium effect on a limited, performance-friendly surface

Do not use shader rendering as the default for full app UI unless performance is validated.

## Workstreams

### 1. Create a reusable glass component foundation

#### Tasks

- define base glass tokens for:
  - background opacity
  - blur radius
  - tint color
  - border/highlight alpha
  - shadow depth
- create a reusable surface component for panels/cards/modals
- test across light/dark variants and contrast-sensitive content

#### Deliverables

- standardized glass styling tokens
- componentized backdrop glass surfaces
- readable default theme settings

#### Acceptance criteria

- panels remain legible on light and dark backgrounds
- blur is visible but not excessive
- component can be reused across multiple UI surfaces

### 2. Add lens-like refraction using SVG displacement

#### Tasks

- identify which surfaces should receive distortion
- create a small SVG filter with displacementMap
- tune distortion strength, blur, and color mixing
- ensure the effect stays subtle and does not degrade text readability

#### Deliverables

- SVG filter asset or inline filter definition
- effect applied to targeted glass surfaces
- Chromium-specific tuning pass

#### Acceptance criteria

- effect feels like subtle refraction rather than visual noise
- text and controls remain readable
- no obvious flicker or heavy CPU cost

### 3. Build interaction states

#### Tasks

- hover/focus/pressed states for glass panels
- highlight movement or brightness pulse
- border glow and depth transitions
- preserve readability and prevent UI clutter

#### Deliverables

- polished interaction states
- smooth motion and state transitions
- consistent hover treatment across controls and surfaces

#### Acceptance criteria

- hover states are obvious but subtle
- transitions feel smooth without visual lag
- no jarring contrast changes

### 4. Platform tuning and fallbacks

#### Tasks

- apply Windows Mica/Acrylic backdrop if supported
- test graceful fallback when native glass is unavailable
- keep CSS glass as the fallback path for all environments
- document which platforms use which visual stack

#### Deliverables

- platform-aware glass strategy
- documented fallback hierarchy
- support matrix for effect quality by environment

#### Acceptance criteria

- each supported environment has a valid effect path
- unsupported platforms still look polished and functional
- no hard dependency on a single platform capability

### 5. Performance and compatibility pass

#### Tasks

- measure blur cost and panel count
- limit displacement effect to small regions where it matters most
- avoid excessive backdrop layers or repeated filters
- validate against lower-power Windows hardware

#### Deliverables

- optimized performance profile
- effect guidelines for supported surfaces
- compatibility notes for Chromium/WebView2 behavior

#### Acceptance criteria

- UI remains smooth during normal interaction
- no obvious stutter on target machines
- effect remains usable even under modest hardware constraints

## Recommended implementation order

1. CSS glass baseline
2. Reusable glass tokens and component surfaces
3. Targeted SVG displacement enhancement
4. Interaction polish
5. Window/platform integration
6. Performance tuning and final QA

## Milestones

### Milestone 1: Glass baseline

- frosted glass styling works across app surfaces
- readability and contrast are validated
- componentized styling is in place

### Milestone 2: Refraction enhancement

- SVG displacement filter is tested and tuned
- effect is applied only where it helps the visual identity
- performance remains acceptable

### Milestone 3: Platform polish

- Windows native treatment is layered in where available
- macOS native material is considered for future use
- fallbacks remain clean and intentional

### Milestone 4: Final QA

- all major surfaces have consistent glass treatment
- interaction states, hover states, and contrast are confirmed
- final effect quality is accepted

## Acceptance criteria for the final effect

- The app has a clear frosted/glass identity without becoming unreadable.
- The effect feels premium but remains practical and performant.
- There is a clear progression from CSS-only to optional SVG distortion to native platform extras.
- The app still looks polished even when advanced effects are unavailable.

## Risk areas

- Overusing blur or distortion reduces readability
- Chromium-specific filters behave differently across versions
- Too much motion can feel noisy or distracting
- Full-page shader approaches can increase lag and complexity

## Mitigations

- keep distortion subtle and localized
- use CSS glass as the default path
- reserve advanced effects for isolated surfaces
- test on the actual target Windows environment early

## Final recommendation

Treat this as a staged visual enhancement, not a full redesign:

- start with CSS glass
- add SVG displacement where it helps
- layer native platform treatment only when appropriate
- avoid shader-based full-page effects unless there is a clear need and a performance budget

This keeps the visual goal achievable, maintainable, and compatible across platforms while still delivering the intended “liquid glass” feel.
