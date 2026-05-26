# HomeOps Stable Test Checklist

## Tunnel Prerequisite

Before running PC-side `curl` checks against `127.0.0.1:8787`, start the SSH tunnel in a separate terminal and leave it running:

```powershell
ssh -N -o ExitOnForwardFailure=yes -L 8787:127.0.0.1:8787 homeops
```

If `curl http://127.0.0.1:8787/health` fails on the PC but `homeops-agent.service` is active on Ubuntu, the tunnel is probably not active.

## Listener Check

Run:

```powershell
ssh homeops "ss -ltnp '( sport = :8787 )'"
```

Expected:

```text
127.0.0.1:8787
```

Not expected:

```text
0.0.0.0:8787
```

## Auth Checks

With the tunnel running:

```powershell
curl http://127.0.0.1:8787/health
curl http://127.0.0.1:8787/api/settings
```

Expected:
- `/health` returns `200`.
- `/api/settings` without a token returns `AUTH_REQUIRED`.

Use the HomeOps Panel Settings page to store the API token before testing Files, Archives, Jobs, Logs, and Resources through the app.
