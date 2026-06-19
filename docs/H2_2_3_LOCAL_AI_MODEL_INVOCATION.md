# H2.2.3 — Fix Local AI Model Invocation Wiring

## Goal

Make HomeOps `/redux-maker` actually call the local Ollama model during
generation so the report shows `provider=ollama_local`, `localModelCalled=true`,
`cloudAiCalled=false`, `publicNetworkCall=false`, `applied=false`.

## Root cause

`/redux-maker` correctly detected Local AI as reachable and the bridge forwarded
`--provider ollama_local --allow-local-llm --local-llm-url http://127.0.0.1:11434`,
yet reports kept showing `localModelCalled=false`. Two independent bugs:

1. **No model name reached the scanner.** `build_run_args` in `redux_bridge.rs`
   already appended `--model <name>` when `input.model` was non-empty, but the UI
   default (`DEFAULT_BRIDGE_SETTINGS.model`) was the empty string. With no model,
   `ai-redux-maker` never invokes the local LLM at all.

2. **The local-LLM read timeout was too short.** The scanner's local-LLM HTTP
   client (`ai_plan/llm/model.rs`) defaults `timeout_ms` to `30_000`. A cold
   `qwen3.5:9b` (6.5 GB) takes ~57s to load + generate on the first call, so the
   request failed with a Windows connection timeout (`os error 10060`,
   `providerCalled=false`) and the run silently produced no local-model call. The
   bridge's `LOCAL_AI_TIMEOUT_SECS=240` is only the **process-kill** window, not
   the engine's per-request HTTP timeout, so it did not help.

A third, prompt-level subtlety worth recording: a pure *asset* prompt (e.g. "red
vertical tracers") resolves to a single `generated_texture_source` component with
**zero text/config child PatchPlans**, so the local LLM is never called regardless
of model wiring (`providerCalledCount=0`, `localModelCalled=false`). The LLM is
only invoked for text/config children such as `visualsettings`. This is correct,
not a bug — telemetry honestly reflects "no model call was needed".

## CLI help flag found

```
ai-redux-maker ... [--allow-local-llm] [--local-llm-url <loopback-url>] [--model <name>]
                   [--max-attempts <n>] [--timeout-ms <n>] [--fallback-to-rule-based] ...
```

The model flag is **`--model <name>`** and the per-request read timeout is
**`--timeout-ms <n>`** (milliseconds). Both were already supported by the scanner.

## Manual CLI proof

Engine: `…\rpf_backend_rs\target\release\rpf_backend_rs.exe`. Ollama running on
`127.0.0.1:11434` with `qwen3.5:9b` pulled.

- Asset-only prompt ("red vertical tracers"), `--model qwen3.5:9b`:
  `providerCalledCount=0`, `localModelCalled=false` — expected (0 text children).
- `visualsettings` prompt, `--model qwen3.5:9b`, **no** `--timeout-ms`:
  call attempted but timed out at 30023 ms → `os error 10060`,
  `providerCalled=false`, `finalBlockedReason="local LLM server unavailable…"`.
- `visualsettings` prompt, `--model qwen3.5:9b --timeout-ms 180000`, warm model:

  ```
  provider=ollama_local  model=qwen3.5:9b
  localModelCalled=True   modelCalled=True   providerCalledCount=1
  parseSuccess=1          cloudAiCalled=False  publicNetworkCall=False
  fallbackUsed=False      applied=False
  ```

  The model was genuinely invoked and returned a parseable plan. (The plan was
  then validator-gated — `blocked_stage` — which is the honest deterministic gate,
  not a model-call failure.)

## HomeOps bridge fix

- `apps/web/src/lib/redux-maker/bridgeSettings.ts` — default `model` changed from
  `''` to `'qwen3.5:9b'` (loopback URL and the existing loopback enforcement are
  unchanged).
- `apps/web/src-tauri/src/redux_bridge.rs` — added `LOCAL_AI_LLM_TIMEOUT_MS =
  180_000` and append `--timeout-ms 180000` to the run argv for any local
  provider. (`--model`, `--allow-local-llm`, `--local-llm-url` forwarding already
  existed.) Kept under the 240s process-kill window.
- `apps/web/src/lib/components/redux-maker/ReduxMakerSettingsModal.svelte` —
  model field relabelled "installed Ollama model", default placeholder
  `qwen3.5:9b`, quick-pick `<datalist>` (`qwen3.5:9b`, `qwen2.5:7b`), hint that the
  model must be pulled in Ollama.

No scanner/engine change was required.

## Command preview after fix (ollama_local)

```
rpf_backend_rs.exe ai-redux-maker --prompt "…" --out-dir … --out … \
  --provider ollama_local --mode full_plan_no_execute --workspace … \
  --allow-local-llm --local-llm-url http://127.0.0.1:11434 \
  --model qwen3.5:9b --timeout-ms 180000 --fallback-to-rule-based
```

## Report parsing

`mvp_report.json` keeps the AI facts under `safetyFacts` (`localModelCalled`,
`cloudAiCalled`, `publicNetworkCall`, `modelCalled`); `fallbackUsed` is top-level.
The bridge already reads these from the correct locations (`snapshot_job`,
`fallback_used`). A regression test pins the real shape and asserts
`localModelCalled` is **not** read from the top level. Nothing is hardcoded —
telemetry mirrors the report.

## Tests added (Tauri lib, `redux_bridge::tests`)

1. `ollama_local_run_includes_model_arg`
2. `ollama_local_run_includes_allow_local_llm`
3. `ollama_local_run_includes_timeout_ms`
4. `ollama_local_run_rejects_non_loopback_url`
5. `rule_based_run_does_not_include_model_arg`
6. `command_preview_includes_model_when_ollama_local`
7. `report_parser_reads_local_model_called_from_real_report_shape`

`rule_based_omits_local_flags` extended to assert `--timeout-ms` is absent for
rule_based. Tauri lib: **36 tests, all pass**.

## GUI smoke

The headless run of the exact bridge argv (model + timeout) is the proof of the
fix. Driving the Tauri desktop GUI is a manual operator step; the operator
checklist is: start Ollama → confirm `Invoke-RestMethod
http://127.0.0.1:11434/api/tags` → open desktop `/redux-maker` → Bridge Settings
(`ollama_local`, allow local AI on, URL `http://127.0.0.1:11434`, model
`qwen3.5:9b`) → run a `visualsettings` prompt → confirm the command preview shows
`--model qwen3.5:9b --timeout-ms 180000` and telemetry shows `localModelCalled=true`,
`cloudAiCalled=false`, `publicNetworkCall=false`, `applied=false`.

## Safety notes

- No server-side RPF apply; no CodeWalker server controls; no server-side
  CodeWalker calls; no RPF write endpoints in server-agent.
- No original GTA archive access; no copied-RPF mutation (no apply this phase).
- Fixed scanner binary path only; args passed as an argv array (no shell).
- Local AI URL is loopback-only; public model URLs are blocked (unchanged).
- Validators were not weakened. The 180s value is a network read timeout, not a
  safety gate, and stays inside the 240s process-kill window.
- HomeOps auth/Tailscale and delete behavior unchanged.

## Limitations

- The default `qwen3.5:9b` must actually be pulled in Ollama; an unpulled model
  fails the request (and, with `--fallback-to-rule-based`, falls back).
- Model discovery from `/api/tags` is **not** implemented; the settings field is a
  text input with a static quick-pick list (sufficient for H2.2.3).
- A cold first call still takes ~60s; the 180s timeout covers it but the run is
  slow until the model is warm.
