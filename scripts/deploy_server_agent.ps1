param(
    [string]$SshHost = "homeops",
    [string]$ExpectedBindHost = "100.68.7.42",
    [int]$ExpectedPort = 8787,
    [string]$DirectUrl = "http://100.68.7.42:8787"
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Stamp = Get-Date -Format "yyyyMMddHHmmss"
$Archive = Join-Path $env:TEMP "homeops-server-agent-src-$Stamp.tar.gz"
$RemoteArchive = "/tmp/homeops-server-agent-src-$Stamp.tar.gz"

function Write-Step($Message) {
    Write-Host "[deploy] $Message" -ForegroundColor Cyan
}

function Invoke-SshText([string]$Command) {
    $output = & ssh $SshHost $Command
    if ($LASTEXITCODE -ne 0) {
        throw "ssh command failed: $Command"
    }
    ($output -join "`n").Trim()
}

function Assert-RemoteConfigSafe {
    Write-Step "checking remote config safety"
    $json = Invoke-SshText "python3 - <<'PY'
import json, ipaddress
d=json.load(open('/srv/homeops/data/homeops_config.json'))
bind=d.get('bind_host')
port=d.get('bind_port')
errors=[]
if not d.get('api_token'):
    errors.append('api_token is not configured')
if d.get('allow_delete') is not False:
    errors.append('allow_delete must be false')
if bind in ('0.0.0.0','::'):
    errors.append('bind_host must not be wildcard')
try:
    ip=ipaddress.ip_address(bind)
    if not (ip.is_loopback or (ip.version == 4 and ipaddress.ip_address('100.64.0.0') <= ip <= ipaddress.ip_address('100.127.255.255'))):
        errors.append('bind_host must be loopback or Tailscale IPv4')
except Exception:
    if bind != 'localhost':
        errors.append('bind_host must be localhost, loopback, or Tailscale IPv4')
if bind == '$ExpectedBindHost' and d.get('direct_tailscale_enabled') is not True:
    errors.append('direct_tailscale_enabled must be true for direct Tailscale mode')
print(json.dumps({'ok': not errors, 'errors': errors, 'bind_host': bind, 'bind_port': port, 'allow_delete': d.get('allow_delete'), 'direct_tailscale_enabled': d.get('direct_tailscale_enabled'), 'api_token_configured': bool(d.get('api_token'))}))
PY"
    $config = $json | ConvertFrom-Json
    if (-not $config.ok) {
        throw "unsafe remote config: $($config.errors -join '; ')"
    }
    Write-Step "remote config safe: bind=$($config.bind_host):$($config.bind_port), token configured=$($config.api_token_configured), allow_delete=$($config.allow_delete)"
}

function New-SourceArchive {
    Write-Step "creating source archive"
    if (Test-Path $Archive) {
        Remove-Item -LiteralPath $Archive -Force
    }
    Push-Location $RepoRoot
    try {
        tar --exclude=target --exclude=*/target --exclude=node_modules --exclude=*/node_modules -czf $Archive Cargo.toml Cargo.lock services packages
        if ($LASTEXITCODE -ne 0) {
            throw "tar failed"
        }
    } finally {
        Pop-Location
    }
}

function Verify-RemoteService {
    Write-Step "verifying service and listener"
    $active = Invoke-SshText "systemctl is-active homeops-agent.service"
    if ($active -ne "active") {
        throw "homeops-agent.service is $active"
    }

    $listener = Invoke-SshText "ss -ltnp '( sport = :$ExpectedPort )' || true"
    if ($listener -match "0\.0\.0\.0:$ExpectedPort") {
        throw "unsafe wildcard listener detected: $listener"
    }
    if ($listener -notmatch [regex]::Escape("$ExpectedBindHost`:$ExpectedPort")) {
        throw "listener did not match $ExpectedBindHost`:$ExpectedPort`: $listener"
    }

    $health = Invoke-RestMethod -Uri "$DirectUrl/health" -TimeoutSec 8
    if ($health.ok -ne $true) {
        throw "health endpoint returned unexpected response"
    }

    try {
        Invoke-RestMethod -Uri "$DirectUrl/api/settings" -TimeoutSec 8 | Out-Null
        throw "/api/settings unexpectedly succeeded without token"
    } catch {
        $status = $_.Exception.Response.StatusCode.value__
        $body = $null
        if ($_.ErrorDetails.Message) {
            try { $body = $_.ErrorDetails.Message | ConvertFrom-Json } catch { $body = $null }
        }
        if ($status -ne 401 -or -not $body -or $body.code -ne "AUTH_REQUIRED") {
            throw "/api/settings auth check returned HTTP $status"
        }
    }
    Write-Step "service verification passed"
}

try {
    Assert-RemoteConfigSafe
    New-SourceArchive

    Write-Step "uploading source archive"
    & scp $Archive "${SshHost}:$RemoteArchive"
    if ($LASTEXITCODE -ne 0) {
        throw "scp failed"
    }

    Write-Step "building and installing on server"
    & ssh $SshHost @"
set -euo pipefail
STAMP="$Stamp"
REMOTE_ARCHIVE="$RemoteArchive"
SRC_DIR="/srv/homeops/agent/src/current"
BIN="/srv/homeops/agent/bin/server-agent"
BACKUP="/srv/homeops/agent/bin/server-agent.backup.$Stamp"
mkdir -p "`$SRC_DIR" /srv/homeops/agent/bin
rm -rf "`$SRC_DIR"
mkdir -p "`$SRC_DIR"
tar -xzf "`$REMOTE_ARCHIVE" -C "`$SRC_DIR"
cd "`$SRC_DIR"
if [ -f "`$HOME/.cargo/env" ]; then . "`$HOME/.cargo/env"; fi
cargo build -p server-agent --release
sudo cp "`$BIN" "`$BACKUP"
sudo systemctl stop homeops-agent.service
restore_and_exit() {
  echo "[deploy] install/start failed; restoring previous binary"
  sudo cp "`$BACKUP" "`$BIN" || true
  sudo chmod 755 "`$BIN" || true
  sudo systemctl start homeops-agent.service || true
  sudo systemctl is-active homeops-agent.service || true
  exit 1
}
trap restore_and_exit ERR
sudo cp "`$SRC_DIR/target/release/server-agent" "`$BIN"
sudo chmod 755 "`$BIN"
sudo systemctl start homeops-agent.service
trap - ERR
systemctl is-active homeops-agent.service
ss -ltnp '( sport = :$ExpectedPort )' || true
"@
    if ($LASTEXITCODE -ne 0) {
        throw "remote build/install failed"
    }

    Verify-RemoteService
    Write-Step "deployment completed successfully"
} finally {
    if (Test-Path $Archive) {
        Remove-Item -LiteralPath $Archive -Force -ErrorAction SilentlyContinue
    }
}
