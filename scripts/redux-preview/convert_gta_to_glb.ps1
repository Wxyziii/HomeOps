<#
.SYNOPSIS
Runs the safe Redux Maker GTA-to-GLB preview conversion bridge.

.NOTES
This script reads copied GTA export files from an ignored local workspace, uses the
HomeOps CodeWalker.Core bridge to parse a selected YDR, and writes preview.glb.
It never writes RPFs, never modifies source exports, and never executes downloaded
converter executables.
#>

param(
	[string]$PreviewDir = ".local\redux-maker\previews\clean-heavypistol",
	[string]$ManifestPath = "",
	[string]$SourceDir = ".local\redux-maker\source-exports\clean-heavypistol",
	[string]$ConverterWorkspace = ".local\redux-maker\converter-workspaces\clean-heavypistol",
	[string]$CodeWalkerCoreDir = "",
	[string]$StaticPreviewRoot = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")
$bridgeProject = Join-Path $PSScriptRoot "CodeWalkerGtaBridge\CodeWalkerGtaBridge.csproj"

function Resolve-WorkspacePath([string]$PathValue) {
	if (-not $PathValue) { return "" }
	if ([System.IO.Path]::IsPathRooted($PathValue)) {
		return [System.IO.Path]::GetFullPath($PathValue)
	}

	return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $PathValue))
}

function Set-JsonProperty($Object, [string]$Name, $Value) {
	if ($Object.PSObject.Properties[$Name]) {
		$Object.$Name = $Value
	} else {
		$Object | Add-Member -NotePropertyName $Name -NotePropertyValue $Value
	}
}

function Get-Sha256([string]$PathValue) {
	return (Get-FileHash -LiteralPath $PathValue -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Add-UniqueWarning([object[]]$Existing, [string[]]$Additions) {
	$result = @($Existing)
	foreach ($warning in $Additions) {
		if ($warning -and $result -notcontains $warning) {
			$result += $warning
		}
	}
	return $result
}

function Read-OptionalConverterConfig {
	$configPath = Join-Path $repoRoot ".local\redux-maker\converter-config.json"
	if (Test-Path -LiteralPath $configPath) {
		return Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
	}

	return $null
}

function Select-SourceYdr($Manifest, [string]$SourceRoot) {
	$candidates = @($Manifest.sourceFiles | Where-Object {
		$_.relativePath -and $_.extension -eq ".ydr" -and $_.status -in @("used", "present")
	})

	$orderedCandidates = @(
		$candidates | Where-Object { $_.role -eq "high_drawable_model" -and $_.fileName -match "_hi\.ydr$" }
		$candidates | Where-Object { $_.role -eq "high_drawable_model" }
		$candidates | Where-Object { $_.role -eq "drawable_model" }
		$candidates
	) | Where-Object { $_ } | Select-Object -Unique

	foreach ($candidate in $orderedCandidates) {
		$fullPath = Join-Path $SourceRoot $candidate.relativePath
		if (Test-Path -LiteralPath $fullPath) {
			return [ordered]@{
				manifestEntry = $candidate
				sourcePath = [System.IO.Path]::GetFullPath($fullPath)
			}
		}
	}

	throw "No readable .ydr source file from manifest was found under $SourceRoot"
}

function Invoke-LoggedProcess([string]$FilePath, [string[]]$Arguments, [string]$WorkingDirectory, [hashtable]$Environment = @{}) {
	$psi = [System.Diagnostics.ProcessStartInfo]::new()
	$psi.FileName = $FilePath
	$psi.Arguments = ($Arguments | ForEach-Object {
		if ($_ -match '[\s"]') {
			'"' + ($_ -replace '"', '\"') + '"'
		} else {
			$_
		}
	}) -join " "
	$psi.WorkingDirectory = $WorkingDirectory
	$psi.UseShellExecute = $false
	$psi.RedirectStandardOutput = $true
	$psi.RedirectStandardError = $true
	foreach ($key in $Environment.Keys) {
		$psi.Environment[$key] = [string]$Environment[$key]
	}

	$process = [System.Diagnostics.Process]::Start($psi)
	$stdout = $process.StandardOutput.ReadToEnd()
	$stderr = $process.StandardError.ReadToEnd()
	$process.WaitForExit()

	return [ordered]@{
		file = $FilePath
		arguments = $Arguments
		exitCode = $process.ExitCode
		stdout = $stdout
		stderr = $stderr
	}
}

$previewPath = Resolve-WorkspacePath $PreviewDir
if (-not $ManifestPath) {
	$ManifestPath = Join-Path $previewPath "weapon_preview_manifest.json"
}
$manifestFile = Resolve-WorkspacePath $ManifestPath
$sourceRoot = Resolve-WorkspacePath $SourceDir
$workspaceRoot = Resolve-WorkspacePath $ConverterWorkspace

if (-not (Test-Path -LiteralPath $manifestFile)) {
	throw "Preview manifest not found: $manifestFile"
}
if (-not (Test-Path -LiteralPath $sourceRoot)) {
	throw "Source export folder not found: $sourceRoot"
}

New-Item -ItemType Directory -Force -Path $previewPath | Out-Null
New-Item -ItemType Directory -Force -Path $workspaceRoot | Out-Null

$manifest = Get-Content -Raw -LiteralPath $manifestFile | ConvertFrom-Json
$config = Read-OptionalConverterConfig
if (-not $CodeWalkerCoreDir -and $env:CODEWALKER_CORE_DIR) {
	$CodeWalkerCoreDir = $env:CODEWALKER_CORE_DIR
}
if (-not $CodeWalkerCoreDir -and $config -and $config.PSObject.Properties["codeWalkerCoreDir"]) {
	$CodeWalkerCoreDir = $config.codeWalkerCoreDir
}
if (-not $CodeWalkerCoreDir) {
	throw "CodeWalkerCoreDir is required. Pass -CodeWalkerCoreDir, set CODEWALKER_CORE_DIR, or create .local\redux-maker\converter-config.json."
}

$codeWalkerCorePath = Resolve-WorkspacePath $CodeWalkerCoreDir
$codeWalkerDll = Join-Path $codeWalkerCorePath "CodeWalker.Core.dll"
if (-not (Test-Path -LiteralPath $codeWalkerDll)) {
	throw "CodeWalker.Core.dll not found under CodeWalkerCoreDir: $codeWalkerCorePath"
}

$selected = Select-SourceYdr $manifest $sourceRoot
$inputDir = Join-Path $workspaceRoot "input"
$logsDir = Join-Path $workspaceRoot "logs"
New-Item -ItemType Directory -Force -Path $inputDir, $logsDir | Out-Null

$copiedYdr = Join-Path $inputDir $selected.manifestEntry.fileName
Copy-Item -LiteralPath $selected.sourcePath -Destination $copiedYdr -Force

$previewGlbPath = Join-Path $previewPath "preview.glb"
$bridgeStatusPath = Join-Path $previewPath "codewalker_bridge_status.json"
$converterStatusPath = Join-Path $previewPath "converter_status.json"

$processEnvironment = @{ CODEWALKER_CORE_DIR = $codeWalkerCorePath }
$build = Invoke-LoggedProcess "dotnet" @("build", $bridgeProject, "-v:minimal") $repoRoot $processEnvironment
if ($build.exitCode -ne 0) {
	throw "CodeWalker bridge build failed. See converter_status.json after this script is rerun with a valid bridge."
}

$bridgeDll = Join-Path $PSScriptRoot "CodeWalkerGtaBridge\bin\Debug\net9.0\CodeWalkerGtaBridge.dll"
if (-not (Test-Path -LiteralPath $bridgeDll)) {
	throw "Built bridge DLL not found: $bridgeDll"
}

$run = Invoke-LoggedProcess "dotnet" @(
	$bridgeDll,
	"--input-ydr", $copiedYdr,
	"--output-glb", $previewGlbPath,
	"--status-json", $bridgeStatusPath
) $repoRoot $processEnvironment

$bridgeStatus = $null
if (Test-Path -LiteralPath $bridgeStatusPath) {
	$bridgeStatus = Get-Content -Raw -LiteralPath $bridgeStatusPath | ConvertFrom-Json
}

$ready = $run.exitCode -eq 0 -and (Test-Path -LiteralPath $previewGlbPath) -and $bridgeStatus -and $bridgeStatus.status -eq "ready"
$warnings = @(
	"YTD texture decode is not implemented yet; the GLB uses material placeholders.",
	"GLB-to-GTA staged export remains incomplete until a safe YDR/YTD writer is implemented."
)

if ($ready) {
	$previewBytes = (Get-Item -LiteralPath $previewGlbPath).Length
	$previewHash = Get-Sha256 $previewGlbPath
	Set-JsonProperty $manifest "previewStatus" "ready"
	Set-JsonProperty $manifest "modelPreview" ([ordered]@{
		url = "preview.glb"
		format = "glb"
		sizeBytes = $previewBytes
		sha256 = $previewHash
		generatedAt = (Get-Date).ToUniversalTime().ToString("o")
		validationStatus = "ready"
		meshCount = $bridgeStatus.meshCount
		primitiveCount = $bridgeStatus.primitiveCount
		vertexCount = $bridgeStatus.vertexCount
		indexCount = $bridgeStatus.indexCount
		materialCount = $bridgeStatus.materialCount
		boundingBox = $bridgeStatus.boundingBox
		realGtaModel = $true
	})

	$existingReverseStatus = "incomplete"
	if ($manifest.PSObject.Properties["conversion"] -and $manifest.conversion.PSObject.Properties["glbToGtaStatus"]) {
		$existingReverseStatus = $manifest.conversion.glbToGtaStatus
	}

	Set-JsonProperty $manifest "conversion" ([ordered]@{
		gtaToGlbStatus = "ready"
		glbToGtaStatus = $existingReverseStatus
		converterName = "HomeOps CodeWalker.Core GLB bridge"
		converterVersion = "CodeWalker.Core local"
		logsPath = "converter_status.json"
		warnings = $warnings
	})

	$oldWarningsToDrop = @(
		"No real GLB preview exists until a safe converter writes preview.glb.",
		"No trusted installed GTA model converter was found on PATH or under Program Files.",
		"No native .ydr/.yft/.ydd/.ytd parser is implemented in HomeOps.",
		"Three.js cannot load GTA resource files directly; preview.glb was not created."
	)
	$filteredWarnings = @($manifest.warnings | Where-Object { $oldWarningsToDrop -notcontains $_ })
	Set-JsonProperty $manifest "warnings" (Add-UniqueWarning $filteredWarnings $warnings)
	Set-JsonProperty $manifest "buildReady" $false
} else {
	$failureWarnings = @("CodeWalker bridge did not produce a ready GLB preview.")
	if ($bridgeStatus -and $bridgeStatus.PSObject.Properties["error"]) {
		$failureWarnings += $bridgeStatus.error
	}
	Set-JsonProperty $manifest "previewStatus" "converter_failed"
	Set-JsonProperty $manifest "modelPreview" $null
	Set-JsonProperty $manifest "conversion" ([ordered]@{
		gtaToGlbStatus = "failed"
		glbToGtaStatus = "incomplete"
		converterName = "HomeOps CodeWalker.Core GLB bridge"
		converterVersion = "CodeWalker.Core local"
		logsPath = "converter_status.json"
		warnings = $failureWarnings
	})
	Set-JsonProperty $manifest "warnings" (Add-UniqueWarning @($manifest.warnings) $failureWarnings)
	Set-JsonProperty $manifest "buildReady" $false
}

$status = [ordered]@{
	generatedAt = (Get-Date).ToUniversalTime().ToString("o")
	status = if ($ready) { "ready" } else { "failed" }
	previewStatus = $manifest.previewStatus
	previewDir = $previewPath
	manifestPath = $manifestFile
	sourceRoot = $sourceRoot
	selectedSource = [ordered]@{
		fileName = $selected.manifestEntry.fileName
		relativePath = $selected.manifestEntry.relativePath
		role = $selected.manifestEntry.role
		sourcePath = $selected.sourcePath
		copiedPath = $copiedYdr
		sourceSha256 = Get-Sha256 $selected.sourcePath
	}
	workspace = $workspaceRoot
	codeWalkerCoreDir = $codeWalkerCorePath
	bridgeProject = $bridgeProject
	previewGlbPath = $previewGlbPath
	previewGlbCreated = Test-Path -LiteralPath $previewGlbPath
	previewGlbSha256 = if (Test-Path -LiteralPath $previewGlbPath) { Get-Sha256 $previewGlbPath } else { $null }
	build = $build
	run = $run
	bridgeStatus = $bridgeStatus
	safety = [ordered]@{
		modifiedOriginalExport = $false
		modifiedOriginalGtaFiles = $false
		wroteRpf = $false
		executedDownloadedTool = $false
		createdPlaceholderGlb = $false
		outputUnderIgnoredLocalWorkspace = $true
	}
}

$status | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $converterStatusPath -Encoding UTF8
$manifest | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $manifestFile -Encoding UTF8

if ($StaticPreviewRoot) {
	$staticRoot = Resolve-WorkspacePath $StaticPreviewRoot
	$previewId = $manifest.previewId
	$staticPreviewDir = Join-Path $staticRoot $previewId
	New-Item -ItemType Directory -Force -Path $staticPreviewDir | Out-Null
	Copy-Item -LiteralPath $manifestFile -Destination (Join-Path $staticPreviewDir "weapon_preview_manifest.json") -Force
	Copy-Item -LiteralPath $converterStatusPath -Destination (Join-Path $staticPreviewDir "converter_status.json") -Force
	Copy-Item -LiteralPath $bridgeStatusPath -Destination (Join-Path $staticPreviewDir "codewalker_bridge_status.json") -Force
	if (Test-Path -LiteralPath $previewGlbPath) {
		Copy-Item -LiteralPath $previewGlbPath -Destination (Join-Path $staticPreviewDir "preview.glb") -Force
	}
}

if ($ready) {
	Write-Output "GTA-to-GLB conversion ready: $previewGlbPath"
} else {
	Write-Output "GTA-to-GLB conversion failed. Status: $converterStatusPath"
	exit 1
}
