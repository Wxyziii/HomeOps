# HomeOps UI Interaction Coverage

This audit records the MVP interaction cleanup after Phase P0.5. Every visible control should either work, be disabled intentionally, show a planned message, be hidden for safety, or be clearly labeled as placeholder.

| Page | Element | Current behavior | Backend available? | Fixed action | Final status |
|---|---|---|---|---|---|
| Dashboard | Connection chip | Uses shared server connection store and `/health`. | Yes | Kept functional and removed fake update wording. | functional |
| Dashboard | Active Jobs panel | Loads newest jobs from `GET /api/jobs`. | Yes | Replaced mock jobs in Phase P0. | functional |
| Dashboard | Recent Logs panel | Loads operation logs from `GET /api/logs/operations`. | Yes | Replaced mock logs in Phase P0. | functional |
| Dashboard | CPU/RAM/storage/network cards | Static visual metrics only. | No resources endpoint yet | Added visible placeholder/resource monitoring later label. | placeholder clearly labeled |
| Dashboard | Disk volumes panel | Static visual disk rows only. | No resources endpoint yet | Added visible placeholder/resource monitoring later label. | placeholder clearly labeled |
| Files | Refresh button | Reloads current folder through `GET /api/files/list`. | Yes | Kept functional. | functional |
| Files | Search input | Filters currently loaded folder rows client-side. | Existing list endpoint only | Wired to local filter. | functional |
| Files | Upload button | Opens browser file picker and posts to `POST /api/files/upload`. | Yes | Kept functional. | functional |
| Files | New folder button | Prompts for folder name and posts to `POST /api/files/create-folder`. | Yes | Kept functional. | functional |
| Files | Breadcrumb buttons | Navigate within the current workspace path. | Yes | Kept functional. | functional |
| Files | Grid view button | No grid view exists yet. | No | Disabled with planned title. | disabled intentionally |
| Files | List view button | Current view is always list. | UI-only | Disabled with active-view title. | disabled intentionally |
| Files | Sort button | Backend already sorts directories first then name. | Yes | Disabled with current-sort title. | disabled intentionally |
| Files | Row open directory | Opens directory rows only. | Yes | Kept functional; file rows are disabled for open. | functional |
| Files | Row download | Downloads files through `GET /api/files/download`. | Yes | Kept functional. | functional |
| Files | Row extract | ZIP files start `POST /api/archives/extract`. | Yes | Kept functional. | functional |
| Files | Row rename | Prompts and posts to `POST /api/files/rename`. | Yes | Kept functional. | functional |
| Files | Row move | Prompts for destination and posts to `POST /api/files/move`. | Yes | Added MVP “Move to...” UI. | functional |
| Files | Row delete | Delete endpoint exists but `allow_delete=false`. | Guard-only endpoint | Hidden unless backend says enabled; disabled when rendered. | hidden for safety |
| Files | Row more/kebab | No overflow menu yet. | No | Disabled with planned title. | disabled intentionally |
| Files | Row checkboxes/select all | Bulk actions do not exist yet. | No | Disabled with planned title. | disabled intentionally |
| Archives | Archive list | Shows real ZIP files from workspace root. | Yes, via files list | Removed fake archive/backup data. | functional |
| Archives | Search input | Filters visible ZIP rows. | Existing list endpoint only | Wired to local filter. | functional |
| Archives | Refresh button | Reloads workspace root ZIP list. | Yes | Added functional refresh. | functional |
| Archives | New archive button | Archive creation/backup system does not exist. | No | Disabled with planned title. | disabled intentionally |
| Archives | Extract action | Starts ZIP extraction job. | Yes | Added Extract action. | functional |
| Archives | Download action | Downloads ZIP file. | Yes | Added Download action. | functional |
| Archives | More action | No archive overflow menu yet. | No | Disabled with planned title. | disabled intentionally |
| Projects | Page body | Project backend/UI is not implemented. | Tables exist, no endpoints | Reworded page as planned workspace module and hid controls. | placeholder clearly labeled |
| Jobs | Refresh button | Reloads jobs and selected job logs. | Yes | Kept functional. | functional |
| Jobs | Diagnostics test job button | Starts approved internal `test_sleep` job. | Yes | Relabeled as diagnostics/dev tool. | functional |
| Jobs | Diagnostics failing job button | Starts approved internal `test_fail` job. | Yes | Relabeled as diagnostics/dev tool. | functional |
| Jobs | Job card | Selects job and loads logs. | Yes | Kept functional. | functional |
| Jobs | Cancel button | Only queued cancellation is implemented. | Partial | Enabled only for queued jobs; disabled otherwise. | disabled intentionally |
| Logs | Operation log list | Loads `GET /api/logs/operations`. | Yes | Kept functional. | functional |
| Logs | Refresh button | Reloads operation logs. | Yes | Kept functional. | functional |
| Logs | Pause/resume button | Pauses/resumes polling. | UI-only | Kept functional. | functional |
| Logs | Search input | Filters loaded operation logs. | Existing operation logs endpoint | Wired to local filter. | functional |
| Logs | Level filters | Filters loaded logs by `error`, `warn`, or `info`. | Existing operation logs endpoint | Wired to local filter. | functional |
| Logs | Jobs filter chip | Cross-job log aggregation is not implemented. | Job-specific logs exist on Jobs page | Marked as disabled/planned. | disabled intentionally |
| Logs | Last 100 select | Backend call is fixed to last 100 for MVP. | Yes | Disabled with explanatory title. | disabled intentionally |
| Resources | Page body | Resource monitoring does not exist yet. | No | Removed any OK implication; listed planned CPU/RAM/disk/network/process view. | placeholder clearly labeled |
| Services | Page body | Service control does not exist and is unsafe before auth/policy. | No | Explicitly marked service control planned; no start/stop/restart controls. | placeholder clearly labeled |
| Apps | AI Redux Maker card | Opens placeholder module dashboard. | No module backend | Status changed from ready to later/planned. | planned message shown |
| Apps | Other module cards | No module pages yet. | No | Clicking shows planned/later message instead of silently staying on Apps. | planned message shown |
| AI Redux Maker | Module metrics | No importer/jobs/reports exist. | No | Values set to 0 and labeled placeholder. | placeholder clearly labeled |
| AI Redux Maker | Latest report/status | No report exists. | No | Changed to none/not implemented/later. | placeholder clearly labeled |
| Settings | Server URL Save | Validates and stores server URL locally. | UI/localStorage | Kept functional. | functional |
| Settings | Test Connection | Calls configured `/health`. | Yes | Kept functional. | functional |
| Settings | Reset to Default | Resets URL to `http://127.0.0.1:8787`. | UI/localStorage | Kept functional. | functional |
| Settings | Backend Refresh | Loads `/api/settings` and `/api/workspace`. | Yes | Kept functional. | functional |
| Settings | Save Backend Settings | Saves safe app/user settings only. | Yes | Kept limited to `app_name`. | functional |
| Settings | `max_parallel_jobs` | Startup-controlled value. | Config only | Rendered read-only as config/restart controlled. | disabled intentionally |
| Settings | `allow_archive_extract` | Startup-controlled value. | Config only | Rendered read-only as config/restart controlled. | disabled intentionally |
| Sidebar | Navigation links | Navigate to visible pages. | UI routes | Kept functional. | functional |
| Sidebar | Jobs/Logs/Resources badges | Previously fake counts/status. | No live sidebar status store | Removed fake badges. | hidden for safety |
| Topbar | Page controls | Per-page controls only. | Varies | Controls are functional, disabled, or labeled per page. | functional |
| Shared components | Disabled buttons | Show native disabled state. | UI-only | Added disabled support to `SmallButton` and `IconButton`. | functional |
| Shared components | SearchInput | Supports bound value and disabled state. | UI-only | Added bindable value for real filtering. | functional |

## Remaining Intentional Placeholders

- Dashboard resource cards and disk volume rows remain static until a resource monitoring endpoint exists.
- Projects remains a planning page.
- Resources remains a planning page.
- Services remains a planning page and intentionally exposes no service controls.
- AI Redux Maker remains a placeholder module dashboard.
- Archive creation/backup scheduling is not implemented.
- Files bulk selection, grid view, and overflow menus are disabled until real actions exist.

## Safety Notes

- Delete remains hidden/disabled because `allow_delete=false`.
- No service control, process kill, shell execution, public binding, WebSockets, auth, dangerous Tauri plugins, or new archive formats were added.
- Backend path safety remains the source of truth for file, upload, download, move, rename, and extraction actions.
