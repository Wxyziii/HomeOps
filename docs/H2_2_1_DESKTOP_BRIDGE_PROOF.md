# H2.2.1 — Desktop Bridge Proof + Live Apply/Rollback Smoke

## Summary

Goal: prove the Windows Tauri GUI bridge end-to-end (detect bridge → plan-only →
corpus context → apply-ready → live copied-RPF apply → rollback to clean SHA).

**Outcome: PARTIALLY PROVEN. The interactive desktop GUI smokes and the live
apply/rollback are STILL NOT executed.** Two real report-parsing bugs were found
(via a headless CLI run of the exact pipeline) and fixed, and the CLI-level
plan-only pipeline + report contract were verified. The live GUI clicks and the
copied-RPF apply/rollback remain blocked in this environment for two reasons:

1. **No desktop GUI access** — this work ran in a headless/agent environment that
   cannot launch and click the Tauri window. GUI smokes (Parts C–H) require a
   human at the Windows desktop.
2. **CodeWalker is offline** — `http://127.0.0.1:5560` refused the connection (TCP
   5560 closed) during this phase, so a real YTD build + `/api/replace-rpf-entry`
   apply could not run regardless of GUI.

No live apply was attempted; the copied test RPF was never modified and stayed at
the clean SHA throughout.

## Environment checks

| Check | Result |
|-------|--------|
| Copied RPF SHA (before / after / final) | `32d6aa…5396dc` — clean, unchanged |
| CodeWalker `127.0.0.1:5560` | **offline** (connection refused, TCP closed) |
| Scanner binary present | yes (`…\rpf_backend_rs\target\release\rpf_backend_rs.exe`) |
| HomeOps `npm run check` / `build` | 0 errors/0 warnings / OK |
| `cargo test -p server-agent` / build | 127 passed / OK |
| Tauri `cargo test --lib` | 29 passed |
| ReduxScannerEngine | not touched (binary present, RPF clean) |

## Smoke results

### Smoke 1 — browser/server mode (Part C)
**Not executed in a live browser** (no GUI). Verified by code: `isTauri()` is false
in the browser build, so every bridge call throws "Local bridge unavailable in
browser mode", and Generate/Apply are disabled with reasons; corpus context still
loads from the read-only HomeOps API. The SSR build compiles and renders without a
bridge. **Manual browser pass still pending.**

### Smoke 2 — desktop bridge status (Part D)
**Not executed** (no GUI). The `redux_maker_bridge_status` command is unit-tested
(scanner-present mirrors filesystem, copied-RPF SHA cleanliness, loopback/reachable
logic). A live desktop status read is **pending**. Note: with CodeWalker offline
the status would honestly show CodeWalker **unreachable**.

### Smoke 3 — plan-only run (Part E)
**Verified at CLI level** (not via GUI). Ran the scanner with the EXACT argv the
bridge builds:

```
rpf_backend_rs.exe ai-redux-maker --prompt "Create a safe GTA V Redux visual
  module plan for colder nights and red vertical top-to-bottom tracers. Do not
  apply anything." --out-dir <run> --out <run>/mvp_report.json
  --provider rule_based --mode full_plan_no_execute --workspace <ws>
```

Report produced (`.tmp/homeops-runs/run-h221-cli-planonly/mvp_report.json`):

| field | value |
|-------|-------|
| status | planned |
| provider / mode | rule_based / full_plan_no_execute |
| moduleSafe | true |
| readyToApply | false (correct — plan-only) |
| applied | false |
| generatedAssetCount | 1 |
| replacementPlanCount | 0 |
| safetyFacts.codewalker*/replaceRpfEntry/originalGtaArchiveTouched/copiedRpfModified | all false |

Copied RPF SHA unchanged. No CodeWalker call, no forbidden endpoint, no cloud/public
network call. **Live GUI run still pending**, but the underlying pipeline + report
contract are proven.

### Smoke 4 — corpus context attached (Part F)
**Not executed** (no GUI; also depends on a populated corpus dataset on the server).
The retrieval endpoint and context-pack builder are unit/logic verified. **Pending.**

### Smoke 5 — apply-ready Red Tracer (Part G)
**Blocked** — apply-ready requires a real YTD build through loopback CodeWalker,
which was offline. Could not reach `readyToApply=true`. **Pending** (start
CodeWalker, then run from the desktop app).

### Smoke 6 — apply + rollback (Parts G/H)
**Not executed** — depends on Smoke 5 and CodeWalker. No apply was attempted; the
copied RPF was never modified. Rollback remains the display-only CLI command
(`ROLLBACK_REDUX_MODULE_COPIED_RPF`). **Pending.**

## Bugs found & fixed

Inspecting the REAL plan-only report exposed two mismatches between the scanner's
output shape and the bridge's report parsing (fixed in
`apps/web/src-tauri/src/redux_bridge.rs`):

1. **`fallbackUsed` location** — the scanner emits `fallbackUsed` at the report top
   level, but the bridge read it from `safetyFacts`, so telemetry always showed
   `false`. Now read from the top level (legacy `safetyFacts` location still
   honored).
2. **`forbiddenEndpointCallCount` absent** — the report has no such numeric field;
   it exposes `safetyFacts.*Called` booleans. The bridge defaulted the count to 0
   unconditionally, which would hide a real forbidden call. It now derives the
   count from the forbidden write flags (`stockReplaceFileCalled`, `importCalled`,
   `reloadServicesCalled`, `setConfigCalled`); `replaceRpfEntryCalled` is the
   ALLOWED path and is not counted. The apply path's `replaceRpfEntryCallCount`
   also falls back to `safetyFacts.replaceRpfEntryCalled` when no explicit count is
   present.

Two unit tests added (`fallback_used_read_from_top_level`,
`forbidden_count_derived_from_safety_facts`) — tauri lib now 29 tests.

## Limitations

- Interactive desktop GUI smokes (1, 2, 4) and the live apply/rollback (5, 6) are
  **still pending** — they need a human at the Windows desktop with CodeWalker
  running on `127.0.0.1:5560`.
- When a report omits `targetRpf` (plan-only does), the apply command skips the
  explicit "target is copied RPF" check; the SHA gate + exact confirmation +
  apply-plan-under-`.tmp/homeops-runs` gates still apply. Apply-ready runs set
  `targetRpf`, so this only affects non-ready reports (which the readyToApply gate
  already rejects).

## To finish the proof (operator checklist)

1. Start CodeWalker.API; confirm `http://127.0.0.1:5560` responds.
2. `cd apps/web && npm run tauri:dev` (or `tauri build`) to open the desktop app.
3. `/redux-maker` → confirm bridge ready, copied RPF clean.
4. Plan-only run → report loads, applied=false, SHA unchanged.
5. Attach tracer corpus context → visible block, run again.
6. Apply-ready Red Tracer → readyToApply=true, replacementPlans ≥ 1.
7. Review & Apply, type `APPLY_REDUX_MODULE_TO_COPIED_RPF` → SHA changes,
   replaceRpfEntryCallCount ≥ 1, forbiddenEndpointCallCount = 0.
8. Run the displayed rollback command → SHA returns to `32d6aa…5396dc`.
