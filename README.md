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

When testing the Ubuntu server from the PC, start the SSH tunnel in a separate terminal and leave it running before these `curl` checks:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
```

If `curl http://127.0.0.1:8787/health` fails on the PC but `homeops-agent.service` is running on Ubuntu, the SSH tunnel is probably not active.

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
