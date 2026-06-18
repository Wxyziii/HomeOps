# H2.1 — Local Redux Maker Bridge for HomeOps Desktop

## Goal

Let the HomeOps **desktop** app safely run the local Windows ReduxScannerEngine
pipeline from the `/redux-maker` Studio page: check the local environment, run
plan-only or apply-ready-proof generation in the background, load the report
into the Studio, and — only behind hard gates — apply a reviewed plan to the
**copied test RPF**.

## Desktop-only bridge architecture

The bridge is implemented entirely in the HomeOps **Tauri** layer
(`apps/web/src-tauri/src/redux_bridge.rs`) plus a thin frontend client
(`apps/web/src/lib/redux-maker/`). It is **local desktop only**:

| Mode | Bridge | Generate | Apply | CodeWalker |
|------|--------|----------|-------|------------|
| Browser / HomeOps server | unavailable | disabled | disabled | never |
| HomeOps desktop (Tauri) | available when env OK | enabled | gated | loopback only |

The Ubuntu **server-agent gets none of this**: no scanner execution, no
CodeWalker, no RPF write endpoints. A server-agent test
(`no_redux_maker_rpf_apply_route_in_server_agent`) fails the build if a Redux
Maker apply route, a CodeWalker write endpoint, or a YTD build endpoint is ever
added to it.

### Browser / server-mode limitations

`isTauri()` is false in the browser. Every bridge call throws
`Local bridge unavailable in browser mode`, the Studio shows
`local bridge: browser mode`, and Generate / Apply are disabled with reasons.
No fake report data is ever shown.

## Fixed local settings

Settings live in the desktop app / `localStorage` only — never in server config
(`apps/web/src/lib/redux-maker/bridgeSettings.ts`). The **paths and SHA are
fixed and owned by the Rust bridge**; the UI cannot point the bridge at an
arbitrary binary or archive in H2.1 (a validated path picker is deferred to
H2.2). User-tunable, loopback-validated fields: `provider`, `allowLocalAi`,
`localAiUrl`, `model`, `codewalkerUrl`.

| Field | Value |
|-------|-------|
| scannerBinaryPath | `…\ReduxScannerEngine_GitHubRepo\rpf_backend_rs\target\release\rpf_backend_rs.exe` |
| workspaceRoot | `C:\Users\Marcel\Downloads\ReduxScannerEngine_GitHubRepo` |
| copiedRpfPath | `C:\Users\Marcel\Downloads\ReduxScannerTest\test-copy\update.rpf` |
| expectedCopiedRpfSha | `32d6aa5395c6b9e06c7375a9627407eb824e197ebcd9cb3c4f318629545396dc` |
| codewalkerUrl | `http://127.0.0.1:5560` |
| provider | `rule_based` (default) |
| allowLocalAi | `false` (default) |
| localAiUrl | `http://127.0.0.1:11434` (loopback only) |

## Tauri commands

| Command | Purpose |
|---------|---------|
| `redux_maker_bridge_status` | Check scanner binary, workspace, copied-RPF existence + SHA + cleanliness, CodeWalker loopback/reachability, optional local-AI reachability. |
| `redux_maker_start_run` | Start a background plan-only (`planOnly`) or apply-ready (`applyReadyProof`) run. Returns `runId` immediately. Args array, no shell, never `--apply`. |
| `redux_maker_get_run_status` | Poll phase + capped stdout/stderr tails + parsed report facts (`moduleSafe`, `readyToApply`, `applyPlanPath`, generated assets, replacement plans, safety facts, forbidden-endpoint count). |
| `redux_maker_cancel_run` | Kill **only the owned child process** for that run id. |
| `redux_maker_apply_reviewed_plan` | Apply a reviewed plan to the copied test RPF behind all gates (below). |
| `redux_maker_read_report_file` | Read-only `.json/.md/.txt/.log` preview confined to `.tmp/homeops-runs`. |

Outputs are written under
`C:\Users\Marcel\Downloads\ReduxScannerEngine_GitHubRepo\.tmp\homeops-runs\run-<id>\`
(gitignored).

## Run flow

1. Open the HomeOps desktop app → `/redux-maker`.
2. Bridge status loads automatically (scanner / copied RPF / CodeWalker).
3. Enter a prompt or pick a preset; optionally tick **apply-ready proof**.
4. Click **Generate Module Plan** → background run starts; the UI polls every
   ~800 ms, streams logs, and stays responsive. **Cancel Run** is available.
5. On finish, the report loads into the blueprint / review / telemetry panes.
   Failed / cancelled / timed-out runs show honest status, no fake report.

## Apply-ready flow

`applyReadyProof` mode adds `--include-ytd-build`, `--allow-local-builder`
(loopback CodeWalker only — the fake-YTD builder is never enabled),
`--target-rpf` (copied RPF) and `--expect-sha` (clean SHA), so the scanner
builds a real YTD and a copied-RPF replacement plan and can set
`readyToApply=true`. Generation **never applies**.

## Apply gates (defence in depth)

`redux_maker_apply_reviewed_plan` refuses unless **all** hold:

- report `readyToApply=true`, `moduleSafe!=false`, not already `applied`, has an
  `applyPlanPath` under `.tmp/homeops-runs`;
- report `targetRpf` is exactly the fixed copied test RPF (original GTA install
  markers rejected);
- confirmation equals exactly `APPLY_REDUX_MODULE_TO_COPIED_RPF`;
- the copied RPF exists and its **current SHA equals the clean SHA**
  `32d6aa…5396dc` (and the caller's `expectedCopiedRpfSha` matches it);
- CodeWalker URL is loopback.

It then runs `apply-redux-module --apply-plan … --confirm … --codewalker-base-url
… --rollback-dir … --out …` via the fixed scanner binary (args array, no shell),
whose only write is the scanner's `POST /api/replace-rpf-entry`. Forbidden
endpoints (`/api/replace-file`, `/api/import`, `/api/reload-services`,
`/api/set-config`) and `ai-redux-maker` / `rollback-redux-module` can never
appear in the apply argv.

### Exact confirmation gate (UI)

The Apply modal shows target RPF, expected SHA, current SHA, CodeWalker URL, and
states original GTA files are not touched. Apply is enabled only after the user
types `APPLY_REDUX_MODULE_TO_COPIED_RPF` and the current SHA is clean.

## Rollback behavior

Rollback is **display-only** in H2.1. After a successful apply the dock shows the
rollback manifest path and a copy-able rollback CLI command using
`ROLLBACK_REDUX_MODULE_COPIED_RPF`. No one-click rollback command is exposed yet.

## Tests

Rust (`homeops-panel` lib, `apps/web/src-tauri`): 27 unit tests including missing
scanner detection, clean/dirty copied-RPF SHA logic, loopback enforcement (local
AI + CodeWalker), args-array/no-shell, never-adds-apply, run-id shape,
cancel-owns-child-only, exact-confirmation, dirty-RPF / original-GTA-target /
non-loopback / not-ready rejections, fixed-scanner-binary, run-dir confinement,
read-only preview guards. Server-agent: `no_redux_maker_rpf_apply_route_in_server_agent`
plus the existing suite (120 tests total).

## Manual smoke results

| Smoke | Status |
|-------|--------|
| 1 — Browser/server mode shows bridge unavailable, Generate/Apply disabled, no fake data | **Verified by build/logic** (SSR build + `isTauri()` gating); live browser pass pending |
| 2 — Desktop bridge status loads (scanner/copied RPF/SHA/CodeWalker) | **Pending** (requires running the Tauri desktop GUI) |
| 3 — Plan-only run loads report, `applied=false`, SHA unchanged | **Pending** (requires desktop GUI) |
| 4 — Apply-ready proof + live apply + SHA change + rollback | **Pending** (requires desktop GUI + running CodeWalker) |

Automated verification performed: `npm run check` (0 errors), `npm run build`
(OK), `cargo test --lib` on `homeops-panel` (27 passed), `cargo test -p
server-agent` (120 passed), `cargo build -p server-agent` (OK). The copied test
RPF SHA was confirmed clean (`32d6aa…5396dc`) before/after this work — no apply
was executed from this environment.

## Limitations

- Live desktop GUI smokes (2–4) and the live copied-RPF apply must be run
  manually on the Windows desktop with CodeWalker started; they were not
  exercised headlessly here.
- Scanner / workspace / copied-RPF paths are hardcoded in the bridge (single
  machine). H2.2 adds a validated path picker.
- Rollback execution stays CLI-only (display command).

## Next phase

**H2.2 — HomeOps Redux Maker Bridge Polish + Corpus Context Retrieval**
(validated path picker, richer report/diff rendering, corpus-context retrieval
into prompts, one-click gated rollback).
