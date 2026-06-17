# H1.0 — Smart Storage Pool + Placement Policy

## Why not physically merge disks yet

The server has two drives (~1 TB system + ~4 TB bulk). Physically merging them
(LVM/ZFS/Btrfs/RAID/mergerfs) is destructive, risky, and irreversible without
downtime and backups. H1.0 instead adds a **logical** Smart Pool: HomeOps shows
the roots as one combined storage view and chooses the best root automatically,
while the disks stay physically separate. No repartition, format, mkfs, or
volume manager is involved.

## Architecture

A `SmartStoragePool` is built from the existing configured `storage_roots`
(`main` + `bulk`). It is a view + policy layer over the roots that already power
the Files module — not a new filesystem.

- `services/server-agent/src/storage_pool.rs` — pool model, placement policy,
  bootstrap. Pure `resolve_placement` over a pool snapshot; `build_server_pool`
  reads live capacity via `fs2`.
- Endpoints in `main.rs` expose the pool and placement to the UI.

## Root roles

| Root | Role | Reserve | Purpose |
|---|---|---|---|
| `main` | system | 25 GiB | metadata, small reports, logs, small projects |
| `bulk` | bulk | 100 GiB | large files, archives, Redux corpus |

Pool routing roots: `defaultRoot=main`, `metadataRoot=main`,
`largeFileRoot=bulk`, `corpusRoot=bulk`.

## Placement policy (backend-authoritative)

The frontend is never trusted; `resolve_placement` enforces:

1. **Redux corpus** paths (`redux-corpus/*`) and corpus/dataset intents → bulk.
2. **Large files** (≥ 2 GiB) → bulk.
3. **Archives** (`.zip .oiv .rpf .7z .rar`, unknown/large size) → bulk.
4. **Archive extraction** intent → bulk.
5. **Metadata / small files / reports** → main (default).
6. **Reserves:** never select a root if free space after the write would drop
   below its reserve (main 25 GiB, bulk 100 GiB).
7. **Fallback:** non-forced placements may fall back to another root with space;
   **forced-bulk (corpus) fails with a clear reason** if bulk is full rather than
   spilling to main.
8. A `preferredRootId` is honored only for non-forced placements.

Every decision returns: `allowed`, `selectedRootId`, `selectedRelativePath`,
`selectedAbsolutePath`, `reason`, `warnings`, `alternatives`,
`requiredFreeBytes`, `rootFreeBytes`.

### Path safety

`resolve_placement` rejects (allowed=false): traversal (`..`), absolute /
Windows-drive paths, backslash paths, and the reserved segments
`.homeops-tmp` / `.homeops-trash`. This reuses the existing `path_safety`
primitives — no path-safety weakening.

## Server paths

```
main -> /srv/homeops/workspace
bulk -> /mnt/storage/homeops-workspace
redux-corpus folders (on bulk):
  redux-corpus/{inbox,input,work,out,datasets,reports,quarantine}
```

## API endpoints (token-authenticated)

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/storage/pools` | all smart pools + root usage |
| GET | `/api/storage/pools/server` | the server smart pool summary |
| POST | `/api/storage/pools/server/resolve-placement` | backend placement decision |
| POST | `/api/storage/pools/server/bootstrap-standard-folders` | create redux-corpus folders on bulk |

No delete endpoint is added.

## UI behavior

- **/storage** page: Smart Pool card (combined capacity/free/used), Placement
  Policy panel (reserves + routing), per-root cards, a Placement Preview tool,
  and a Redux-corpus bulk-policy panel + bootstrap button.
- **Resources** page links to the Smart Pool.
- **Files** page root selector includes **Smart Pool**. In Smart Pool mode,
  uploads call `resolve-placement` per file and route to the chosen root
  (big/corpus → bulk, small → main); the chosen root + reason are shown in the
  upload panel. Browsing uses the default root; existing explicit `main` / `bulk`
  modes are unchanged.

HomeOps dark command-center styling; no app-wide redesign; no blue-heavy theme.

## Safety model

No disk formatting/partitioning/merge. No LVM/ZFS/Btrfs/RAID/mergerfs. No file
moves of existing data. No deletes; `allow_delete` stays false. No shell
execution, no service/process control. API token auth still required for
`/api/*`; Tailscale/UFW bind safety unchanged. All resolved paths stay inside a
configured root; bootstrap is symlink-escape safe.

## Deployment result

Deployed to `homeops-agent.service` (active, bound `100.68.7.42:8787`). Combined
pool: **4.92 TB total, 4.59 TB free** (main 854 GB free / bulk 3.74 TB free).

## Smoke result

- `GET /api/storage/pools/server` → healthy, 2 roots, defaultRoot=main, corpusRoot=bulk.
- large `mod.zip` (5 GiB) → **bulk** (large file).
- `summary.json` (2 KiB) → **main** (small/metadata default).
- `redux-corpus/inbox/pack` → **bulk** (redux-corpus path).
- bootstrap → created 7 folders on **bulk**; **none on main** (verified).
- Backend: `cargo test -p server-agent` 105 passed (19 new); `cargo build` OK.
- Frontend: `npm run check` 0 errors; `npm run build` OK.

## Limitations

- Logical pool only; no transparent cross-root filesystem and no automatic data
  migration. Cross-root move stays explicit/unsupported in Smart Pool mode.
- The Files aggregated tree view shows one root at a time (with a Smart Pool
  routing banner) rather than a merged listing.
- No frontend unit-test harness exists in `apps/web`; UI is validated via
  `svelte-check` + `vite build` and live API smoke.

## Next phase

**T2.2 — HomeOps Redux Corpus Job Integration**: run the ReduxScannerEngine
headless batch (T2.1) as a HomeOps job against the bulk `redux-corpus/` inbox,
using this smart pool placement policy.
