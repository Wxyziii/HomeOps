# HomeOps Production MVP Review

## Executive Summary

HomeOps Panel is now a credible private-server MVP for direct Tailscale use. The desktop UI remains on the PC as a SvelteKit/Tauri app, while the Ubuntu server runs only the Rust `server-agent` under systemd. The current deployment uses API token protection for `/api/*`, a Tailscale-only bind at `100.68.7.42:8787`, UFW restricted to `tailscale0`, a restrictive Tauri CSP, and no dangerous native Tauri plugins.

The review found no confirmed Critical issues. The implementation still should not be exposed to public internet or general LAN, but it is reasonable for private Tailscale use by the owner with the current token model and firewall assumptions.

## MVP Readiness Verdict

**Ready for direct Tailscale production use.**

This verdict means: safe enough for the intended private owner-operated MVP over Tailscale, with API token auth enabled and `allow_delete=false`. It does not mean ready for public internet, shared multi-user LAN, or untrusted users.

Not ready for:

- Public internet exposure.
- General LAN exposure.
- Multi-user auth/authorization.
- Service/process control.
- Running arbitrary tools or shell commands.

## Architecture Audit

### PC App

- The main user interface is the Tauri desktop app wrapping the existing SvelteKit/Tailwind frontend.
- The frontend stores the server URL and API token locally in browser/Tauri storage.
- Tauri capabilities remain minimal with `core:default`.
- No Tauri filesystem, shell, process, updater, notification, or native server-control plugins are enabled.

### Ubuntu Server-Agent

- The server runs the Rust Axum `server-agent` only.
- The server-agent is installed as `homeops-agent.service`.
- It runs as user/group `marcel`.
- Runtime paths:
  - Binary: `/srv/homeops/agent/bin/server-agent`
  - Config: `/srv/homeops/data/homeops_config.json`
  - Workspace: `/srv/homeops/workspace`
  - Data: `/srv/homeops/data`
  - Logs: `/srv/homeops/logs`

### Connection Modes

- Tunnel mode remains supported through SSH forwarding.
- Direct Tailscale mode is enabled for `http://100.68.7.42:8787`.
- Direct mode is guarded by backend config validation:
  - `direct_tailscale_enabled=true`
  - `api_token` configured
  - `allow_delete=false`
  - bind host must be loopback or a Tailscale IPv4 range address

### Storage Layout

- SQLite database lives under `<data_dir>/homeops.db`.
- Job file logs live under `<logs_dir>/jobs`.
- File operations are restricted to `<workspace_root>`.
- The current server runtime layout cleanly separates source/binary, workspace, data, and logs.

## Security Audit

### API Token Protection

- `/api/*` routes require `Authorization: Bearer <token>` when `api_token` is configured.
- `/health` remains unauthenticated for local/service health checks.
- Token values are not returned by `/api/settings`; the UI receives only `api_token_configured`.
- Token values should not be printed in logs or committed.

Assessment: good for a private single-user MVP. This is not a full user/session auth system.

### Route Protection

Protected routes include:

- Settings and workspace endpoints.
- File manager endpoints.
- Upload/download endpoints.
- Archive extraction endpoint.
- Jobs/logs endpoints.
- Resources endpoint.

### `/health` Unauthenticated

`/health` is unauthenticated. This is acceptable for the current private Tailscale deployment because it returns only a minimal health shape:

```json
{ "ok": true, "service": "server-agent" }
```

Before public exposure, either keep it minimal and rate-limited behind a trusted network boundary or make health protection configurable.

### Bind Safety

Positive findings:

- `0.0.0.0` is rejected.
- Public IPs are rejected.
- Arbitrary LAN IPs are rejected.
- Direct bind requires Tailscale IPv4 range.
- Direct bind requires token protection.
- Direct bind requires delete disabled.

Residual risk:

- The code checks the bind address is in Tailscale's CGNAT range, but it does not independently verify the address is actually assigned to the local `tailscale0` interface. The deployment procedure verified this manually.

Recommendation:

- Add a startup interface verification in a later hardening phase, or keep the deployment check documented and manual.

### Firewall

Current production-style expectation:

- UFW allows `8787/tcp` only on `tailscale0`.
- No global 8787 rule should exist.
- No firewall changes were needed for public/LAN exposure.

Assessment: appropriate for direct Tailscale MVP.

### CORS And CSP

- Backend CORS is narrow:
  - `http://127.0.0.1:5173`
  - `http://localhost:5173`
  - `http://tauri.localhost`
- Allowed methods are limited to `GET`, `POST`, `PUT`, `OPTIONS`.
- Allowed request headers are limited to `Authorization`, `Content-Type`.
- Tauri CSP allows direct connection to:
  - `http://127.0.0.1:8787`
  - `http://localhost:8787`
  - `http://100.68.7.42:8787`
  - Vite dev origins and HMR WebSockets.

Assessment: good for desktop/Tauri and browser development. Do not broaden without a dedicated exposure phase.

## File Safety Audit

### Workspace Boundary

All file operations reviewed are intended to resolve paths through the backend path safety helper and stay inside `workspace_root`.

Covered operations:

- List
- Create folder
- Rename
- Move
- Download
- Upload
- Delete guard

### Path Rejection

Positive findings:

- `../` traversal is rejected.
- Absolute external paths are rejected.
- Windows drive paths such as `C:\Windows` and `C:/Windows` are rejected.
- Backslash paths are rejected.
- Missing child paths inside the workspace can be accepted where appropriate.
- Symlinks resolving outside the workspace are rejected where practical.

### Upload Safety

Positive findings:

- Upload destination must be a safe existing workspace directory.
- Upload filenames reject empty names, `.`/`..`, path separators, control characters, and overlong names.
- Upload uses non-overwrite behavior.
- Partial upload cleanup is attempted where practical.
- Max upload size is hardcoded for this MVP and documented for later config migration.

### Move/Rename Behavior

Positive findings:

- Rename/move reject workspace root.
- Overwrite is rejected.
- Moving into an existing directory is supported by appending the original item name.
- Backend remains the source of truth for safety.

### Delete Behavior

- Delete is disabled by default through `allow_delete=false`.
- The endpoint exists as a guard but does not perform deletion in the current production MVP.
- UI hides/disables delete when deletion is not enabled.

Assessment: appropriate. Keep deletion disabled until a trash/quarantine design exists.

## Archive Safety Audit

### Supported Formats

- ZIP only.
- No `.7z` or `.rar` support.
- No shelling out to `7z`.
- No arbitrary command execution.

### Extraction Model

- Extraction runs as a background job.
- HTTP request creates an `archive_extract` job and returns quickly.
- Job logs record extraction steps and failures.

### ZIP Entry Safety

Positive findings:

- Traversal entries are blocked.
- Absolute Unix paths are blocked.
- Windows drive paths are blocked.
- Symlink-like entries are rejected where detectable.
- Overwrite is rejected by default.
- Entry count and total extracted byte limits exist.
- Invalid/corrupt ZIP files fail with a clearer user-facing error.

Residual risk:

- Partial extraction can leave files already extracted before a later unsafe/corrupt entry fails.

Recommendation:

- For a later hardening phase, extract into a temporary staging directory and atomically promote only after the archive passes validation.

## Jobs And Logs Audit

### Job Types

Allowed internal job types:

- `test_sleep`
- `test_fail`
- `archive_extract`

No arbitrary job submission exists. No shell commands or external process runner exists.

### Lifecycle

Job statuses:

- `queued`
- `running`
- `finished`
- `failed`
- `cancelled`

Positive findings:

- Jobs are inserted as queued.
- Jobs transition conditionally from queued to running, fixing the queued-cancel race.
- Job progress is stored.
- Job logs are stored in SQLite.
- Job file logs are written under the configured logs directory.
- Operation logs record important events.

### Cancellation

- Queued cancellation is implemented honestly.
- Running cancellation returns a clear not-implemented error.

Assessment: acceptable. Running cancellation can wait until long-running production jobs require cooperative cancellation.

## Resources Page Audit

The Resources page is read-only and suitable for MVP task-manager visibility.

Positive findings:

- Shows real CPU, memory, swap, uptime/load, disk, workspace disk, and process data.
- Process list is limited to top 100.
- `server-agent` is kept visible in the process list.
- Process search/filter/sort works in the UI.
- No process kill button exists.
- No service restart/start/stop controls exist.

Limitations:

- Data is point-in-time polling, not streaming.
- Process ownership/command details can be platform-dependent.
- No per-process actions exist by design.

## UI Coverage Audit

### Dashboard

- Uses real jobs/logs where backend data exists.
- Resource cards should use real resource data or remain clearly marked when placeholder.
- No fake operational jobs/logs should remain.

### Files

Functional:

- List workspace contents.
- Breadcrumb navigation.
- Create folder.
- Upload.
- Download with visible success feedback.
- Move into existing folder.
- Rename.
- ZIP extract action.

Hidden/disabled:

- Delete, because `allow_delete=false`.

### Archives

- Secondary archive view exists.
- Downloads show feedback.
- ZIP extraction is primarily available through Files; Archives may remain a lightweight view.

### Jobs

- Real job list.
- Real job logs.
- Diagnostic test jobs are labeled as diagnostics/dev tooling.
- Cancel is available only for queued jobs.

### Logs

- Shows real operation logs.
- No fake operational log data should remain.

### Resources

- Real read-only resource/task-manager data.
- No process or service control.

### Settings

- Server URL save/test/reset works.
- API token save/clear works.
- Backend config values are displayed.
- Config/startup-controlled settings are read-only.
- Direct Tailscale status is displayed as config/restart controlled.

### Placeholder Pages

Acceptable placeholders:

- Projects
- Services
- Apps module cards
- AI Redux Maker placeholder
- Other future modules

Requirement for placeholders:

- They should not imply the feature is complete or ready when no backend exists.

## Deployment Audit

### Systemd

Expected service:

- `homeops-agent.service`
- Runs as `marcel:marcel`
- Working directory: `/srv/homeops/agent`
- Uses `HOMEOPS_CONFIG=/srv/homeops/data/homeops_config.json`
- Restarts on failure
- Uses journal plus app logs

Assessment: appropriate for MVP.

### Update Process

Current update process is manual:

1. Copy backend source to `/srv/homeops/agent/src`.
2. Build release binary on Ubuntu.
3. Stop service.
4. Copy binary to `/srv/homeops/agent/bin/server-agent`.
5. Restart service.
6. Verify listener and endpoints.

Recommended improvement:

- Add a scripted deployment/update workflow in P7.

### Rollback Process

Current rollback is manual:

- Restore previous binary backup if available.
- Or set config back to tunnel-only:

```json
{
  "bind_host": "127.0.0.1",
  "direct_tailscale_enabled": false
}
```

Then:

```bash
sudo systemctl restart homeops-agent.service
ss -ltnp '( sport = :8787 )'
```

Expected rollback listener:

```text
127.0.0.1:8787
```

## Direct Tailscale Mode Audit

Direct URL:

```text
http://100.68.7.42:8787
```

Safety requirements:

- Tailscale is running.
- API token is configured.
- `direct_tailscale_enabled=true`.
- `allow_delete=false`.
- Listener is `100.68.7.42:8787`, not `0.0.0.0:8787`.
- UFW allows 8787 only on `tailscale0`.

Troubleshooting:

- If direct access fails, verify Tailscale connectivity first.
- If ProtonVPN is active on the PC, it may interfere with Tailscale routing depending on VPN settings. Temporarily test with VPN disabled or allow LAN/Tailscale traffic in the VPN client.
- Confirm server listener:

```bash
ssh homeops "ss -ltnp '( sport = :8787 )'"
```

- Confirm Tailscale IP:

```bash
ssh homeops "tailscale ip -4"
```

Tunnel mode remains available by forwarding local port 8787 to the server's active listener.

## Critical Issues

No confirmed Critical issues found.

Critical classes reviewed and not observed:

- Workspace escape.
- Unsafe delete.
- Public bind.
- Arbitrary command execution.
- Dangerous Tauri filesystem/shell/process plugins.
- Token leak through API settings.

## High Issues

### HIGH-001: No full auth/session model

- **Severity:** High for shared/untrusted use; acceptable for single-user Tailscale MVP.
- **Area:** Security/auth.
- **Problem:** API token auth is a single shared bearer secret with no users, sessions, rotation UI, or scoped permissions.
- **Why it matters:** If the token leaks, all protected MVP APIs are accessible over Tailscale.
- **How to fix:** Add token rotation, token hashing-at-rest or external secret handling, audit logging for auth failures, and eventually user/session auth if more users are added.
- **Must fix before:** LAN/shared-user/public exposure.

### HIGH-002: Direct bind validates Tailscale range, not assigned interface

- **Severity:** High before broader deployment automation; Medium for current manually verified server.
- **Area:** Bind safety/deployment.
- **Problem:** Config validation allows any IPv4 in Tailscale CGNAT range when guarded, but does not verify the IP is assigned to the server's `tailscale0`.
- **Why it matters:** Misconfiguration could cause confusing startup behavior or bind to an unintended local address if present.
- **How to fix:** At startup, enumerate local interfaces or use a safe network crate to confirm `bind_host` is assigned to a Tailscale interface.
- **Must fix before:** Automated direct-mode setup wizard.

## Medium Issues

### MED-001: Manual deployment workflow

- **Severity:** Medium.
- **Area:** Deployment.
- **Problem:** Server updates are manual copy/build/restart steps.
- **Why it matters:** Manual deploys increase the chance of version drift, missed backups, or inconsistent verification.
- **How to fix:** Add P7 deployment/update script with preflight checks, binary backup, build, install, restart, health/auth/listener verification, and rollback.
- **Can wait until:** Before frequent updates or packaging.

### MED-002: Archive extraction partial failure cleanup

- **Severity:** Medium.
- **Area:** Archive safety/reliability.
- **Problem:** A failing archive job can leave already extracted files behind.
- **Why it matters:** Failed jobs may clutter workspace or create confusing partial output.
- **How to fix:** Validate archive entries first where possible or extract to a staging directory and promote on success.
- **Can wait until:** Before larger archive/import workflows.

### MED-003: No WebSockets or push updates

- **Severity:** Medium.
- **Area:** UX/reliability.
- **Problem:** Jobs/resources/logs rely on polling.
- **Why it matters:** Polling is fine for MVP but less efficient and less immediate.
- **How to fix:** Add authenticated WebSocket or server-sent events later.
- **Can wait until:** After core deployment/backup workflows.

### MED-004: No native save dialog

- **Severity:** Medium.
- **Area:** Desktop UX.
- **Problem:** Downloads go to the default downloads folder via browser-style blob download.
- **Why it matters:** Users cannot choose a save location from the app.
- **How to fix:** Add a carefully scoped Tauri filesystem/save-dialog capability later, not broad filesystem access.
- **Can wait until:** Packaging/polish phase.

## Low Issues

### LOW-001: Placeholder modules remain

- **Severity:** Low.
- **Area:** UI.
- **Problem:** Projects, Services, and Apps modules are placeholders.
- **Why it matters:** Not harmful, but users need clear labeling.
- **How to fix:** Keep placeholder labels honest until each backend exists.

### LOW-002: Hardcoded upload limits

- **Severity:** Low.
- **Area:** Config.
- **Problem:** Upload overwrite and size policy are hardcoded for the MVP.
- **Why it matters:** Future installs may need different limits.
- **How to fix:** Move upload policy into startup config with read-only display in Settings.

### LOW-003: No packaged `.exe` release

- **Severity:** Low.
- **Area:** Distribution.
- **Problem:** Tauri dev/build works, but no final release artifact workflow exists.
- **Why it matters:** Running from source is fine for development, but not a polished desktop release.
- **How to fix:** Add P9 production packaging/signing/release workflow.

## Positive Findings

- Strong separation between desktop UI and server-agent.
- No full server UI deployment to Ubuntu.
- API token protects all `/api/*` routes.
- Token value is redacted from API responses.
- Direct Tailscale bind is guarded by config checks.
- UFW rule is scoped to `tailscale0`.
- File operations consistently use workspace path safety.
- Delete remains disabled.
- ZIP extraction is job-based and native Rust, not shell-based.
- Job cancellation race was fixed.
- Resources page is read-only.
- Tauri permissions are minimal.
- CSP/CORS are narrow and intentional.

## Known Limitations

- No Redux Maker implementation yet.
- No scanner integration yet.
- No service control.
- No process control.
- No shell commands.
- No WebSockets.
- No delete support.
- No `.7z` or `.rar` support.
- No native save dialog.
- No packaged `.exe` release yet.
- No server setup wizard yet.
- No automated deployment script yet.
- No multi-user auth/roles.
- No token rotation UI.
- No automated backup/restore workflow for HomeOps state.

## Must-Fix Before Public Exposure

- Add full authentication/session model or a stronger deployment gateway.
- Keep or replace bearer token with a rotated, hashed/managed secret model.
- Add rate limiting and request size hardening.
- Add HTTPS/TLS termination if not fully inside a trusted mesh.
- Add CSRF/origin review if browser access beyond local dev is ever supported.
- Verify bind interface at startup, not only IP range.
- Review CORS for the exact public deployment model.
- Add audit logs for auth failures and sensitive operations.
- Keep delete disabled or implement safe trash/quarantine.
- Add a threat model for uploads and archive extraction.

## Must-Fix Before Redux Maker

- Add HomeOps state backup for database/config/logs.
- Add deployment/update script so backend changes are repeatable.
- Add job categories and clearer long-running job cancellation semantics.
- Add workspace project/import conventions.
- Add archive staging cleanup for failed imports.
- Add resource limits for AI/Redux processing jobs.
- Add explicit module workspace paths and quotas.
- Add structured reports storage under the workspace.
- Add scanner integration only through approved job types, not shell endpoints.

## Recommended Next Phases

1. **P7 Deployment/update workflow**
   - Scripted server-agent deploy with preflight, build, backup, restart, endpoint checks, listener checks, and rollback.

2. **P8 HomeOps state backup**
   - Backup `/srv/homeops/data`, config, SQLite DB, and critical logs. Keep workspace backups separate and explicit.

3. **P9 Tauri production packaging**
   - Build signed/repeatable desktop artifacts, review CSP in packaged mode, and document install/update steps.

4. **Redux Scanner integration later**
   - Add only as approved background jobs with workspace-scoped inputs and logs.

5. **AI Redux Maker module later**
   - Build on the existing job/log/report foundation.

## Final Recommendation

Keep using HomeOps in **direct Tailscale production mode** for the private home server MVP:

```text
http://100.68.7.42:8787
```

Maintain these invariants:

- API token configured.
- `allow_delete=false`.
- Listener is `100.68.7.42:8787`, never `0.0.0.0:8787`.
- UFW allows 8787 only on `tailscale0`.
- No shell/process/service-control features until separately designed and reviewed.

The next best phase is **P7 deployment/update workflow**, because the largest remaining operational risk is manual server updates rather than missing MVP functionality.
