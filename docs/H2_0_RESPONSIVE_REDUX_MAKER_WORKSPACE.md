# H2.0 — Responsive HomeOps Shell + Redux Maker Workspace + Theme Settings

## Goal

Make HomeOps feel like one unified, polished desktop app that works in
non-maximized windows: add a first-class Redux Maker workspace page, improve
responsive layout, match the top app bar to the HomeOps style, and add a
Settings → Themes section. No server-side RPF editing is added.

## Why the Redux Maker page was added

The HomeOps server only handled storage, jobs, and Redux corpus scanning. The
user wanted the Redux Maker workspace inside HomeOps too. H2.0 adds a Redux
Maker **workspace page** that surfaces server corpus context and points at the
local desktop Redux Maker — without faking any server-side generation or apply.

## Local / server split

| Concern | Where |
| --- | --- |
| Storage, jobs, Redux corpus scan + dataset build | HomeOps server (`server-agent`) |
| AI planning, CodeWalker, copied-RPF apply, rollback | Local desktop Redux Maker app (`apps/redux-maker-ui`) |
| Redux Maker workspace UI + corpus context | HomeOps frontend (this phase) |

The HomeOps server still does **not** edit RPF files. A safe local bridge is
deferred to H2.1.

## Responsive shell behavior

Breakpoints:

- **≥ 1100px** — full layout: 250px sidebar + content + global top strip.
- **≤ 1100px** — sidebar narrows to 200px; auto-fit card grids collapse columns.
- **≤ 980px** — top strip drops the endpoint/brand-sub/guard chips.
- **≤ 880px** — sidebar collapses to a compact **icon rail** (labels/sections/
  storage dock hidden; active item keeps its accent bar).
- **≤ 760px** — top-strip quick-nav links hide (the rail covers navigation);
  brand + status stay reachable.

All scroll lives in the main content / table wrappers; the file table uses
`table-layout: fixed` so it compresses instead of breaking the page. Card pages
(storage, redux-corpus, redux-maker, settings, resources) use
`auto-fit, minmax(...)` grids that reflow to one column.

## Title / top bar behavior

OS window decorations are **kept** (no `decorations:false`) so window
drag/resize/minimize/maximize/close are never broken. The in-app
`global-topbar` strip in `AppShell.svelte` provides the HomeOps-styled bar:
brand `HO` mark + "HomeOps Panel", server endpoint, quick-nav, and a Tailscale
"private · active" status chip, on `--bg-input` with a thin bottom border. This
matches the command-center theme tokens.

Limitation: a fully custom native titlebar (themed OS controls) would require
`decorations:false` + custom drag/controls and is intentionally **not** done in
H2.0 to avoid window-control regressions.

## Settings → Themes

New "Appearance · Themes" panel at the top of Settings:

- Theme registry at `apps/web/src/lib/theme/themes.ts`.
- **HomeOps Command Dark** (`command-dark-current`) is the only selectable theme
  (active badge, preview swatches, description).
- Placeholder themes (OLED Dark, Amber Terminal) render disabled with a "soon"
  badge and "More themes will be added later" — not selectable, not faked.
- Selection persists in `localStorage` (`homeops.theme`) and is applied on load
  via `data-theme` on the document root. The default theme's tokens live on
  `:root`, so there is no load flash.

## Routes audited

`/ · /files · /archives · /jobs · /logs · /resources · /storage · /settings ·
/projects · /redux-corpus · /redux-maker · /minecraft (+ servers/files/console)`
— each renders without catastrophic horizontal overflow at medium/small widths;
primary actions and navigation stay reachable.

## Smoke results

- `npm run check` → 0 errors / 0 warnings (364 files).
- `npm run build` → success.
- `cargo test -p server-agent` / `cargo build -p server-agent` → pass (backend
  unchanged this phase).
- Manual resize smoke recommended at 1366×768 / 1100×700 / 900×650 (rail engages
  ≤ 880px). No backend deploy performed (frontend-only phase).

## Limitations

- No custom native titlebar (OS decorations kept by design).
- No server-side Redux generation/apply; Redux Maker page is context + handoff
  only until H2.1.
- HomeOps has no JS unit-test runner; frontend verified via check + build.

## Next phase

H2.1 — Local Redux Maker Bridge for HomeOps Desktop.
