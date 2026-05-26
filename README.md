# HomeOps Panel

HomeOps Panel is a private desktop control panel for a home server. The main UI runs on the PC as a SvelteKit/Tauri app. The Ubuntu server runs only the Rust `server-agent`, which is constrained to a safe workspace at `/srv/homeops/workspace`.

The backend must stay bound to `127.0.0.1` and should be reached through an SSH tunnel for the current deployment. API token protection is available for future Tailscale/LAN work, but LAN/public exposure is still intentionally not enabled.

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
- Rust Axum `server-agent` with local-only bind.
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
  - guarded delete endpoint disabled by default
- Safe multipart upload with overwrite rejection.
- In-process job runner for approved internal jobs only.
- Job logs in SQLite and append-only job log files.
- ZIP-only archive extraction as background jobs with traversal/overwrite/limit checks.
- Read-only Resources page with CPU, memory, disks, workspace, and process snapshot data.
- Ubuntu manual deployment has been verified with SSH tunneling.

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
ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
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
HOMEOPS_TUNNEL_REMOTE_HOST=127.0.0.1
HOMEOPS_TUNNEL_REMOTE_PORT=8787
```

`dev:local` starts `cargo run -p server-agent` from the repo root instead of an SSH tunnel. If port `8787` is already in use, the launcher prints a clear message and does not spawn a duplicate backend/tunnel.

Stop the launcher with `Ctrl+C`. The launcher attempts to cleanly stop child processes it started.

Development ports:

```text
5173  Svelte/Vite frontend
8787  backend or SSH tunnel
```

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

For the real Ubuntu server, keep using an SSH tunnel:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
```

Then keep the HomeOps Panel server URL set to:

```text
http://127.0.0.1:8787
```

Do not expose port `8787` to LAN/public in the current deployment. API token auth exists, but direct LAN/Tailscale binding should be handled in a later explicit phase with auth, CORS, and deployment settings reviewed together. CORS is currently for local development origins only.

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

## Ubuntu Deployment State

The server-agent has been manually deployed and verified on Ubuntu using:

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

The service should run as `marcel`, use `HOMEOPS_CONFIG=/srv/homeops/data/homeops_config.json`, and keep `bind_host` set to `127.0.0.1`.

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
