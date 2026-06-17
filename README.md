# HomeOps Panel

HomeOps Panel is a private desktop control panel for a home server. The main UI runs on the PC as a SvelteKit/Tauri app. The Ubuntu server runs only the Rust `server-agent`, which is constrained to a safe workspace at `/srv/homeops/workspace`.

The backend must stay bound to either `127.0.0.1` for tunnel mode or the approved Tailscale IP for direct mode. API token protection is required for direct Tailscale use, and LAN/public exposure is still intentionally not enabled.

## Structure

```text
HomeOpsPanel/
├── apps/
│   └── web/                 # SvelteKit + TypeScript + Tailwind + Tauri shell
├── apps-desktop-later/      # reserved
├── services/
│   └── server-agent/        # Rust Axum backend
├── packages/
│   ├── ui/                  # future shared Svelte UI package
│   └── types/               # shared type placeholder
├── docs/
└── README.md
```

## Current Capabilities

- Tauri desktop wrapper for the HomeOps Panel UI.
- Server URL settings stored locally in the UI.
- Rust Axum `server-agent` with loopback bind for local/tunnel mode or Tailscale-only bind for direct mode.
- Optional API token protection for every `/api/*` route.
- Config loading through `HOMEOPS_CONFIG`, Windows dev fallback, and Linux `/srv/homeops/data/homeops_config.json`.
- SQLite database with settings, modules, jobs, job logs, and operation logs.
- Workspace status endpoint and path safety helper.
- Safe workspace file manager:
  - list files
  - create folders
  - rename
  - move
  - download
  - guarded delete, disabled by default, moving items to `.homeops-trash` when enabled
  - configurable storage root selector for the main workspace plus approved additional roots
- Safe multipart upload with overwrite rejection, frontend progress, action locking, and atomic temp-file finalization.
- In-process job runner for approved internal jobs only.
- Job logs in SQLite and append-only job log files.
- ZIP-only archive extraction as background jobs with traversal/overwrite/configured limit checks.
- Read-only Resources page with CPU, memory, disks, workspace, and process snapshot data.
- Real Dashboard cards for resources, jobs, operation logs, and configured storage roots.
- Ubuntu deployment has been verified with direct Tailscale mode.

## T0.7 Storage Roots And Core Actions

HomeOps still restricts all file operations to configured, approved storage roots. The default root is always the main workspace:

```text
/srv/homeops/workspace
```

Additional disks are configured in `/srv/homeops/data/homeops_config.json` through `storage_roots`. Do not guess mount paths; add only paths that are intentionally dedicated to HomeOps data.

Example:

```json
{
  "workspace_root": "/srv/homeops/workspace",
  "storage_roots": [
    {
      "id": "bulk",
      "label": "Bulk storage",
      "path": "/mnt/homeops-bulk"
    }
  ]
}
```

The backend automatically keeps the main workspace root available even when additional roots are configured. The Files and Archives pages include a root selector. Listing, upload, download, move, rename, delete-to-trash, and ZIP extraction use the selected root. Archive extraction jobs log the selected root id.

Delete behavior remains conservative:

- `allow_delete=false` keeps delete disabled in the UI and API.
- `allow_delete=true` moves files or folders to `.homeops-trash` inside the selected storage root.
- HomeOps internal paths `.homeops-tmp` and `.homeops-trash` are hidden from root listing and blocked from file/archive operations.
- Permanent delete is not implemented.

The Dashboard now uses real backend data for resource summaries, recent jobs, recent operation logs, and storage-root usage. It does not show fake operational jobs/logs.

Not included in T0.7:

- scanner integration
- AI Redux Maker implementation
- WebSockets
- service/process control
- process kill
- arbitrary shell execution
- dangerous Tauri filesystem/shell/process plugins

## T0.8 Live Storage Activation

The Ubuntu server has two live storage locations configured for HomeOps:

```text
main  Main workspace  /srv/homeops/workspace
bulk  Bulk storage    /mnt/storage/homeops-workspace
```

The bulk root is backed by the mounted 3.6 TB disk:

```text
/dev/sda1  ext4  LABEL=STORAGE  mounted at /mnt/storage
```

The dedicated HomeOps folder on that disk is:

```text
/mnt/storage/homeops-workspace
```

Safe read-only disk inspection commands:

```bash
lsblk -f
df -h
findmnt
sudo blkid
```

Do not format, repartition, or wipe disks from HomeOps setup steps. If a large disk is not mounted, stop and inspect it manually before creating any mount configuration.

Live config path:

```text
/srv/homeops/data/homeops_config.json
```

Storage root config shape:

```json
{
  "storage_roots": [
    {
      "id": "main",
      "label": "Main workspace",
      "path": "/srv/homeops/workspace"
    },
    {
      "id": "bulk",
      "label": "Bulk storage",
      "path": "/mnt/storage/homeops-workspace"
    }
  ]
}
```

After changing storage roots:

```bash
sudo systemctl restart homeops-agent.service
sudo systemctl status homeops-agent.service --no-pager
curl http://100.68.7.42:8787/health
```

Authenticated workspace check:

```bash
TOKEN="$(cat /srv/homeops/data/homeops_api_token.txt)"
curl -H "Authorization: Bearer $TOKEN" http://100.68.7.42:8787/api/workspace
```

Expected: two storage roots, both writable.

The selected storage root affects:

- file listing
- folder creation
- upload
- download
- rename
- move within the same root
- ZIP extraction
- delete-to-trash when delete is enabled

Cross-root move is not supported. Move operations stay inside the currently selected root.

Delete-to-trash status:

- Backend implementation moves deleted items to `.homeops-trash` inside the selected root.
- `.homeops-trash` and `.homeops-tmp` are hidden from normal listings and blocked from direct file/archive actions.
- Production direct Tailscale mode currently keeps `allow_delete=false`.
- This is intentional: server-agent safety checks reject direct Tailscale bind when `allow_delete=true`.
- Permanent delete is not implemented.

Scanner integration and AI Redux Maker remain out of scope.

## T0.9 Projects / Workspaces

Projects are generic HomeOps workspaces. A project is metadata in SQLite that points to a safe folder inside one configured storage root. A project is not a scanner project and not an AI Redux Maker project.

Example:

```text
Name: Redux Circle
Root: bulk
Path: projects/redux-circle
Status: active
Tags: modding, redux
Notes: Working copy of circle.zip
```

Project rules:

- Project paths are relative paths inside a configured storage root.
- Absolute paths, `../` traversal, Windows/backslash paths, symlink escapes, `.homeops-tmp`, and `.homeops-trash` are rejected.
- A project points to a folder, not a file.
- Creating a project can create a new folder under the selected root.
- Attaching a project requires the folder to already exist.
- The storage root itself cannot be used as a project path; use a subfolder.
- Deleting project metadata does not delete files or folders.
- Archiving/unarchiving a project changes metadata only.

Project APIs:

```text
GET    /api/projects
POST   /api/projects
GET    /api/projects/:id
PATCH  /api/projects/:id
DELETE /api/projects/:id
```

Open in Files:

1. Open `Projects`.
2. Select `Open in Files` on a project.
3. HomeOps switches to `Files`, selects the project storage root, and opens the project folder.

Upload and ZIP extraction use existing Files behavior after opening the project folder. Project cards intentionally do not duplicate file-operation logic.

Manual Projects checklist:

1. Create project on main root.
2. Create project on bulk root.
3. Attach existing folder as project.
4. Reject traversal path.
5. Reject Windows/backslash path.
6. Reject absolute external path.
7. Reject `.homeops-tmp` / `.homeops-trash` path.
8. Open project in Files and verify root/path changes.
9. Upload a file into project folder.
10. Extract a ZIP into project folder.
11. Archive/unarchive project metadata.
12. Delete project metadata and verify files remain.
13. Confirm Projects page has no fake buttons.
14. Confirm Apps/Services remain hidden.
15. Confirm scanner/AI UI is not added.

## H1.0 Smart Storage Pool

HomeOps shows the two server drives as one logical **Smart Pool** while keeping
the roots physically separate. **No disks are merged or reformatted.** The
backend chooses the best root automatically for new files and workflows.

Placement policy (backend-authoritative):

- Large files (≥ 2 GiB) → `bulk`.
- Archives (`.zip .oiv .rpf .7z .rar`) → `bulk`.
- Redux corpus (`redux-corpus/*`) → `bulk` (forced; fails clearly if bulk full).
- Metadata / small files / reports → `main` (default).
- Reserves kept free: `main` 25 GiB, `bulk` 100 GiB.

Endpoints (token-auth): `GET /api/storage/pools`,
`GET /api/storage/pools/server`,
`POST /api/storage/pools/server/resolve-placement`,
`POST /api/storage/pools/server/bootstrap-standard-folders`.

UI: a **/storage** page (pool card, policy, root cards, placement preview,
corpus bootstrap), a Resources link, and a **Smart Pool** option in the Files
root selector that routes uploads automatically. The Redux corpus will use the
`bulk` root automatically (see `docs/H1_0_SMART_STORAGE_POOL.md`). Next:
**T2.2 — HomeOps Redux Corpus Job Integration**.

⚠️ No physical disk merge/reformat is performed; deletes stay disabled and path
safety is unchanged.

## T2.2 HomeOps Redux Corpus Job Integration

Drop many Redux/mod packages into a server inbox and run a fully automatic,
**read-only** dataset build from HomeOps. Wires only the ReduxScannerEngine T2.1
headless batch scanner — no Redux Maker generation, CodeWalker, RPF apply, or AI.

Corpus folder layout (Smart Pool **bulk** root, derived — never user-set):

```
/mnt/storage/homeops-workspace/redux-corpus/
  inbox  input  work  out  datasets  reports  quarantine
```

Workflow: upload packages into `redux-corpus/inbox` (Files page) → open
**/redux-corpus** → Bootstrap folders → Scan Corpus / Build Dataset → watch the
background job on the Jobs page → datasets/reports/coverage/quarantine appear
under `bulk/redux-corpus`. Source packages are never modified.

Endpoints (token-auth): `GET /api/redux-corpus/status`,
`POST /api/redux-corpus/bootstrap`, `POST /api/redux-corpus/scan`,
`GET /api/redux-corpus/reports/latest`, `GET /api/redux-corpus/dataset/summary`,
`GET /api/redux-corpus/quarantine`.

Scanner deployment (fixed, admin-controlled — never an arbitrary UI path):
`/opt/homeops-tools/redux-scanner/redux-scanner`, run with an args array and the
fixed `scan-redux-corpus-batch` subcommand plus `--metadata-only`. One active
scan at a time; stdout/stderr drained to job logs. See
`docs/T2_2_HOMEOPS_REDUX_CORPUS_JOB_INTEGRATION.md`. Next:
**T2.3 — Corpus Coverage Matrix + Dataset Browser inside HomeOps**.

⚠️ Live RPF apply remains in the local desktop Redux Maker, never on the HomeOps
server. Read-only metadata scanning only; no source mutation; API-token and
Tailscale/UFW safety unchanged.

## H2.0 Responsive Shell + Redux Maker Workspace + Themes

HomeOps now feels like one unified desktop app that works in non-maximized
windows.

- **Redux Maker workspace** (`/redux-maker`): surfaces server Redux corpus
  context (status/dataset/report) and links to the local desktop Redux Maker
  app. No server-side generate/apply — AI planning, CodeWalker, copied-RPF
  apply, and rollback stay in the local app until **H2.1**.
- **Responsive shell**: full layout ≥ 1100px; narrowed sidebar ≤ 1100px;
  compact **icon rail** ≤ 880px; top-strip quick-nav collapses ≤ 760px. Tables
  use fixed layout + scroll wrappers; card pages reflow to one column.
- **Title/top bar**: OS decorations kept (window controls never broken); the
  in-app `global-topbar` carries the HomeOps-styled brand, endpoint, quick-nav,
  and Tailscale status chip.
- **Settings → Themes**: registry at `apps/web/src/lib/theme/themes.ts`;
  **HomeOps Command Dark** is the only selectable theme (others are disabled
  "soon" placeholders), persisted in `localStorage` and applied via
  `data-theme` with no load flash.

See `docs/H2_0_RESPONSIVE_REDUX_MAKER_WORKSPACE.md`. Full RPF apply remains in
the local Redux Maker until **H2.1 — Local Redux Maker Bridge for HomeOps
Desktop**.

## H2.0.1 Full Redux Maker Studio in HomeOps

`/redux-maker` now renders a real embedded **Redux Maker Studio** layout
(matching the standalone app) instead of a generic dashboard:

- Top status ribbon (local bridge / corpus server / safety), left **Redux
  Blueprint** tree, center **review/diff** workspace, bottom **AI Patch Prompt**
  composer, right **telemetry / system log / action dock**
  (`apps/web/src/lib/components/redux-maker/`).
- Honest empty state: no run, no report, no module plan, no generated assets.
  "Generate Module Plan" and "Review & Apply Plan" are disabled with clear
  reasons (local bridge not connected). Corpus context is read live from the
  read-only HomeOps API.
- Responsive: three panes ≥ 1200px, dock drops below center 900–1200px, full
  stack ≤ 880px; no catastrophic overflow.

Local apply / generation still require the standalone Redux Maker app until
**H2.1**; the HomeOps server never edits RPF. See
`docs/H2_0_1_FULL_REDUX_MAKER_STUDIO_IN_HOMEOPS.md`.

## Backend

Run locally on the PC:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
cargo run -p server-agent
```

The backend binds to:

```text
127.0.0.1:8787
```

Useful checks:

```powershell
curl http://127.0.0.1:8787/health
curl http://127.0.0.1:8787/api/settings
curl http://127.0.0.1:8787/api/workspace
curl http://127.0.0.1:8787/api/files/list
curl http://127.0.0.1:8787/api/jobs
curl http://127.0.0.1:8787/api/logs/operations
```

When testing the Ubuntu server from the PC in current direct Tailscale mode, use:

```powershell
curl http://100.68.7.42:8787/health
```

Tunnel mode is still available, but because the server now listens on `100.68.7.42:8787`, forward to the Tailscale listener:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:100.68.7.42:8787 homeops
```

If you use the older tunnel target `127.0.0.1:8787`, SSH will print `channel ... open failed: connect failed: Connection refused` because the remote service is no longer bound to remote loopback.

If `api_token` is configured, `/health` remains open but `/api/*` checks require:

```powershell
curl -H "Authorization: Bearer <token>" http://127.0.0.1:8787/api/settings
```

## Root Development Launcher

From the repo root, one command starts the normal desktop development stack:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
npm run dev
```

Default mode is tunnel mode. It expects SSH alias `homeops` to reach the Ubuntu server and starts this tunnel if local port `8787` is free:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:100.68.7.42:8787 homeops
```

Then it runs the existing Tauri dev script in `apps\web`, which starts Vite on `127.0.0.1:5173` and opens the HomeOps Panel desktop window.

Explicit modes:

```powershell
npm run dev:tunnel
npm run dev:local
```

`dev:tunnel` uses these optional environment variables:

```text
HOMEOPS_DEV_MODE=tunnel
HOMEOPS_SSH_HOST=homeops
HOMEOPS_TUNNEL_LOCAL_PORT=8787
HOMEOPS_TUNNEL_REMOTE_HOST=100.68.7.42
HOMEOPS_TUNNEL_REMOTE_PORT=8787
```

`dev:local` starts `cargo run -p server-agent` from the repo root instead of an SSH tunnel. If port `8787` is already in use, the launcher prints a clear message and does not spawn a duplicate backend/tunnel.

Stop the launcher with `Ctrl+C`. The launcher attempts to cleanly stop child processes it started.

Development ports:

```text
5173  Svelte/Vite frontend
8787  backend or SSH tunnel
```

The desktop shell uses a restrictive Tauri Content Security Policy. It allows local app assets, the local Vite dev server, and the local backend/tunnel origins only:

```text
http://127.0.0.1:5173
http://localhost:5173
http://127.0.0.1:8787
http://localhost:8787
http://100.68.7.42:8787
```

Vite WebSocket origins on port `5173` are allowed for hot reload during development. Broad origins such as `*`, `http:`, and `https:` are intentionally not allowed.

## Frontend

Run the browser UI:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\apps\web
npm install
npm run dev -- --host 127.0.0.1
```

Open:

```text
http://127.0.0.1:5173
```

The app primarily calls the configured server URL, defaulting to:

```text
http://127.0.0.1:8787
```

## Desktop App

Run the Tauri desktop shell:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\apps\web
npm run tauri:dev
```

Build the desktop app:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\apps\web
npm run tauri:build
```

The Tauri wrapper currently keeps minimal permissions. It does not add filesystem, shell, process, updater, notification, or native server-control plugins.

## Server Connection

Tunnel mode connects through SSH and keeps the backend reachable from the PC at:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
```

Then keep the HomeOps Panel server URL set to:

```text
http://127.0.0.1:8787
```

When the server-agent is in direct Tailscale mode and listens on `100.68.7.42:8787`, tunnel mode can still be used by forwarding the local port to the server's Tailscale listener:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:100.68.7.42:8787 homeops
```

For the root launcher in that mode, set:

```powershell
$env:HOMEOPS_TUNNEL_REMOTE_HOST='100.68.7.42'
npm run dev:tunnel
```

Direct Tailscale mode connects without an SSH tunnel:

```text
http://100.68.7.42:8787
```

Direct mode is intentionally narrow. The server-agent may bind to a Tailscale IPv4 address only when `direct_tailscale_enabled=true`, `api_token` is configured, and `allow_delete=false`. It must never bind to `0.0.0.0`, public IPs, or arbitrary LAN IPs.

Ubuntu direct-mode config example:

```json
{
  "bind_host": "100.68.7.42",
  "bind_port": 8787,
  "direct_tailscale_enabled": true,
  "allow_delete": false,
  "api_token": "replace-with-a-private-token"
}
```

Rollback to tunnel-only mode by editing `/srv/homeops/data/homeops_config.json`:

```json
{
  "bind_host": "127.0.0.1",
  "direct_tailscale_enabled": false
}
```

Then restart and confirm the listener:

```bash
sudo systemctl restart homeops-agent.service
ss -ltnp '( sport = :8787 )'
```

Expected rollback listener:

```text
127.0.0.1:8787
```

Do not expose port `8787` to public internet or general LAN. CORS is intentionally local/Tauri-only and allows only these origins:

```text
http://127.0.0.1:5173
http://localhost:5173
http://tauri.localhost
```

Allowed CORS methods are limited to `GET`, `POST`, `PUT`, and `OPTIONS`. Allowed request headers are limited to `Authorization` and `Content-Type`; `Content-Disposition` is exposed for authenticated downloads.

## API Token Auth

The server-agent supports an optional startup config field:

```json
{
  "api_token": "replace-with-a-private-token"
}
```

When `api_token` is missing, null, or empty, `/api/*` routes stay unauthenticated for local/tunnel development and the server logs a warning. When `api_token` is set, every `/api/*` endpoint requires:

```text
Authorization: Bearer <token>
```

`/health` remains unauthenticated for local health checks.

On Ubuntu, set the token in:

```text
/srv/homeops/data/homeops_config.json
```

Then restart the service:

```bash
sudo systemctl restart homeops-agent.service
```

Never commit real tokens. The UI stores the token locally in browser/Tauri storage under `homeops.apiToken` and does not send it to `/health`.

## Archive Extraction Limits

Archive extraction supports standard `.zip` files only. The server-agent does not use shell extraction tools, and `.7z` or `.rar` support is intentionally not implemented yet.

Extraction limits are startup/config-controlled and are read-only in the UI for now:

```json
{
  "max_archive_extract_bytes": 8589934592,
  "max_archive_entries": 10000
}
```

Defaults:

```text
max_archive_extract_bytes = 8589934592 bytes, 8 GiB
max_archive_entries = 10000
```

If these fields are missing from an older `/srv/homeops/data/homeops_config.json`, the server-agent starts with the safe defaults above. After changing either value, restart the server-agent.

Some Redux/GTA archives, including ZIPs containing large `update.rpf` files, can exceed the old 2 GiB extracted-size limit. Those failures were size-limit blocks, not corrupt ZIP errors, when the job log says the extracted bytes would exceed the configured limit.

## T0.6 Core Stability Pass

T0.6 tightened the existing HomeOps MVP without adding scanner, AI Redux Maker, WebSockets, delete support, service control, process control, shell execution, or dangerous Tauri plugins.

Implemented stability work:

- Uploads show a persistent app-level progress panel with filename, destination, status, percentage, and byte counts.
- Frontend uploads use `XMLHttpRequest` so upload progress is real rather than a final-only notification.
- Active upload paths are tracked locally. File actions such as extract, move, rename, and download are disabled while that path is uploading.
- Backend uploads write to an internal temporary path first, then atomically rename into the final workspace path after the upload completes.
- Temporary upload parts live under `.homeops-tmp/uploads`, are hidden from normal file listing, and are rejected by normal file/archive APIs.
- Failed uploads remove their temporary part where practical and do not leave a visible final file.
- Files, Archives, Jobs, Logs, and Resources refresh automatically with polling; WebSockets are still intentionally not used.
- Polling preserves current folder, search/filter text, sort state, selected job, and visible page state.
- Dead or future controls remain disabled, hidden, or clearly labeled as planned.

Manual T0.6 checklist:

1. Upload a large ZIP and verify progress appears while the upload is active.
2. While upload is running, verify extract, move, rename, and download are disabled for that path.
3. Verify the final file only appears as a usable file after upload completes.
4. Interrupt/fail an upload if practical and verify no broken final file remains.
5. Verify file and archive lists refresh automatically.
6. Verify jobs, logs, and resources refresh automatically.
7. Verify current folder, search/filter text, sort state, and selected job do not reset during refresh.
8. Verify dead buttons are removed, disabled, or clearly marked planned.
9. Run:

   ```powershell
   cargo test -p server-agent
   cargo build -p server-agent
   cd apps\web
   npm run check
   npm run build
   ```

## CSP/CORS Troubleshooting

If the Tauri app cannot connect after a security change:

1. Confirm the SSH tunnel is running:

   ```powershell
   ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
   ```

2. Confirm the backend is reachable through the tunnel:

   ```powershell
   curl http://127.0.0.1:8787/health
   ```

3. Confirm the API token is saved in `Settings -> API token` when the server reports `api_token_configured=true`.
4. Check the Tauri/WebView console for CSP `connect-src` errors. The expected local backend URL is `http://127.0.0.1:8787`.
5. Check the backend response for CORS errors only when running the browser/Vite dev UI from `127.0.0.1:5173` or `localhost:5173`, or the packaged Tauri app from `http://tauri.localhost`; other origins are intentionally rejected.

## Ubuntu Deployment State

The server-agent is deployed on Ubuntu using:

```text
/srv/homeops/agent/bin/server-agent
/srv/homeops/data/homeops_config.json
/srv/homeops/workspace
/srv/homeops/data
/srv/homeops/logs
```

The systemd service status should be checked on the server with:

```bash
systemctl status homeops-agent.service --no-pager
```

The service should run as `marcel`, use `HOMEOPS_CONFIG=/srv/homeops/data/homeops_config.json`, and bind only to the configured safe address. Current direct Tailscale mode expects:

```text
100.68.7.42:8787
```

Confirm the listener from the PC with:

```powershell
ssh homeops "ss -ltnp '( sport = :8787 )'"
```

Expected direct-mode listener:

```text
100.68.7.42:8787
```

It should not show `0.0.0.0:8787`.

## Server-Agent Deployment Workflow

Run the deployment health check from the repo root:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
.\scripts\check_server_agent.ps1
```

Deploy a new server-agent build from the repo root:

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
.\scripts\deploy_server_agent.ps1
```

The deployment script copies only source needed by Cargo:

```text
Cargo.toml
Cargo.lock
services/server-agent
packages
```

It never copies:

```text
apps/web
node_modules
target
SQLite databases
logs
uploaded or extracted workspace files
local config
tokens or secrets
```

Deployment flow:

1. Check SSH connectivity and remote config safety.
2. Refuse to deploy if `api_token` is missing, `allow_delete` is not `false`, or the bind address is unsafe.
3. Create a temporary source archive on the PC.
4. Upload it to `/tmp` on the server.
5. Extract to `/srv/homeops/agent/src/current`.
6. Build with `cargo build -p server-agent --release`.
7. Back up the old binary to `/srv/homeops/agent/bin/server-agent.backup.<timestamp>`.
8. Stop `homeops-agent.service`.
9. Install the new binary to `/srv/homeops/agent/bin/server-agent`.
10. Start `homeops-agent.service`.
11. Verify service active, listener safety, `/health`, and `AUTH_REQUIRED` for `/api/settings` without a token.

If the build fails, the currently running service is not stopped. If install/start fails after the service is stopped, the script attempts to restore the previous binary backup and restart the service.

Manual rollback, if needed:

```bash
sudo systemctl stop homeops-agent.service
sudo cp /srv/homeops/agent/bin/server-agent.backup.<timestamp> /srv/homeops/agent/bin/server-agent
sudo chmod 755 /srv/homeops/agent/bin/server-agent
sudo systemctl start homeops-agent.service
sudo systemctl status homeops-agent.service --no-pager
```

Direct Tailscale safety assumptions:

- `bind_host` remains `100.68.7.42`.
- `direct_tailscale_enabled=true`.
- `api_token` is configured.
- `allow_delete=false`.
- UFW allows 8787 only on `tailscale0`.
- The listener must never be `0.0.0.0:8787`.

The scripts do not print the API token.

## HomeOps State Backup

HomeOps can create an on-demand state backup from the Settings page. Open `Settings -> HomeOps State Backup` and choose `Create Backup`. The backup runs as a normal backend job, so progress and logs are visible on the Jobs page.

Backups are stored inside the safe workspace at:

```text
/srv/homeops/workspace/backups/homeops-state/
```

Each backup is a timestamped `.zip` archive containing state files when they exist:

```text
homeops.db
homeops_config.json
homeops_api_token.txt
backup_manifest.json
```

The manifest records metadata such as timestamp, hostname/bind mode, `api_token_configured`, and `allow_delete`, but it never includes the token value. The backup archive itself does include sensitive config/token files, so keep it private and do not commit it.

State backups do not include workspace uploads, extracted archives, project files, reports, or other user data under `/srv/homeops/workspace`. Back up those folders separately when needed.

Download a backup from the same Settings card. Downloads use the authenticated file-download path and save to the default downloads folder.

Manual restore outline:

```bash
sudo systemctl stop homeops-agent.service
sudo cp homeops.db /srv/homeops/data/homeops.db
sudo cp homeops_config.json /srv/homeops/data/homeops_config.json
sudo cp homeops_api_token.txt /srv/homeops/data/homeops_api_token.txt
sudo chown marcel:marcel /srv/homeops/data/homeops.db /srv/homeops/data/homeops_config.json /srv/homeops/data/homeops_api_token.txt
sudo chmod 600 /srv/homeops/data/homeops_config.json /srv/homeops/data/homeops_api_token.txt
sudo systemctl start homeops-agent.service
```

Review restored config before starting if you are changing between tunnel and direct Tailscale modes.

## Test Commands

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
cargo check
cargo test -p server-agent
cd apps\web
npm run check
npm run build
npm run tauri:dev
```

## Not Done Yet

- WebSockets.
- AI Redux Maker implementation.
- 7z/rar archive support.
- Service/process control.
- Public/LAN exposure.
- Dynamic runtime updates for startup-controlled settings such as `max_parallel_jobs` and `allow_archive_extract`.
