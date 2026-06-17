# T2.2 — HomeOps Redux Corpus Job Integration

## Goal

Integrate the ReduxScannerEngine T2.1 headless batch scanner into HomeOps so the
user can drop many Redux/mod packages into a server inbox and run a fully
automatic, **read-only** dataset build from the HomeOps UI. This phase wires only
the corpus/dataset scanner — no live Redux Maker generation, CodeWalker, RPF
apply, or AI module generation.

## Server paths

All corpus data lives on the Smart Pool **bulk** root and is derived from it (it
is never user-configurable through the UI):

```
bulk root: /mnt/storage/homeops-workspace
corpus root: /mnt/storage/homeops-workspace/redux-corpus
  ├── inbox        # user drops packages here (uploads via Files page)
  ├── input        # registered/normalized package inputs
  ├── work         # scratch work area
  ├── out          # per-package scan outputs
  ├── datasets     # corpus_dataset_records.jsonl, corpus_coverage_report.md
  ├── reports      # batch_report.json/.md, batch_index.json, quarantine_records.jsonl
  └── quarantine   # quarantined package records (metadata only)
```

## Smart Pool dependency

Corpus paths are derived from `storage_pool::build_server_pool()` — the H1.0
Smart Pool bulk/corpus root. `redux-corpus/*` is force-placed on bulk by the
existing placement policy, and folder bootstrap reuses
`storage_pool::bootstrap_standard_folders()` (symlink-safe, refuses any path
outside the bulk root, blocks `.homeops-tmp` / `.homeops-trash`).

## API endpoints (all under `/api/*`, API-token protected)

| Method | Path | Behavior |
| --- | --- | --- |
| GET  | `/api/redux-corpus/status` | enabled, scannerConfigured, scannerPath, scannerVersion, all corpus roots, directoriesOk, diskFree, smartPoolRootId (bulk), activeJob, latestBatchReport |
| POST | `/api/redux-corpus/bootstrap` | create/verify corpus folders on bulk (no scanner execution) |
| POST | `/api/redux-corpus/scan` | start one background scan job (single active scan enforced) |
| GET  | `/api/redux-corpus/reports/latest` | parsed `batch_report.json` + report/coverage file paths |
| GET  | `/api/redux-corpus/dataset/summary` | package/dataset/feature/target counts + category coverage |
| GET  | `/api/redux-corpus/quarantine` | parsed `quarantine_records.jsonl` summary |

No delete endpoints are added.

## HomeOps page behavior (`/redux-corpus`)

Sections: Corpus Status, Inbox / Drop-Zone (open in Files + copy path), Actions
(Bootstrap / Scan / Refresh / View Report / View Quarantine), Latest Batch
Report counts, Quarantine (metadata only, no delete), Output Files (safe paths).
"Scan corpus" is disabled when the scanner is not configured or a scan is active.
No apply, AI generation, CodeWalker, or RPF controls exist on the page.

## Job behavior

The scan runs as a standard HomeOps background job (`job_type =
redux_corpus_scan`) on the existing Jobs page. A single active scan is enforced
via an atomic flag in `JobRunner`; a second start returns
`REDUX_CORPUS_SCAN_ACTIVE`. The scanner runs as a child process spawned with an
**args array only** (no shell string, no user-supplied command/executable). Its
stdout/stderr are drained line-by-line into the job logs. The job succeeds only
when the scanner exits 0 and emits the `CORPUS_BATCH_OK` marker. The owned child
is reaped on drop (`kill_on_drop`); arbitrary process kill is never offered.

## Scanner binary deployment

The scanner is a fixed, admin-controlled binary — never an arbitrary path from
the UI. Default configured path:

```
/opt/homeops-tools/redux-scanner/redux-scanner
```

`scannerConfigured = redux_corpus.enabled && scanner_binary.is_file()`. A
sibling `VERSION` file (optional) is surfaced as `scannerVersion`. The binary
must be executable by the `homeops-agent` service user and must not be writable
by corpus inbox uploads.

Build/deploy on Ubuntu:

```bash
# copy source to /opt/redux-scanner-engine, then:
cargo build --manifest-path /opt/redux-scanner-engine/rpf_backend_rs/Cargo.toml --release
sudo mkdir -p /opt/homeops-tools/redux-scanner
sudo cp /opt/redux-scanner-engine/rpf_backend_rs/target/release/rpf_backend_rs \
        /opt/homeops-tools/redux-scanner/redux-scanner
sudo chmod 755 /opt/homeops-tools/redux-scanner/redux-scanner
```

The configured scanner subcommand is fixed to `scan-redux-corpus-batch`
(inspect uses `inspect-redux-corpus-batch`). The full invocation:

```
redux-scanner scan-redux-corpus-batch \
  --inbox-root      .../redux-corpus/inbox \
  --input-root      .../redux-corpus/input \
  --work-root       .../redux-corpus/work \
  --out-root        .../redux-corpus/out \
  --dataset-root    .../redux-corpus/datasets \
  --report-root     .../redux-corpus/reports \
  --quarantine-root .../redux-corpus/quarantine \
  --max-packages <cfg> --max-files-per-package <cfg> --max-bytes-per-package <cfg> \
  --include-archives --metadata-only
```

No apply/RPF/CodeWalker args are ever added.

## Config (server-side defaults)

```
redux_corpus.enabled                = true
redux_corpus.scanner_binary         = /opt/homeops-tools/redux-scanner/redux-scanner
redux_corpus.max_packages           = 100
redux_corpus.max_files_per_package  = 100000
redux_corpus.max_bytes_per_package  = 10737418240   # 10 GiB
redux_corpus.include_archives       = true
redux_corpus.allow_text_extract     = false
```

The scan always passes `--metadata-only`. `--allow-text-extract` is only passed
when `allow_text_extract` is explicitly enabled (admin opt-in; default off).

## Drop-zone workflow

1. Upload/move Redux/mod packages into `redux-corpus/inbox` (Files page).
2. Open HomeOps → Redux Corpus.
3. Bootstrap folders (first run) → Scan Corpus / Build Dataset.
4. Watch progress/logs on the Jobs page.
5. Dataset, reports, coverage, and quarantine records appear under
   `bulk/redux-corpus`. Source packages are never modified.

## Synthetic smoke result

Server-side smoke (2026-06-17) on `100.68.7.42:8787` with a synthetic
`SyntheticReduxPack/` folder + `broken_pack.zip` dropped in the inbox:

- `POST /api/redux-corpus/bootstrap` → all 7 folders present on `bulk`.
- `GET /api/redux-corpus/status` → `enabled=true`, `scannerConfigured=true`,
  `scannerVersion=T2.1-batch …`, `smartPoolRootId=bulk`, `directoriesOk=true`.
- `POST /api/redux-corpus/scan` → job `running` → `finished`.
- `GET /api/redux-corpus/reports/latest` → packagesTotal=2, scanned=2,
  quarantined=0, datasetRecords=3.
- `GET /api/redux-corpus/dataset/summary` → categoryCoverage
  `{oiv_installer:1, unknown:1, weapons:1}`.
- Outputs written under `bulk/redux-corpus/reports` (batch_report.json/.md,
  batch_index.json, batch_manifest.json) and `…/datasets`
  (corpus_dataset_records.jsonl, corpus_coverage_report.md).
- Source packages **unchanged** in the inbox; `input/` held only registration
  metadata; no original GTA file or RPF touched; no CodeWalker called.

Synthetic test data was removed from the server inbox after the smoke.

## Safety model

- Read-only corpus scanning only; sources are never mutated/moved/deleted.
- No original GTA files touched; no copied test RPF touched; no apply.
- No CodeWalker calls; no RPF write endpoint; no native RPF writing.
- No arbitrary shell execution; no arbitrary executable path — only the fixed
  configured binary, run with an args array and a fixed subcommand.
- All corpus paths are derived from the bulk root; traversal, absolute/Windows
  paths, symlink escapes, `.homeops-tmp`, `.homeops-trash` are rejected (Smart
  Pool bootstrap + path-safety helpers).
- One active corpus scan at a time; no parallel runs; no arbitrary process kill.
- API-token auth required for `/api/*`; Tailscale/UFW bind safety unchanged;
  HomeOps delete behavior unchanged (still off by default).

## Deployment result

- HomeOps backend deployed via `scripts/deploy_server_agent.ps1` (release build
  on server, service restarted, listener verified on `100.68.7.42:8787`,
  `/api/settings` 401 without token confirmed).
- Scanner built on Ubuntu from source in `$HOME/redux-scanner-build` and
  installed to `/opt/homeops-tools/redux-scanner/redux-scanner` (root-owned,
  mode 755, not writable by inbox uploads). Sibling `VERSION` written.
  `--help` verified (`HELP_OK`).

## Limitations

- HomeOps frontend has no JS unit-test runner; the frontend is verified via
  `svelte-check` + `vite build`. Backend safety is covered by Rust tests.
- `scannerVersion` is read from an optional sibling `VERSION` file; the binary is
  never executed for version probing.
- Live RPF apply / Redux Maker generation remains a local desktop concern and is
  intentionally absent from the server.

## Next phase

T2.3 — Corpus Coverage Matrix + Dataset Browser inside HomeOps.
