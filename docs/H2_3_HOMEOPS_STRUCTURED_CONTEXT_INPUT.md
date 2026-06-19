# H2.3 — Structured Context Input (HomeOps)

HomeOps side of H2.3. `/redux-maker` now ships a **structured corpus context pack**
to the local ReduxScannerEngine via `--context-pack`, in addition to the visible
text block it already appended to the prompt.

## What changed

- **`apps/web/src/lib/redux-maker/contextPack.ts`** — adds `buildStructuredContextPack(records, prompt)`
  and `serializeStructuredContextPack(pack)`. The structured pack matches the engine
  schema v1 (`schemaVersion`, `source`, `query`, `summary`, `records[]`,
  `constraints`). Evidence is sanitized: base64 `data:` URIs stripped, ASCII control
  chars removed, capped to 600 chars; records capped to 200. Never embeds binary or
  copyrighted asset bytes.
- **`ReduxMakerStudio.svelte`** — keeps the raw selected records; on generate it
  builds the structured pack against the live prompt and passes `contextPackJson` to
  the bridge (only when records are attached). The visible context block still goes
  into the prompt so the user always sees what was attached.
- **`bridge.ts`** — `StartRunInput.contextPackJson`; `RunStatus` gains
  `corpusContextAttached`, `contextRecordCount`, `contextCategories`,
  `contextTargetPatterns`, `contextAppliedToPrompt`, `contextWarnings`, read from the
  real report shape (top-level fields; AI facts stay under `safetyFacts`).
- **`src-tauri/src/redux_bridge.rs`** — `redux_maker_start_run` accepts
  `contextPackJson`, enforces a 256 KiB cap, writes it to
  `<run-dir>/context_pack.json`, and adds `--context-pack <path>` to the argv. The
  pack path lives under `.tmp/homeops-runs` and never trips the apply/write
  forbidden-token guard. Context is plan-only; it is never part of an apply argv.
- **`ReduxMakerTelemetry.svelte`** — surfaces report-derived context telemetry (ctx
  in report, ctx applied (local LLM vs advisory), categories, targets, warnings).

## Bridge command preview (with context, ollama_local)

```
rpf_backend_rs.exe ai-redux-maker --prompt "…" --out-dir … --out … \
  --provider ollama_local --mode full_plan_no_execute --workspace … \
  --allow-local-llm --local-llm-url http://127.0.0.1:11434 \
  --model qwen3.5:9b --timeout-ms 180000 \
  --context-pack …\.tmp\homeops-runs\run-<id>\context_pack.json --fallback-to-rule-based
```

## Local AI proof (headless, mirrors the bridge argv)

visualsettings prompt + `--context-pack`, Ollama qwen3.5:9b:

```
corpusContextAttached=true  contextRecordCount=1
contextCategories=visualsettings  contextTargetPatterns=update_rpf
contextAppliedToPrompt=true  localModelCalled=true
cloudAiCalled=false  publicNetworkCall=false  applied=false
```

A no-apply `eval-redux-context-generation` run (provider ollama_local, 2 cases) gave
`safetyOk=true`, the unsafe original-GTA case rejected in both arms, no apply, no
cloud, no public network.

## Safety model

- Context is advisory and plan-only; it never enables an apply or relaxes a
  validator. Bad/oversized/binary packs are rejected (engine continues without
  context, recording why).
- Local AI loopback-only; public model URLs blocked; no cloud AI; no public network.
- No server-side RPF apply, no CodeWalker server controls, no new RPF write endpoints
  in server-agent. The copied test RPF is never mutated in this phase.

## Limitations

- Context-driven correctness gains are expected mainly with a local LLM; the
  deterministic rule-based generator records context but does not consume the prose.
- Ollama model discovery is still a manual text field (default `qwen3.5:9b`).
- Generated context packs are written under `.tmp/homeops-runs` and are never
  committed.

## Next phase

H2.4 — Corpus Coverage Matrix + Dataset Browser.
