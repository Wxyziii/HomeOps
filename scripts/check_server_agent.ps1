param(
    [string]$SshHost = "homeops",
    [string]$DirectUrl = "http://100.68.7.42:8787",
    [string]$ExpectedBindHost = "100.68.7.42",
    [int]$ExpectedPort = 8787
)

$ErrorActionPreference = "Stop"

function Write-Pass($Message) {
    Write-Host "[PASS] $Message" -ForegroundColor Green
}

function Write-Fail($Message) {
    Write-Host "[FAIL] $Message" -ForegroundColor Red
}

function Write-Info($Message) {
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Invoke-SshText([string]$Command) {
    $output = & ssh $SshHost $Command
    if ($LASTEXITCODE -ne 0) {
        throw "ssh command failed: $Command"
    }
    ($output -join "`n").Trim()
}

function Test-ApiSettingsRequiresToken([string]$Url) {
    try {
        Invoke-RestMethod -Uri "$Url/api/settings" -TimeoutSec 8 | Out-Null
        return "UNEXPECTED_SUCCESS"
    } catch {
        $status = $_.Exception.Response.StatusCode.value__
        $body = $null
        if ($_.ErrorDetails.Message) {
            try {
                $body = $_.ErrorDetails.Message | ConvertFrom-Json
            } catch {
                $body = $null
            }
        }
        if ($status -eq 401 -and $body -and $body.code -eq "AUTH_REQUIRED") {
            return "AUTH_REQUIRED"
        }
        return "HTTP_$status"
    }
}

$failed = $false

try {
    Invoke-SshText "true" | Out-Null
    Write-Pass "SSH connectivity to $SshHost"
} catch {
    Write-Fail "SSH connectivity to $SshHost failed: $($_.Exception.Message)"
    exit 1
}

try {
    $service = Invoke-SshText "systemctl is-active homeops-agent.service"
    if ($service -eq "active") {
        Write-Pass "homeops-agent.service is active"
    } else {
        Write-Fail "homeops-agent.service is $service"
        $failed = $true
    }
} catch {
    Write-Fail "could not read systemd status: $($_.Exception.Message)"
    $failed = $true
}

try {
    $configJson = Invoke-SshText "python3 - <<'PY'
import json
d=json.load(open('/srv/homeops/data/homeops_config.json'))
print(json.dumps({
  'bind_host': d.get('bind_host'),
  'bind_port': d.get('bind_port'),
  'allow_delete': d.get('allow_delete'),
  'direct_tailscale_enabled': d.get('direct_tailscale_enabled'),
  'api_token_configured': bool(d.get('api_token'))
}))
PY"
    $config = $configJson | ConvertFrom-Json
    if ($config.api_token_configured -eq $true) { Write-Pass "api_token is configured" } else { Write-Fail "api_token is not configured"; $failed = $true }
    if ($config.allow_delete -eq $false) { Write-Pass "allow_delete=false" } else { Write-Fail "allow_delete is not false"; $failed = $true }
    if ($config.bind_host -ne "0.0.0.0" -and $config.bind_host -ne "::") { Write-Pass "bind_host is not wildcard ($($config.bind_host))" } else { Write-Fail "bind_host is unsafe wildcard"; $failed = $true }
    if ($config.bind_host -eq $ExpectedBindHost -and [int]$config.bind_port -eq $ExpectedPort) { Write-Pass "config bind matches expected $ExpectedBindHost`:$ExpectedPort" } else { Write-Fail "config bind is $($config.bind_host):$($config.bind_port), expected $ExpectedBindHost`:$ExpectedPort"; $failed = $true }
} catch {
    Write-Fail "config safety check failed: $($_.Exception.Message)"
    $failed = $true
}

try {
    $listener = Invoke-SshText "ss -ltnp '( sport = :$ExpectedPort )' || true"
    if ($listener -match "0\.0\.0\.0:$ExpectedPort") {
        Write-Fail "listener is wildcard: $listener"
        $failed = $true
    } elseif ($listener -match [regex]::Escape("$ExpectedBindHost`:$ExpectedPort")) {
        Write-Pass "listener is $ExpectedBindHost`:$ExpectedPort"
    } else {
        Write-Fail "listener did not match expected bind: $listener"
        $failed = $true
    }
} catch {
    Write-Fail "listener check failed: $($_.Exception.Message)"
    $failed = $true
}

try {
    $health = Invoke-RestMethod -Uri "$DirectUrl/health" -TimeoutSec 8
    if ($health.ok -eq $true) {
        Write-Pass "direct health works at $DirectUrl"
    } else {
        Write-Fail "direct health returned unexpected response"
        $failed = $true
    }
} catch {
    Write-Fail "direct health failed at $DirectUrl`: $($_.Exception.Message)"
    $failed = $true
}

$authResult = Test-ApiSettingsRequiresToken $DirectUrl
if ($authResult -eq "AUTH_REQUIRED") {
    Write-Pass "/api/settings requires token"
} else {
    Write-Fail "/api/settings auth check returned $authResult"
    $failed = $true
}

if ($failed) {
    Write-Fail "server-agent checks failed"
    exit 1
}

Write-Pass "server-agent checks passed"
