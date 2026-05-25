# HomeOps Panel

HomeOps Panel is a private home server control panel. The main UI is developed on this PC and is intended to be wrapped with Tauri later. The Linux server runs the Rust `server-agent` backend and owns the safe workspace at `/srv/homeops/workspace`.

## Structure

```text
HomeOpsPanel/
├── apps/
│   └── web/                 # SvelteKit + TypeScript + Tailwind UI
├── apps-desktop-later/      # future Tauri wrapper placeholder
├── services/
│   └── server-agent/        # Rust Axum backend
├── packages/
│   ├── ui/                  # future shared Svelte UI package
│   └── types/               # shared Rust/type placeholder
├── docs/
└── README.md
```

## Backend

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
cargo run -p server-agent
```

The development backend binds to `127.0.0.1:8787`.

The backend creates a local development config under `%LOCALAPPDATA%\HomeOpsPanel` on Windows unless `HOMEOPS_CONFIG` is set. On Linux/server, the default config path is `/srv/homeops/data/homeops_config.json`.

Health and foundation checks:

```powershell
curl http://127.0.0.1:8787/health
curl http://127.0.0.1:8787/api/settings
curl http://127.0.0.1:8787/api/workspace
curl http://127.0.0.1:8787/api/files/list
```

Expected response:

```json
{ "ok": true, "service": "server-agent" }
```

## Frontend

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\apps\web
npm install
npm run dev -- --host 127.0.0.1
```

The Vite dev server still proxies `/health` to `http://127.0.0.1:8787`, but the app now primarily uses the configured server URL stored in browser localStorage.

## Desktop App

The desktop wrapper lives in:

```text
C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\apps\web\src-tauri
```

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

The desktop app is named `HomeOps Panel` and loads the same SvelteKit UI. The wrapper currently has only minimal default Tauri permissions and does not add native file operations, shell execution, process control, notifications, updater, or server-management commands.

To connect the PC UI to the server backend during development:

```powershell
ssh -N -L 8787:127.0.0.1:8787 homeops
```

Later LAN/Tailscale backend target:

```text
http://100.68.7.42:8787
```

Do not expose the app publicly in the current phase.

Packaged Tauri builds may need an additional CORS origin once the final production origin is known. Auth/token protection is intentionally deferred to a later phase, so this phase remains development-only and should stay behind localhost, SSH forwarding, or a trusted private network.

## Test Commands

```powershell
cd C:\Users\Marcel\Documents\GitHub\HomeOpsPanel
cargo check
cargo test -p server-agent
cd apps\web
npm install
npm run check
npm run build
npm run tauri:dev
```

## Current Limitations

- Dashboard, Files, Archives, Jobs, Logs, Apps, and AI Redux Maker use mock data.
- SQLite, config loading, app/user settings, seeded modules, workspace status, and path-safety tests are implemented as backend foundation only.
- Safe workspace-only file listing, folder creation, rename/move, guarded delete, and attachment downloads are implemented for the configured workspace root.
- No upload, job execution, archive extraction, WebSockets, service management, shell execution, or auth are implemented.
- Server URL settings are stored in localStorage under `homeops.serverUrl`.
- No systemd service is created in this phase.
- The server prototype at `/srv/homeops/app/homeops-panel` is intentionally left untouched.
