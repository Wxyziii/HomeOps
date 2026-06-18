# H2.2.2 — Windows GUI Bridge Proof

## Outcome: BLOCKED — still not proven

Goal: run the HomeOps Tauri desktop app on Windows, start CodeWalker.API, and prove
the full Redux Maker bridge (status → plan-only → context → apply-ready → apply →
rollback) from `/redux-maker`.

This attempt could **not** complete the live proof, for the same two reasons as
H2.2.1 — neither resolved at run time:

1. **CodeWalker.API offline** — `http://127.0.0.1:5560` refused the connection (TCP
   5560 closed). Per Part B, apply-ready / apply / rollback (Smokes 4–6) must not be
   attempted when CodeWalker is offline. They were not.
2. **No interactive desktop GUI** — this ran in a headless/agent environment that
   cannot launch and click the Tauri window, so the GUI smokes (1–3) cannot be
   driven here.

No apply was attempted. The copied test RPF stayed at the clean SHA
`32d6aa…5396dc` throughout.

## What was verified this phase

| Check | Result |
|-------|--------|
| Copied RPF SHA (before / final) | `32d6aa…5396dc` — clean, unchanged |
| CodeWalker `127.0.0.1:5560` | **offline** (connection refused, TCP closed) |
| `npm run check` | 0 errors / 0 warnings |
| `cargo test -p server-agent` | 127 passed |
| git tree | clean |
| Tauri `cargo test --lib` / builds | unchanged since `f899271` (29 tests green) — not re-run for a no-op |

No code bug surfaced (no GUI/CodeWalker access to reproduce one), so no code change
was made this phase. The CLI-level plan-only path and the two report-parsing fixes
proven in H2.2.1 remain in place.

## Smoke status

| Smoke | Status |
|-------|--------|
| 1 — desktop bridge status | **pending** (needs GUI) |
| 2 — plan-only generation | proven at CLI in H2.2.1; **GUI pending** |
| 3 — corpus context attached | **pending** (needs GUI + populated dataset) |
| 4 — apply-ready Red Tracer | **blocked** (CodeWalker offline) |
| 5 — apply copied RPF | **blocked** (CodeWalker offline) |
| 6 — rollback | **blocked** (no apply performed) |

## To finish the proof (operator, on the Windows desktop)

1. Start CodeWalker.API; confirm `Invoke-WebRequest http://127.0.0.1:5560` responds.
2. `cd apps/web && npm run tauri dev` to open the HomeOps desktop app.
3. `/redux-maker` → bridge ready, copied RPF clean, CodeWalker reachable.
4. Plan-only run → report loads, applied=false, SHA unchanged.
5. Attach tracer corpus context → visible block, run again.
6. Apply-ready Red Tracer → moduleSafe=true, readyToApply=true, replacementPlans ≥ 1.
7. Review & Apply, type `APPLY_REDUX_MODULE_TO_COPIED_RPF` → SHA changes,
   replaceRpfEntryCallCount ≥ 1, forbiddenEndpointCallCount = 0, rollback manifest
   shown.
8. Run the displayed rollback command → SHA returns to `32d6aa…5396dc`.

Full field-by-field checklist: `docs/H2_2_1_DESKTOP_BRIDGE_PROOF.md`.

## Limitations

The bridge remains **CLI-pipeline-proven but GUI-unproven**, and the live
copied-RPF apply/rollback is **unproven** until CodeWalker is running and the proof
is executed from the desktop GUI. This is an environment/operator gap, not a known
code defect.
