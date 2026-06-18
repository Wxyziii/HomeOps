# H2.2 — Redux Maker Bridge Polish + Corpus Context Retrieval

## Goal

Polish the HomeOps Redux Maker bridge and make `/redux-maker` use the server-side
Redux Corpus dataset as structured **context** for prompt/module planning. This is
**not** model fine-tuning and **not** AI training — it retrieves safe metadata
records from the already-scanned corpus and lets the user attach them to a prompt
before running the local desktop ReduxScannerEngine pipeline.

## H2.1 desktop bridge proof — STILL PENDING

The mandatory H2.1 desktop GUI smokes (bridge status, plan-only run, apply-ready
proof, live copied-RPF apply) require running the Tauri desktop app + CodeWalker
on the Windows machine. They were **not** performed in this headless environment
and are **not** claimed as proven. The copied RPF stayed clean
(`32d6aa…5396dc`); no apply was executed. Run them manually per H2.1 Part B / this
phase Part B before relying on live apply.

## Bridge polish

- Bridge status ribbon shows distinct honest states: browser mode, desktop ready,
  scanner missing, copied RPF dirty/clean, CodeWalker reachable/offline/non-loopback.
- Action dock: **Refresh bridge + corpus**, **Copy scanner path**, **Copy copied
  RPF path**, **Copy Redux Maker path**, **Copy local dev command**, and **Copy
  rollback command** (after apply).
- Disabled Generate/Apply always show the exact blocking reason.
- Run logs (stdout/stderr tails) render in the review pane, capped in the bridge.
- Failed / timed-out / cancelled runs display honestly; no fake report data.
- Telemetry now shows: provider, run phase, module plan, generated assets,
  moduleSafe, readyToApply, applied, localModelCalled, fallbackUsed, cloudAiCalled,
  publicNetworkCall, replace-rpf-entry calls, forbiddenEndpointCallCount, SHA
  before/after, and corpus-context attached (record count + categories).

## Run history

Local `localStorage` only (`lib/redux-maker/runHistory.ts`) — never server-side.
Stores per run: runId, prompt, presetId, provider, mode, status, outDir,
mvpReportPath, readyToApply, applied, corpusContextAttached, contextRecordCount,
contextCategories, copiedRpfShaBefore/After, createdAt, updatedAt. The action dock
lists recent runs; clicking one reloads its report (via `redux_maker_get_run_status`,
falling back to a read-only `redux_maker_read_report_file` of `mvp_report.json`).
**Clear** removes the index only — it never deletes `.tmp/homeops-runs` folders.

## Corpus context retrieval

`/redux-maker` → Corpus Context panel (`ReduxMakerCorpusContext.svelte`):

- shows dataset counts (read-only HomeOps API);
- filter by **category** (from `categoryCoverage`) and **target pattern**;
- **Find** calls the read-only `GET /api/redux-corpus/dataset/records`;
- selectable record cards (category, intent, target patterns, blocked reasons,
  confidence);
- **Attach Context to Prompt** builds a context pack from the selected records.

Records contain ONLY metadata + safe evidence — no raw binary, no asset bytes, no
real package files.

## Context pack format

`lib/redux-maker/contextPack.ts` emits a single, user-visible, clearable block
appended to the prompt (the composer visually separates user prompt / attached
context / safety). A token estimate is shown. Example:

```
[Redux Corpus Context]
Records: 1
Categories: tracers

• tracers — replace bullet tracer visuals (vertical top-to-bottom)
  Common targets:
    - update.rpf/x64/textures/frontend.ytd
  Safe templates: tracer_texture_replacement_plan
  File types: ytd
  Evidence: a/b.ytd
  Confidence: 0.85

Safety:
  - copied-RPF apply only; original GTA paths are never targeted
  - metadata-only context; no raw binary or copyrighted asset bytes
  - tracer orientation is vertical top-to-bottom
[/Redux Corpus Context]
```

Because the scanner has no structured context argument yet, H2.2 **prepends the
context text to the prompt** (temporary). H2.3 will add a structured context input
to ReduxScannerEngine. The run history records `corpusContextAttached` +
`contextRecordCount` + `contextCategories`.

## Read-only dataset endpoint added

`GET /api/redux-corpus/dataset/records?category=&targetPattern=&packageId=&limit=`
(`services/server-agent/src/redux_corpus.rs` + `main.rs`):

- reads ONLY the fixed `corpus_dataset_records.jsonl` under the bulk corpus
  datasets root (path derived from the Smart Pool bulk root, `assert_inside_root`);
- filters are pure data and can never change which file is read;
- limit defaults to 50, capped at 500; per-line (64 KiB), total-bytes (64 MiB),
  and lines-scanned (200k) caps; malformed/oversized lines are skipped safely;
- no raw binary reads, no write/delete, token auth required (same router).

### Tests

`dataset_records_caps_limit`, `dataset_records_filters_category`,
`dataset_records_filters_target_pattern`, `dataset_records_ignores_malformed_lines_safely`,
`dataset_records_rejects_traversal`, `dataset_records_reads_only_bulk_corpus_root`,
`dataset_records_reads_written_jsonl_from_bulk`, plus the existing
`no_redux_maker_rpf_apply_route_in_server_agent` guard. Server-agent: **127 passed**.

## Safety model

No server-side RPF apply, no CodeWalker controls/calls in server-agent, no new RPF
write endpoints. Apply stays local desktop only, copied-RPF only, behind the exact
phrase `APPLY_REDUX_MODULE_TO_COPIED_RPF`, a clean-SHA gate, and loopback
CodeWalker, via the scanner's single `/api/replace-rpf-entry` path. Fixed scanner
path, args-array, no arbitrary shell/exe. Local AI loopback-only; public model URLs
blocked.

## Manual smoke results

| Smoke | Status |
|-------|--------|
| A — corpus context (counts, filter, select, attach, clear) | **Logic + build verified**; live data pass pending (needs a built corpus dataset + GUI) |
| B — bridge run with context, plan-only | **Pending** (desktop GUI) |
| C — apply-ready proof after context + rollback | **Pending** (desktop GUI + CodeWalker) |

Automated verification: `npm run check` (0/0), `npm run build` (OK), tauri
`cargo test --lib` (27 passed), `cargo test -p server-agent` (127 passed),
`cargo build -p server-agent` (OK). Copied RPF SHA clean before/after; no apply run.

## Limitations

- Desktop GUI smokes (H2.1 + A/B/C) still pending manual execution.
- Context is prepended to the prompt (no structured scanner context arg yet).
- Run-history reload depends on the stored `mvp_report.json` still existing on disk.
- Only the `dataset/records` endpoint was added; `/features` and `/target-patterns`
  were not needed (target patterns are filterable via records).

## Next phase

**H2.3 — Structured Context Input + Real Redux Generation Evaluation**
(structured context arg into ReduxScannerEngine, generation-quality eval over the
attached corpus context, richer diff/eval surfacing).
