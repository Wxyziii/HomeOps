<#
.SYNOPSIS
Imports a ReduxScannerEngine selected RPF export into the ignored HomeOps Redux Maker preview workspace.

.NOTES
This helper only reads the scanner export and copies selected source files into a local ignored workspace.
It never modifies the source export, GTA folders, or RPF archives, and it never runs a converter.
#>

param(
	[Parameter(Mandatory = $true)]
	[string]$InputFolder,

	[Parameter(Mandatory = $true)]
	[string]$OutDir,

	[string]$PreviewDir = "",
	[string]$WeaponPrefix = "w_pi_heavypistol",
	[string]$PreviewId = "",
	[string]$DisplayName = "",
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

function Test-IsUnderPath([string]$Child, [string]$Parent) {
	$childFull = [System.IO.Path]::GetFullPath($Child).TrimEnd('\', '/')
	$parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\', '/')
	return $childFull.StartsWith($parentFull + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase) -or
		$childFull.Equals($parentFull, [System.StringComparison]::OrdinalIgnoreCase)
}

function Convert-ToManifestSlash([string]$PathValue) {
	return $PathValue.Replace('\', '/')
}

function Get-SourceRole($File) {
	$ext = (".{0}" -f $File.extension).ToLowerInvariant()
	$name = [System.IO.Path]::GetFileNameWithoutExtension($File.fileName).ToLowerInvariant()
	$roleGuess = [string]$File.roleGuess

	if ($ext -eq ".ytd") { return "texture_dictionary" }
	if ($ext -eq ".yft") { return "fragment_model" }
	if ($ext -eq ".ydd") { return "drawable_dictionary" }
	if ($ext -eq ".meta" -or $ext -eq ".ymt") { return "metadata" }
	if ($roleGuess -eq "weapon_high_model" -or $name.EndsWith("_hi")) { return "high_drawable_model" }
	if ($ext -eq ".ydr") { return "drawable_model" }
	return "unknown"
}

$inputRoot = (Resolve-Path -LiteralPath $InputFolder).Path
$exportManifestPath = Join-Path $inputRoot "selected_rpf_export_manifest.json"
if (-not (Test-Path -LiteralPath $exportManifestPath)) {
	throw "selected_rpf_export_manifest.json was not found in $inputRoot"
}

$sourceOut = Resolve-WorkspacePath $OutDir
if (-not $PreviewDir) {
	$PreviewDir = Join-Path ".local\redux-maker\previews" (Split-Path -Leaf $sourceOut)
}
$previewOut = Resolve-WorkspacePath $PreviewDir

if (Test-IsUnderPath $sourceOut $inputRoot -or Test-IsUnderPath $previewOut $inputRoot) {
	throw "Output folders must not be inside the source export folder."
}

$exportManifest = Get-Content -Raw -LiteralPath $exportManifestPath | ConvertFrom-Json
$allowedExtensions = @("ydr", "yft", "ydd", "ytd", "meta", "ymt")
$allFiles = @($exportManifest.files | Where-Object { $allowedExtensions -contains ([string]$_.extension).ToLowerInvariant() })
$matchingFiles = @($allFiles | Where-Object {
	$base = [System.IO.Path]::GetFileNameWithoutExtension([string]$_.fileName).ToLowerInvariant()
	$guess = ([string]$_.weaponNameGuess).ToLowerInvariant()
	$base.StartsWith($WeaponPrefix.ToLowerInvariant()) -or $guess.StartsWith($WeaponPrefix.ToLowerInvariant())
})

if (-not $matchingFiles.Count) {
	throw "No exported files matched weapon prefix '$WeaponPrefix'."
}

if (-not $PreviewId) {
	$PreviewId = (Split-Path -Leaf $previewOut)
}

if (-not $DisplayName) {
	$DisplayName = ($WeaponPrefix -replace '^w_', '' -replace '_', ' ')
	$DisplayName = (Get-Culture).TextInfo.ToTitleCase($DisplayName)
}

New-Item -ItemType Directory -Force -Path $sourceOut, $previewOut | Out-Null

$copiedFiles = @()
foreach ($file in $matchingFiles) {
	$relativePath = [string]$file.outputRelativePath
	$sourcePath = Join-Path $inputRoot $relativePath
	if (-not (Test-Path -LiteralPath $sourcePath)) {
		continue
	}

	$destinationPath = Join-Path $sourceOut $relativePath
	New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destinationPath) | Out-Null
	Copy-Item -LiteralPath $sourcePath -Destination $destinationPath -Force
	$copiedFiles += $file
}

$sourceFiles = @(
	foreach ($file in $copiedFiles | Sort-Object outputRelativePath) {
		$isPrimaryTemplate = (([string]$file.weaponNameGuess).Equals($WeaponPrefix, [System.StringComparison]::OrdinalIgnoreCase))
		[ordered]@{
			fileName = [string]$file.fileName
			relativePath = Convert-ToManifestSlash ([string]$file.outputRelativePath)
			extension = ".{0}" -f ([string]$file.extension).ToLowerInvariant()
			role = Get-SourceRole $file
			sizeBytes = [int64]$file.sizeBytes
			sha256 = [string]$file.sha256
			status = if ($isPrimaryTemplate) { "used" } else { "present" }
		}
	}
)

$textureDictionaries = @(
	foreach ($file in $sourceFiles | Where-Object { $_.role -eq "texture_dictionary" }) {
		[ordered]@{
			fileName = $file.fileName
			textureNames = @()
			status = "texture mapping not available yet"
		}
	}
)

$firstYtd = ($textureDictionaries | Select-Object -First 1).fileName
$materialSlots = @(
	[ordered]@{
		slotName = "source dictionaries"
		textureName = $null
		sourceYtd = $firstYtd
		status = "texture mapping not available yet"
		notes = "Importer detects .ytd files but does not decode texture names."
	}
)

$requiredFiles = @(
	$sourceFiles |
		Where-Object { $_.status -eq "used" -and $_.extension -in @(".ydr", ".yft", ".ydd", ".ytd") } |
		ForEach-Object { $_.relativePath }
)

$manifest = [ordered]@{
	manifestVersion = "1.0.0"
	previewId = $PreviewId
	displayName = $DisplayName
	weaponName = $DisplayName
	weaponPrefix = $WeaponPrefix
	sourceLabel = "ReduxScannerEngine selected clean GTA export"
	sourceKind = "clean_gta_export"
	previewStatus = "converter_not_configured"
	modelPreview = $null
	sourceFiles = $sourceFiles
	textureDictionaries = $textureDictionaries
	materialSlots = $materialSlots
	conversion = [ordered]@{
		gtaToGlbStatus = "not_configured"
		glbToGtaStatus = "unsupported"
		converterName = $null
		converterVersion = $null
		logsPath = "converter_status.json"
		warnings = @(
			"No trusted local GTA-to-GLB converter has produced preview.glb yet.",
			"Browser preview supports GLB/GLTF only; raw GTA resources are metadata here."
		)
	}
	reverseTemplate = [ordered]@{
		templateWeaponName = $WeaponPrefix
		originalSourceFiles = @($sourceFiles | ForEach-Object { $_.relativePath })
		requiredFiles = $requiredFiles
		stagedOutputDir = $null
		validationStatus = "unsupported"
	}
	warnings = @(
		"Generated from selected RPF export metadata and ignored local source copies.",
		"No real GLB preview exists until a safe converter writes preview.glb.",
		"Texture names and material slots were not decoded from .ytd files."
	)
	notes = @(
		"Generated by scripts/redux-preview/import_selected_rpf_export.ps1.",
		"Original scanner export folder was read only and not modified.",
		"Copied source files belong only in ignored local workspaces."
	)
	buildReady = $false
}

$status = [ordered]@{
	generatedAt = (Get-Date).ToUniversalTime().ToString("o")
	status = "not_configured"
	previewStatus = "converter_not_configured"
	inputFolder = $inputRoot
	sourceOutDir = $sourceOut
	previewDir = $previewOut
	weaponPrefix = $WeaponPrefix
	sourceFileCount = $sourceFiles.Count
	textureDictionaryCount = $textureDictionaries.Count
	previewGlbCreated = $false
	blockers = @(
		"No converter ran during import.",
		"Run convert_gta_to_glb.ps1 to perform safe converter detection and status update."
	)
}

$manifestPath = Join-Path $previewOut "weapon_preview_manifest.json"
$statusPath = Join-Path $previewOut "converter_status.json"
$manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
$status | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statusPath -Encoding UTF8

if ($StaticPreviewRoot) {
	$staticRoot = Resolve-WorkspacePath $StaticPreviewRoot
	$staticPreviewDir = Join-Path $staticRoot $PreviewId
	New-Item -ItemType Directory -Force -Path $staticPreviewDir | Out-Null
	Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $staticPreviewDir "weapon_preview_manifest.json") -Force
	Copy-Item -LiteralPath $statusPath -Destination (Join-Path $staticPreviewDir "converter_status.json") -Force
}

Write-Output "Imported $($sourceFiles.Count) source files for $WeaponPrefix"
Write-Output "Manifest: $manifestPath"
Write-Output "Status: $statusPath"
