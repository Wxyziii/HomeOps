# HomeOps MVP Code Review

## Executive Summary

HomeOps Panel has a solid MVP foundation for local/tunnel-only server control. The current implementation includes a Tauri desktop shell, SvelteKit UI, an Axum `server-agent`, config loading, SQLite persistence, workspace-constrained file operations, uploads/downloads, background jobs, operation logs, and ZIP extraction jobs. The strongest positive finding is that the dangerous classes of behavior originally forbidden for V1 are still absent: there is no arbitrary command endpoint, no shell/process Tauri plugin, no public bind, and delete is disabled.

The audit did not find a confirmed critical workspace escape. The backend consistently routes file-like paths through `path_safety::resolve_workspace_path`, and symlinks that resolve outside the workspace are rejected or marked unsafe. However, several readiness gaps remain before this should be treated as a polished unattended MVP:

- Some settings exposed in the UI are stored in SQLite but not actually enforced by the running backend (`allow_archive_extract`, `max_parallel_jobs`).
- Queued job cancellation can be overwritten by the already-spawned runner, so a cancelled queued job may still run later.
- The path parser normalizes leading `/` and `\` instead of rejecting them, so absolute-looking paths are not rejected consistently.
- Dashboard and Archives still show mock data where real backend data now exists.
- There is no auth yet, so the service must remain localhost/tunnel-only.

## MVP Readiness Verdict

**Ready for systemd after minor fixes**

The service is safe enough for local-only, SSH-tunneled MVP use because it binds to `127.0.0.1`, has no arbitrary command execution, has no native Tauri filesystem/shell/process permissions, and keeps file writes constrained to `workspace_root`. That said, the current implementation should not be considered ready for LAN/public exposure, and the high/medium issues below should be addressed before relying on the app as an unattended daily operations panel.

If systemd is already installed, it can remain running for tunnel-only testing. The fixes most worth doing before continued systemd use are: fix queued cancellation, make runtime-affecting settings honest, and make absolute path rejection strict.

## Critical Issues

No confirmed Critical issues were found in the current local-only implementation.

There is no observed workspace escape, unsafe delete, public bind, arbitrary command execution, or dangerous Tauri native permission. The absence of auth would become Critical only if the backend were bound to LAN/public or exposed through an untrusted tunnel.

## High Issues

### H-01

- **Severity:** High
- **Area:** Config / Settings / Archive Jobs
- **File/function:** `services/server-agent/src/main.rs::normalize_safe_setting`, `services/server-agent/src/jobs.rs::ArchiveExtractTask::new`, `apps/web/src/routes/settings/+page.svelte::saveBackendSettings`
- **Problem:** The UI can update `allow_archive_extract` in SQLite through `PUT /api/settings`, but archive extraction checks `AppConfig.allow_archive_extract`, not the SQLite setting. Disabling archive extraction in Settings does not disable `POST /api/archives/extract`.
- **Why it matters:** This creates a false safety control. The UI says a risky operation has been disabled while the backend still allows it until the config file changes and the service restarts.
- **How to fix:** Choose one source of truth. Either make `allow_archive_extract` a runtime config value only and remove it from editable UI settings, or make archive extraction read the current value from SQLite at job creation time. If it stays in SQLite, seed it there and enforce it there.
- **Must fix before:** Treating Settings as authoritative; LAN/auth/public exposure; adding more archive formats or import workflows.

### H-02

- **Severity:** High
- **Area:** Job System / Cancellation
- **File/function:** `services/server-agent/src/jobs.rs::cancel_job`, `services/server-agent/src/jobs.rs::run_job`, `services/server-agent/src/db.rs::update_job_running`
- **Problem:** Queued cancellation is not race-safe. `cancel_job` can mark a queued job as `cancelled`, but the spawned `run_job` task later acquires the semaphore and unconditionally marks the same job `running`.
- **Why it matters:** The system can claim a job was cancelled while still executing it later. This is tolerable for toy test jobs, but unsafe once backup, extraction, AI import, or other long tasks are added.
- **How to fix:** Make the running transition conditional in SQL: update from `queued` to `running` only if current status is still `queued`, then check affected rows. If the job is already `cancelled`, exit without running. Add a regression test that queues behind `max_parallel_jobs=1`, cancels before semaphore acquisition, and confirms the job never runs.
- **Must fix before:** More real job types; unattended systemd operation with user-triggered long tasks; LAN/auth/public exposure.

### H-03

- **Severity:** High
- **Area:** Auth / Exposure Model
- **File/function:** `services/server-agent/src/main.rs` routes, `services/server-agent/src/config.rs::ensure_local_bind`
- **Problem:** There is no authentication or authorization. This is acceptable only while the backend binds to `127.0.0.1` and is accessed through a trusted SSH tunnel.
- **Why it matters:** If the bind address is ever relaxed or the port is forwarded by another mechanism, every endpoint becomes available to whoever can reach the port. That includes upload, rename, move, archive extraction, and future job controls.
- **How to fix:** Before LAN/Tailscale/public exposure, add token-based auth at minimum. Require auth on all `/api/*` routes, store the token outside the repo, rotate it through config, redact it from logs, and update the Tauri app to send it. Keep CORS explicit and narrow.
- **Must fix before:** LAN, Tailscale direct backend URL, public exposure, or shared-machine usage.

## Medium Issues

### M-01

- **Severity:** Medium
- **Area:** Path Safety
- **File/function:** `services/server-agent/src/path_safety.rs::parse_relative_path`
- **Problem:** `parse_relative_path` trims leading and trailing `/` and `\` before checking whether the path is absolute. A request like `/etc/passwd` becomes `etc/passwd` and is treated as a workspace-relative path instead of being rejected as absolute.
- **Why it matters:** This does not create a workspace escape because the normalized path remains inside `workspace_root`, but it violates the MVP requirement to reject absolute paths and creates inconsistent security semantics.
- **How to fix:** Check `Path::is_absolute`, Windows prefixes, leading `/`, leading `\`, and drive-like patterns before trimming. Prefer rejecting absolute-looking input rather than normalizing it. Add tests for `/etc/passwd`, `\\server\\share`, `\Windows`, `C:\Windows`, and `/srv/homeops/workspace/file`.
- **Must fix before:** Relying on path rejection semantics in UX or logs; LAN/auth/public exposure.

### M-02

- **Severity:** Medium
- **Area:** Path Safety / Windows-style Paths
- **File/function:** `services/server-agent/src/path_safety.rs::parse_relative_path`
- **Problem:** On Linux, a path like `C:\Windows\file.txt` is not a Windows prefix to Rust's Unix `Path` parser. It can be accepted as a single normal component containing backslashes.
- **Why it matters:** It still stays inside the workspace, but it violates the requested Windows-style path handling and can create confusing filenames that look like external paths.
- **How to fix:** Reject `\` anywhere in API path input unless the project explicitly decides to normalize it to `/`. Reject drive-letter patterns with a small platform-independent check like the one already used for ZIP entries.
- **Must fix before:** LAN/auth/public exposure; cross-platform path-heavy UI polish.

### M-03

- **Severity:** Medium
- **Area:** Job System / Settings
- **File/function:** `services/server-agent/src/jobs.rs::JobRunner::new`, `services/server-agent/src/main.rs::normalize_safe_setting`, `apps/web/src/routes/settings/+page.svelte`
- **Problem:** `max_parallel_jobs` can be edited in Settings, but the `JobRunner` semaphore is created from startup config and never updates after `PUT /api/settings`.
- **Why it matters:** The UI implies the concurrency limit changed, but the running service continues using the old semaphore size until restart and config changes.
- **How to fix:** Either make `max_parallel_jobs` config-only and mark it restart-required/read-only in UI, or implement a runtime job-runner setting that reads from SQLite and adjusts queue scheduling safely.
- **Must fix before:** Users depend on settings to control resource usage; more CPU/disk-heavy jobs.

### M-04

- **Severity:** Medium
- **Area:** Archive Extraction
- **File/function:** `services/server-agent/src/jobs.rs::extract_zip_archive`
- **Problem:** Failed extraction can leave files that were extracted before the failure. This is especially visible if an archive has safe entries first and a malicious/overwrite/limit-triggering entry later.
- **Why it matters:** The job correctly fails and logs the unsafe entry, but the destination may contain a partial tree. Users may mistake partial output for a successful extraction or retry into a dirty destination.
- **How to fix:** Extract into a job-specific staging directory under the destination parent, then atomically rename to the final destination on success. On failure, remove the staging directory or leave it clearly named as failed with logs. Continue refusing overwrites by default.
- **Must fix before:** Large real-world archive use; AI Redux import workflows that consume extracted output.

### M-05

- **Severity:** Medium
- **Area:** Archive Extraction / Runtime Performance
- **File/function:** `services/server-agent/src/jobs.rs::extract_zip_archive`
- **Problem:** ZIP extraction uses synchronous `std::fs` and blocking ZIP reads inside a Tokio task.
- **Why it matters:** For large archives, this can occupy async runtime worker threads. The semaphore limits the number of jobs, but the extraction itself is still blocking work on Tokio's async runtime.
- **How to fix:** Wrap archive extraction in `tokio::task::spawn_blocking` or create a dedicated blocking job worker. Keep the async job lifecycle around the blocking operation.
- **Must fix before:** Heavy archive workloads or multiple concurrent long-running job types.

### M-06

- **Severity:** Medium
- **Area:** Frontend / Dashboard Data
- **File/function:** `apps/web/src/routes/+page.svelte`, `apps/web/src/lib/data/mock.ts`
- **Problem:** Dashboard Active jobs and Recent logs still use mock data even though real job and operation log endpoints exist.
- **Why it matters:** The dashboard can mislead the user about actual server state. This is a major UX mismatch for an operations panel.
- **How to fix:** Replace mock jobs/logs with `GET /api/jobs` and `GET /api/logs/operations`. Keep CPU/memory/storage mock cards explicitly labeled as placeholders until resource monitoring exists.
- **Must fix before:** Calling the MVP operationally complete.

### M-07

- **Severity:** Medium
- **Area:** Frontend / Archives Page
- **File/function:** `apps/web/src/routes/archives/+page.svelte`, `apps/web/src/lib/data/mock.ts`
- **Problem:** Archives page is entirely mock data and has placeholder actions, while real ZIP extraction is available only from Files.
- **Why it matters:** A user navigating to Archives expects real archive visibility. The page can imply backup/archive management exists when it does not.
- **How to fix:** Either mark the page clearly as placeholder/secondary, or list real `.zip` files from the workspace and expose the same extraction action as Files.
- **Must fix before:** Treating Archives as a real MVP page.

### M-08

- **Severity:** Medium
- **Area:** Frontend / Files UI Coverage
- **File/function:** `apps/web/src/routes/files/+page.svelte`, `apps/web/src/lib/api/client.ts::moveFile`
- **Problem:** `POST /api/files/move` exists in the backend and API client, but there is no UI workflow for move.
- **Why it matters:** This is an implemented backend feature with no app access and no documented reason. It currently works only through curl/API.
- **How to fix:** Add a compact "Move to..." action using a validated destination prompt or folder picker, or intentionally hide/document move as API-only until a better UI exists.
- **Must fix before:** Claiming full Files page feature coverage.

### M-09

- **Severity:** Medium
- **Area:** Tauri Security
- **File/function:** `apps/web/src-tauri/tauri.conf.json`
- **Problem:** Tauri config has `"csp": null`.
- **Why it matters:** Current Tauri permissions are minimal, but frontend XSS would still be able to call the backend API if the user has a server URL saved. With no auth, this increases impact.
- **How to fix:** Add a restrictive CSP that allows the app itself, Tabler font assets, and configured HTTP connections needed for development. Revisit for packaged Tauri origins.
- **Must fix before:** LAN/auth/public exposure; packaged distribution beyond this private dev machine.

### M-10

- **Severity:** Medium
- **Area:** Upload / Error Consistency
- **File/function:** `services/server-agent/src/main.rs` `DefaultBodyLimit`, `services/server-agent/src/files.rs::upload_files`
- **Problem:** Upload code returns consistent API errors for multipart and per-file size failures, but Axum body limit rejections may bypass the custom `{ ok:false, error, code }` shape.
- **Why it matters:** The frontend may show a generic HTTP/body-limit failure for oversized requests instead of the expected API error format.
- **How to fix:** Add a route-level rejection handler or custom extractor path that converts body limit errors into `UPLOAD_TOO_LARGE`.
- **Must fix before:** Polished upload UX; large-file field testing.

### M-11

- **Severity:** Medium
- **Area:** Systemd Hardening
- **File/function:** Deployment unit `/etc/systemd/system/homeops-agent.service` from Phase 2F.2
- **Problem:** The service uses `NoNewPrivileges=true` and `PrivateTmp=true`, but it does not yet restrict writable paths or system access further.
- **Why it matters:** If the backend is compromised, systemd can reduce blast radius beyond running as `marcel`.
- **How to fix:** Consider `ProtectSystem=strict`, `ProtectHome=true`, `ReadWritePaths=/srv/homeops`, `PrivateDevices=true`, `RestrictSUIDSGID=true`, `LockPersonality=true`, and a narrow `UMask`. Test carefully because SQLite and logs need writes.
- **Must fix before:** LAN/auth/public exposure.

## Low Issues

### L-01

- **Severity:** Low
- **Area:** Frontend / Success Messages
- **File/function:** `apps/web/src/routes/files/+page.svelte::refresh`, `createNewFolder`, `renameEntry`, `handleUpload`
- **Problem:** `refresh()` clears `actionMessage`, so success messages set before `await refresh()` disappear for create, rename, and upload.
- **Why it matters:** Users do not get confirmation for successful actions.
- **How to fix:** Move success message assignment after refresh, or add a refresh option that does not clear messages.
- **Must fix before:** UX polish.

### L-02

- **Severity:** Low
- **Area:** Frontend / Job UX
- **File/function:** `apps/web/src/routes/jobs/+page.svelte::requestCancel`
- **Problem:** Cancel button is shown for all jobs. Finished jobs and running jobs surface errors rather than clear disabled states.
- **Why it matters:** It is honest, but noisy and confusing.
- **How to fix:** Show cancel only for queued jobs until running cancellation exists. For running jobs, show a disabled button/tooltip stating that running cancellation is not implemented.
- **Must fix before:** UX polish.

### L-03

- **Severity:** Low
- **Area:** Frontend / Files Search
- **File/function:** `apps/web/src/routes/files/+page.svelte`
- **Problem:** The search input is present but not wired to filter results.
- **Why it matters:** It looks functional but does nothing.
- **How to fix:** Add client-side filtering for current folder results or remove/label it until search exists.
- **Must fix before:** UX polish.

### L-04

- **Severity:** Low
- **Area:** Frontend / Download UX
- **File/function:** `apps/web/src/routes/files/+page.svelte::downloadEntry`
- **Problem:** Downloads use `window.location.href`, so API errors cannot be rendered in the Files page.
- **Why it matters:** Failed downloads may feel like nothing happened.
- **How to fix:** Fetch first to detect errors or open in a hidden anchor only after constructing a validated URL.
- **Must fix before:** UX polish.

### L-05

- **Severity:** Low
- **Area:** Documentation
- **File/function:** `README.md`
- **Problem:** README is stale. It still says several implemented features are not implemented and says systemd was not created.
- **Why it matters:** Future agents/humans may make wrong assumptions.
- **How to fix:** Update README after the audit to reflect current status: systemd installed, uploads/jobs/logs/ZIP extraction working, Archives/Dashboard still partly mock, auth/WebSockets/resource monitoring still missing.
- **Must fix before:** Onboarding another contributor or packaging a release.

### L-06

- **Severity:** Low
- **Area:** Frontend / Test Jobs
- **File/function:** `apps/web/src/routes/jobs/+page.svelte`, `services/server-agent/src/main.rs::run_test_sleep`, `run_test_fail`
- **Problem:** Test job buttons are visible in the main Jobs page.
- **Why it matters:** They are safe internal jobs, but they clutter production-like UI and create artificial logs.
- **How to fix:** Keep them behind a development flag or move them to Settings/Diagnostics.
- **Must fix before:** Production polish.

## Feature/UI Coverage Matrix

| Backend feature / endpoint | Implemented? | UI exposed? | UI location/page | Status | Notes | Recommended fix |
|---|---:|---:|---|---|---|---|
| `GET /health` | Yes | Yes | Settings, Dashboard connection chip | Complete | Used through shared connection store. | Keep. |
| `GET /api/settings` | Yes | Yes | Settings | Partial | Runtime config and SQLite settings are shown. Some displayed/editable settings are not enforced at runtime. | Fix `allow_archive_extract` and `max_parallel_jobs` source-of-truth mismatch. |
| `PUT /api/settings` | Yes | Yes | Settings | Partial | Protects dangerous runtime fields, but editable safe settings can be misleading. | Make editable settings enforceable or mark restart/config-only. |
| `GET /api/workspace` | Yes | Yes | Settings | Complete | Shows root, writable state, free space, and safety status. | Keep; add server identity later. |
| `GET /api/files/list` | Yes | Yes | Files | Complete | Lists workspace folders/files and symlink warnings. | Tighten absolute path rejection. |
| `POST /api/files/create-folder` | Yes | Yes | Files | Complete | Parent must exist; no recursive create. | Keep; improve success message. |
| `POST /api/files/rename` | Yes | Yes | Files row action | Complete | Rejects overwrite and root. | Keep; add better modal later. |
| `POST /api/files/move` | Yes | No | None | Partial | API/client exists but no UI workflow. | Add "Move to..." UI or document as intentionally hidden. |
| `GET /api/files/download` | Yes | Yes | Files row action | Complete | Attachment download; directory rejected. | Improve frontend error handling. |
| `POST /api/files/upload` | Yes | Yes | Files topbar upload | Complete | Filename sanitization and no overwrite. | Add progress and body-limit error shape. |
| `POST /api/files/delete` | Guard only | Hidden/disabled | Files row action hidden unless `allow_delete=true` | Intentionally hidden | Delete disabled by config and not implemented. | Keep hidden; implement trash semantics only after explicit phase. |
| `POST /api/archives/extract` | Yes | Yes | Files row action for `.zip` | Complete for Files; Archives page missing | ZIP-only, background job, blocks unsafe entries. | Add Archives page integration or mark Archives placeholder. |
| `GET /api/jobs` | Yes | Yes | Jobs | Complete | Newest-first, limit 200. | Consider pagination later. |
| `GET /api/jobs/:id` | Yes | Yes indirectly | Jobs selected job state uses list; API client exists | Partial | Client has `getJob`, but Jobs page mostly relies on list. | Use `getJob` for selected detail refresh if needed. |
| `GET /api/jobs/:id/logs` | Yes | Yes | Jobs selected log panel | Complete | Default limit 500; backend clamps. | Keep. |
| `POST /api/jobs/test-sleep` | Yes | Yes | Jobs | Complete but diagnostic | Safe internal job, no shell. | Move behind diagnostics/dev flag later. |
| `POST /api/jobs/test-fail` | Yes | Yes | Jobs | Complete but diagnostic | Safe internal failing job. | Move behind diagnostics/dev flag later. |
| `POST /api/jobs/:id/cancel` | Yes | Yes | Jobs cancel button | Partial | Running cancellation honest; queued cancellation has race. | Fix queued transition race and hide button except queued. |
| `GET /api/logs/operations` | Yes | Yes | Logs | Complete | Polls last 100 operation logs. | Add filters later. |

## Positive Findings

- `server-agent` binds to `127.0.0.1` and rejects non-loopback bind config in `config::ensure_local_bind`.
- No arbitrary shell execution or external process runner exists.
- Tauri has only `core:default` capability and no filesystem, shell, process, updater, or notification plugin.
- File list, create-folder, rename, move, download, upload, and archive extraction all route through workspace path validation.
- Delete is explicitly guarded and disabled by default.
- Upload filenames reject separators, empty names, `.`/`..`, control characters, and long names.
- Uploads use `create_new(true)` and attempt partial cleanup on write failure.
- ZIP extraction is native Rust, ZIP-only, background-job based, and rejects traversal, absolute entries, backslash paths, drive-like paths, symlink-like entries, overwrites, too many entries, and too many bytes.
- Job logs are stored in SQLite and append-only job log files.
- Operation logs exist and power the Logs page.
- SQLite queries use bind parameters rather than SQL string interpolation.
- `.gitignore` excludes `node_modules`, build artifacts, `target`, DB files, logs, env files, config files, and secrets.

## Recommended Fixes Before Systemd

Systemd has already been installed and verified. If treating this as "before keeping systemd as the normal runtime," prioritize:

- [ ] Fix queued cancellation so a cancelled queued job cannot later run.
- [ ] Make `allow_archive_extract` either config-only/read-only or enforce the SQLite value at job creation.
- [ ] Make `max_parallel_jobs` either config-only/read-only or enforce it dynamically.
- [ ] Tighten `parse_relative_path` so absolute-looking paths and Windows-style paths are rejected, not normalized.
- [ ] Update README so future operation notes match the deployed state.

## Recommended Fixes Before LAN/Auth Exposure

- [ ] Add authentication to all `/api/*` routes.
- [ ] Keep `bind_host` local until auth is implemented and tested.
- [ ] Add token storage/rotation guidance and never commit tokens.
- [ ] Add a restrictive Tauri CSP instead of `"csp": null`.
- [ ] Revisit CORS for the final packaged Tauri origin and any LAN origin.
- [ ] Add rate limits or request size limits with consistent JSON API errors.
- [ ] Harden systemd further with `ProtectSystem`, `ReadWritePaths`, `ProtectHome`, and related sandboxing after testing.
- [ ] Hide or gate diagnostic test jobs.
- [ ] Add a proper server identity/status display so the UI makes clear whether it is connected to local dev or the real Ubuntu server.
- [ ] Add audit log entries for file uploads, downloads, rename/move, and archive extraction requests, not only job lifecycle.

## Recommended Tests To Add

- Path parser tests for `/etc/passwd`, `/srv/homeops/workspace/file`, `\Windows`, `\\server\\share`, and `C:\Windows` on Linux and Windows.
- Integration tests for every file endpoint returning the consistent `{ ok:false, error, code }` error shape.
- Upload test where Axum body limit is exceeded, verifying the response shape is still API-consistent.
- Queued cancellation race test with `max_parallel_jobs=1`: queue two jobs, cancel the second, confirm it never transitions to running.
- Test that changing `allow_archive_extract` through Settings actually disables extraction, or test that the endpoint rejects attempts to mutate config-only settings.
- Test that changing `max_parallel_jobs` either takes effect or returns a restart-required/read-only response.
- Archive partial failure test proving staging/cleanup behavior after the staging-dir fix.
- Frontend component/e2e test for Files extraction workflow: upload ZIP, start extraction, navigate to Jobs, see logs.
- Frontend test that Delete controls are hidden when `allow_delete=false`.
- Tauri config assertion that dangerous plugins remain absent.

## Files And Functions That Need Attention

- `services/server-agent/src/path_safety.rs::parse_relative_path`
- `services/server-agent/src/jobs.rs::cancel_job`
- `services/server-agent/src/jobs.rs::run_job`
- `services/server-agent/src/db.rs::update_job_running`
- `services/server-agent/src/jobs.rs::ArchiveExtractTask::new`
- `services/server-agent/src/jobs.rs::extract_zip_archive`
- `services/server-agent/src/main.rs::normalize_safe_setting`
- `services/server-agent/src/main.rs` route/body-limit setup
- `apps/web/src/routes/settings/+page.svelte::saveBackendSettings`
- `apps/web/src/routes/+page.svelte`
- `apps/web/src/routes/archives/+page.svelte`
- `apps/web/src/routes/files/+page.svelte`
- `apps/web/src/routes/jobs/+page.svelte`
- `apps/web/src-tauri/tauri.conf.json`
- `README.md`

## Final Recommendation

Keep using the deployed systemd service only in the current local-only/SSH-tunnel model. Do not expose the backend over LAN, Tailscale direct URL, or public network yet.

The next engineering step should be to fix the high-confidence correctness issues before adding new features:

1. Fix queued cancellation.
2. Resolve the runtime settings source-of-truth mismatch.
3. Tighten absolute and Windows-style path rejection.
4. Replace Dashboard mock jobs/logs with real backend data.
5. Update README/deployment docs.

After those fixes, the MVP is in good shape for continued local server use. Before any broader network exposure, add authentication, CSP, CORS hardening, and stronger systemd sandboxing.
