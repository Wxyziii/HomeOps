# H2.0.1 — Port Full Redux Maker Studio UI into HomeOps

## Why H2.0 was not enough

H2.0 added `/redux-maker` as a simplified dashboard: a header, a row of generic
workflow cards, a corpus-context panel, and copy buttons. It did not look or
feel like the actual Redux Maker Studio. The user wanted the real Studio layout
(blueprint tree, diff/review workspace, prompt composer, telemetry/log/action
dock) embedded inside HomeOps.

## What H2.0.1 changed

`/redux-maker` now renders a real embedded **Redux Maker Studio** workspace that
structurally matches the standalone app (`apps/redux-maker-ui`), inside the
HomeOps shell, using HomeOps theme tokens. The generic workflow-card page is
gone as the main view.

## New Studio layout

`ReduxMakerStudio.svelte` orchestrates:

- **Top status ribbon** (`ReduxMakerLocalBridgeStatus`): "Redux Maker Studio",
  local bridge: not connected, corpus server: ready/unavailable, safety: server
  does not edit RPF.
- **Left** (`ReduxMakerBlueprint`): Redux Blueprint tree — Run / Module /
  Blocked / Assets sections with honest `EMPTY` rows and a "No run loaded"
  footer.
- **Center** (`ReduxMakerReviewPane` + `ReduxMakerPromptDock`): context ribbon,
  dual INPUT/REVIEW panes in a truthful empty state (readyToApply/applied/
  rollbackReady all `false`), and the AI Patch Prompt composer at the bottom.
- **Right** (`ReduxMakerTelemetry` + `ReduxMakerActionDock`): corpus-context card
  (`ReduxMakerCorpusContext`), process telemetry (idle, no run, 0 forbidden
  endpoint calls), system log (real facts only), and the action dock.

Components live under `apps/web/src/lib/components/redux-maker/`.

## Responsive behavior

- **≥ 1200px** — full three-pane Studio: `blueprint | center | dock`.
- **900–1200px** — right dock drops below the center (blueprint spans both
  rows); dock cards lay out in a row.
- **≤ 880px** — everything stacks: blueprint and dock become short scrollable
  sections, the center (with the prompt) stays primary; the page scrolls.
- Review panes collapse from 2-col to stacked ≤ 720px; the prompt input row
  stacks ≤ 720px. No catastrophic horizontal overflow; text truncates.

## Local / server split

- **HomeOps server**: storage, jobs, Redux corpus scan + dataset build (read-
  only). Surfaced here as corpus context (status/dataset/report/quarantine).
- **Local desktop Redux Maker app** (`apps/redux-maker-ui`): AI planning,
  CodeWalker, copied-RPF apply, rollback. Generation/apply happen only there.
- **HomeOps Studio page**: faithful UI + corpus context + handoff. No
  generation or apply.

## Local bridge limitation

The safe local bridge (H2.1) is not implemented. Therefore:

- "Generate Module Plan" is disabled — reason: **Local bridge not connected**.
- "Review & Apply Plan" is disabled — reason: **No local run loaded / local
  bridge required**.
- Blueprint/review/telemetry show honest empty state — no fake run, report,
  module plan, generated assets, or readyToApply.

## Safety model

- No server-side RPF apply; no CodeWalker server controls; no RPF write
  endpoints; no arbitrary shell or executable launch from HomeOps.
- No original GTA file access; no copied-RPF mutation from HomeOps.
- No fake run/report/apply-ready/telemetry. Corpus context is read from the
  existing read-only HomeOps API and shown honestly (or "unavailable" on error).
- API-token / Tailscale security unchanged; HomeOps delete behavior unchanged.

## Settings / Themes compatibility

Untouched: Settings → Appearance · Themes, HomeOps Command Dark active theme,
disabled placeholder themes, `localStorage` persistence, `data-theme` behavior,
the responsive shell, sidebar, global topbar, Redux Corpus / Storage / Files.

## Smoke results

- `npm run check` → 0 errors / 0 warnings (372 files).
- `npm run build` → success.
- `cargo test -p server-agent` / `cargo build -p server-agent` → pass (backend
  unchanged this phase).
- Manual resize smoke recommended at 1366×768 / 1100×700 / 900×650 (dock drops
  below ≤ 1200px, full stack ≤ 880px).

## Next phase

H2.1 — Local Redux Maker Bridge for HomeOps Desktop.
