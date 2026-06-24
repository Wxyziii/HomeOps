<#
.SYNOPSIS
Prepares the strict Redux Maker GLB-to-GTA staged export status.

.NOTES
This script validates and inspects the edited GLB, but does not attempt blind
GLB-to-YDR/YTD conversion. GTA output requires a safe resource writer and
original template resources. Until that toolchain exists, it writes
reverse_converter_status.json and updates the manifest with an honest
incomplete status.
#>

param(
	[Parameter(Mandatory = $true)]
	[string]$InputGlb,

	[Parameter(Mandatory = $true)]
	[string]$TemplateManifest,

	[Parameter(Mandatory = $true)]
	[string]$OutDir,

	[string]$StaticPreviewRoot = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")

function Resolve-WorkspacePath([string]$PathValue) {
	if ([System.IO.Path]::IsPathRooted($PathValue)) {
		return [System.IO.Path]::GetFullPath($PathValue)
	}

	return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $PathValue))
}

function Test-IsForbiddenOutputPath([string]$PathValue) {
	$full = [System.IO.Path]::GetFullPath($PathValue)
	return $full -match '\\Grand Theft Auto V(\\|$)' -or $full -match '\.rpf(\\|$)'
}

function Set-JsonProperty($Object, [string]$Name, $Value) {
	if ($Object.PSObject.Properties[$Name]) {
		$Object.$Name = $Value
	} else {
		$Object | Add-Member -NotePropertyName $Name -NotePropertyValue $Value
	}
}

function Read-UInt32Le([byte[]]$Bytes, [int]$Offset) {
	return [System.BitConverter]::ToUInt32($Bytes, $Offset)
}

function Read-GlbInfo([string]$PathValue) {
	$bytes = [System.IO.File]::ReadAllBytes($PathValue)
	if ($bytes.Length -lt 20 -or [System.Text.Encoding]::ASCII.GetString($bytes, 0, 4) -ne "glTF") {
		throw "Input file is not a GLB 2.0 file: $PathValue"
	}
	$version = Read-UInt32Le $bytes 4
	$declaredLength = Read-UInt32Le $bytes 8
	if ($version -ne 2) {
		throw "Only GLB version 2 is supported for inspection. Found version $version."
	}
	if ($declaredLength -ne $bytes.Length) {
		throw "GLB declared length does not match file length."
	}

	$jsonLength = [int](Read-UInt32Le $bytes 12)
	$jsonType = Read-UInt32Le $bytes 16
	if ($jsonType -ne 0x4E4F534A -or 20 + $jsonLength -gt $bytes.Length) {
		throw "GLB JSON chunk is missing or invalid."
	}

	$jsonText = [System.Text.Encoding]::UTF8.GetString($bytes, 20, $jsonLength).Trim([char]0, [char]32)
	$gltf = $jsonText | ConvertFrom-Json
	$primitiveCount = 0
	foreach ($mesh in @($gltf.meshes)) {
		$primitiveCount += @($mesh.primitives).Count
	}

	return [ordered]@{
		version = $version
		byteLength = $bytes.Length
		meshCount = @($gltf.meshes).Count
		primitiveCount = $primitiveCount
		materialCount = @($gltf.materials).Count
		accessorCount = @($gltf.accessors).Count
		bufferCount = @($gltf.buffers).Count
	}
}

$manifestPath = Resolve-WorkspacePath $TemplateManifest
if (-not (Test-Path -LiteralPath $manifestPath)) {
	throw "Template manifest not found: $manifestPath"
}

$outputDir = Resolve-WorkspacePath $OutDir
if (Test-IsForbiddenOutputPath $outputDir) {
	throw "Staged output directory must not be inside a GTA folder or RPF-like path."
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$inputGlbPath = Resolve-WorkspacePath $InputGlb
$inputExists = Test-Path -LiteralPath $inputGlbPath
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$glbInfo = $null
$glbParseError = $null
if ($inputExists) {
	try {
		$glbInfo = Read-GlbInfo $inputGlbPath
	} catch {
		$glbParseError = $_.Exception.Message
	}
}

$blockers = @(
	"GLB geometry inspection is implemented, but template-based GLB-to-YDR/YTD writing is not implemented in HomeOps.",
	"CodeWalker.Core can parse the template YDR, but this bridge does not yet replace drawable geometry and serialize a safe new YDR/YTD pair.",
	"Blind GLB-to-GTA output would lose GTA resource headers, drawable structure, shader/material bindings, texture dictionary links, and attachment metadata.",
	"No staged .ydr or .ytd files were created."
)

if (-not $inputExists) {
	$blockers = @("Input GLB was not found: $inputGlbPath") + $blockers
} elseif ($glbParseError) {
	$blockers = @("Input GLB could not be parsed: $glbParseError") + $blockers
}

$status = [ordered]@{
	generatedAt = (Get-Date).ToUniversalTime().ToString("o")
	status = "incomplete"
	inputGlb = $inputGlbPath
	inputGlbExists = $inputExists
	glbGeometryParsed = [bool]$glbInfo
	glbInfo = $glbInfo
	templateManifest = $manifestPath
	outDir = $outputDir
	stagedFilesCreated = @()
	blockers = $blockers
	safety = [ordered]@{
		modifiedOriginalExport = $false
		modifiedOriginalGtaFiles = $false
		wroteRpf = $false
		createdFakeYdrOrYtd = $false
	}
}

$existingGtaStatus = "not_configured"
if ($manifest.PSObject.Properties["conversion"] -and $manifest.conversion.PSObject.Properties["gtaToGlbStatus"]) {
	$existingGtaStatus = $manifest.conversion.gtaToGlbStatus
}

Set-JsonProperty $manifest "conversion" ([ordered]@{
	gtaToGlbStatus = $existingGtaStatus
	glbToGtaStatus = "incomplete"
	converterName = $manifest.conversion.converterName
	converterVersion = $manifest.conversion.converterVersion
	logsPath = $manifest.conversion.logsPath
	warnings = @($manifest.conversion.warnings) + $blockers
})

Set-JsonProperty $manifest "reverseTemplate" ([ordered]@{
	templateWeaponName = $manifest.weaponPrefix
	originalSourceFiles = @($manifest.sourceFiles | ForEach-Object { $_.relativePath })
	requiredFiles = @($manifest.reverseTemplate.requiredFiles)
	stagedOutputDir = $outputDir
	validationStatus = "incomplete"
})

$warnings = @($manifest.warnings)
foreach ($blocker in $blockers) {
	if ($warnings -notcontains $blocker) {
		$warnings += $blocker
	}
}
Set-JsonProperty $manifest "warnings" $warnings
Set-JsonProperty $manifest "buildReady" $false

$statusPath = Join-Path $outputDir "reverse_converter_status.json"
$status | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $statusPath -Encoding UTF8
$manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

if ($StaticPreviewRoot) {
	$staticRoot = Resolve-WorkspacePath $StaticPreviewRoot
	$previewId = $manifest.previewId
	$staticPreviewDir = Join-Path $staticRoot $previewId
	New-Item -ItemType Directory -Force -Path $staticPreviewDir | Out-Null
	Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $staticPreviewDir "weapon_preview_manifest.json") -Force
	Copy-Item -LiteralPath $statusPath -Destination (Join-Path $staticPreviewDir "reverse_converter_status.json") -Force
}

Write-Output "GLB-to-GTA staged conversion status: incomplete"
Write-Output "Status: $statusPath"
