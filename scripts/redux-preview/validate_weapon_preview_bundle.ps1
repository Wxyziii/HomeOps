<#
.SYNOPSIS
Validates a Redux Maker weapon preview bundle.
#>

param(
	[Parameter(Mandatory = $true)]
	[string]$ManifestPath,

	[string]$SourceRoot = "",
	[string]$PreviewDir = "",
	[string]$StaticPreviewRoot = "apps\web\static\redux-previews"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")

function Resolve-WorkspacePath([string]$PathValue) {
	if (-not $PathValue) { return "" }
	if ([System.IO.Path]::IsPathRooted($PathValue)) {
		return [System.IO.Path]::GetFullPath($PathValue)
	}

	return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $PathValue))
}

function Add-Result([System.Collections.Generic.List[string]]$List, [string]$Message) {
	$List.Add($Message) | Out-Null
}

function Test-IsForbiddenOutputPath([string]$PathValue) {
	if (-not $PathValue) { return $false }
	$full = [System.IO.Path]::GetFullPath($PathValue)
	return $full -match '\\Grand Theft Auto V(\\|$)' -or $full -match '\.rpf(\\|$)'
}

function Read-UInt32Le([byte[]]$Bytes, [int]$Offset) {
	return [System.BitConverter]::ToUInt32($Bytes, $Offset)
}

function Test-GlbFile([string]$PathValue, [System.Collections.Generic.List[string]]$Errors, [System.Collections.Generic.List[string]]$Warnings) {
	$bytes = [System.IO.File]::ReadAllBytes($PathValue)
	if ($bytes.Length -lt 20) {
		Add-Result $Errors "GLB is too small: $PathValue"
		return $null
	}
	if ([System.Text.Encoding]::ASCII.GetString($bytes, 0, 4) -ne "glTF") {
		Add-Result $Errors "GLB header check failed: $PathValue"
		return $null
	}
	$version = Read-UInt32Le $bytes 4
	$declaredLength = Read-UInt32Le $bytes 8
	if ($version -ne 2) {
		Add-Result $Errors "GLB version must be 2, got $version"
	}
	if ($declaredLength -ne $bytes.Length) {
		Add-Result $Errors "GLB declared length $declaredLength does not match file length $($bytes.Length)."
	}

	$jsonChunkLength = [int](Read-UInt32Le $bytes 12)
	$jsonChunkType = Read-UInt32Le $bytes 16
	if ($jsonChunkType -ne 0x4E4F534A) {
		Add-Result $Errors "First GLB chunk is not JSON."
		return $null
	}
	if (20 + $jsonChunkLength -gt $bytes.Length) {
		Add-Result $Errors "GLB JSON chunk length exceeds file length."
		return $null
	}

	$jsonText = [System.Text.Encoding]::UTF8.GetString($bytes, 20, $jsonChunkLength).Trim([char]0, [char]32)
	$gltf = $jsonText | ConvertFrom-Json
	if (-not $gltf.asset -or $gltf.asset.version -ne "2.0") {
		Add-Result $Errors "GLB JSON asset.version must be 2.0."
	}
	if (-not $gltf.meshes -or @($gltf.meshes).Count -lt 1) {
		Add-Result $Errors "GLB must contain at least one mesh."
	}
	if (-not $gltf.buffers -or @($gltf.buffers).Count -lt 1) {
		Add-Result $Errors "GLB must contain a buffer."
	}

	$primitiveCount = 0
	foreach ($mesh in @($gltf.meshes)) {
		foreach ($primitive in @($mesh.primitives)) {
			$primitiveCount++
			if (-not $primitive.attributes -or -not $primitive.attributes.PSObject.Properties["POSITION"]) {
				Add-Result $Errors "Every GLB primitive must include a POSITION accessor."
			}
			if (-not $primitive.PSObject.Properties["indices"]) {
				Add-Result $Warnings "GLB primitive has no indices accessor."
			}
		}
	}
	if ($primitiveCount -lt 1) {
		Add-Result $Errors "GLB must contain at least one primitive."
	}

	return [ordered]@{
		version = $version
		byteLength = $bytes.Length
		meshCount = @($gltf.meshes).Count
		primitiveCount = $primitiveCount
		materialCount = @($gltf.materials).Count
		accessorCount = @($gltf.accessors).Count
	}
}

$manifestFile = Resolve-WorkspacePath $ManifestPath
if (-not (Test-Path -LiteralPath $manifestFile)) {
	throw "Manifest not found: $manifestFile"
}

if (-not $PreviewDir) {
	$PreviewDir = Split-Path -Parent $manifestFile
}

$previewPath = Resolve-WorkspacePath $PreviewDir
$sourceRootPath = Resolve-WorkspacePath $SourceRoot
$staticRoot = Resolve-WorkspacePath $StaticPreviewRoot
$manifest = Get-Content -Raw -LiteralPath $manifestFile | ConvertFrom-Json

$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()

foreach ($required in @("manifestVersion", "previewId", "displayName", "weaponName", "sourceKind", "previewStatus", "sourceFiles", "conversion", "reverseTemplate")) {
	if (-not $manifest.PSObject.Properties[$required]) {
		Add-Result $errors "Missing required manifest field: $required"
	}
}

if ($manifest.buildReady -ne $false) {
	Add-Result $errors "buildReady must remain false for preview bundles."
}

$modelPreview = $manifest.modelPreview
if ($manifest.previewStatus -eq "ready") {
	if (-not $modelPreview -or -not $modelPreview.url) {
		Add-Result $errors "previewStatus=ready requires modelPreview.url."
	} else {
		$modelPath = $modelPreview.url
		if ($modelPath -notmatch '^(https?:|blob:|/)') {
			$modelPath = Join-Path $previewPath $modelPath
		}
		if ($modelPath -notmatch '^(https?:|blob:)' -and -not (Test-Path -LiteralPath $modelPath)) {
			Add-Result $errors "modelPreview.url does not exist: $modelPath"
		}
		if ($modelPreview.format -notin @("glb", "gltf")) {
			Add-Result $errors "modelPreview.format must be glb or gltf."
		}
		if ($modelPreview.format -eq "glb" -and (Test-Path -LiteralPath $modelPath)) {
			$glbInfo = Test-GlbFile $modelPath $errors $warnings
			if ($glbInfo) {
				if ($modelPreview.sizeBytes -and [int64]$modelPreview.sizeBytes -ne [int64]$glbInfo.byteLength) {
					Add-Result $warnings "modelPreview.sizeBytes does not match GLB file length."
				}
				if ($modelPreview.meshCount -and [int]$modelPreview.meshCount -ne [int]$glbInfo.meshCount) {
					Add-Result $warnings "modelPreview.meshCount does not match parsed GLB."
				}
				if ($modelPreview.primitiveCount -and [int]$modelPreview.primitiveCount -ne [int]$glbInfo.primitiveCount) {
					Add-Result $warnings "modelPreview.primitiveCount does not match parsed GLB."
				}
			}
		}
	}
}

if ($manifest.previewStatus -ne "ready" -and $modelPreview -and $modelPreview.url) {
	Add-Result $warnings "modelPreview.url is present while previewStatus is not ready."
}

if ($sourceRootPath) {
	foreach ($file in @($manifest.sourceFiles)) {
		if (-not $file.relativePath) {
			Add-Result $warnings "Source file missing relativePath: $($file.fileName)"
			continue
		}

		$sourcePath = Join-Path $sourceRootPath $file.relativePath
		if ($file.status -in @("present", "used") -and -not (Test-Path -LiteralPath $sourcePath)) {
			Add-Result $errors "Source file marked $($file.status) is missing: $($file.relativePath)"
		}
	}
}

if ($staticRoot -and (Test-Path -LiteralPath $staticRoot)) {
	$rawStatic = Get-ChildItem -LiteralPath $staticRoot -Recurse -File -ErrorAction SilentlyContinue |
		Where-Object { $_.Extension.ToLowerInvariant() -in @(".ydr", ".yft", ".ydd", ".ytd") }
	if ($rawStatic) {
		Add-Result $errors "Raw GTA source files were found under static redux previews."
	}
}

$stagedDir = $manifest.reverseTemplate.stagedOutputDir
if (Test-IsForbiddenOutputPath $stagedDir) {
	Add-Result $errors "reverseTemplate.stagedOutputDir points to a forbidden GTA/RPF path."
}

$result = [ordered]@{
	manifestPath = $manifestFile
	previewId = $manifest.previewId
	previewStatus = $manifest.previewStatus
	gtaToGlbStatus = $manifest.conversion.gtaToGlbStatus
	glbToGtaStatus = $manifest.conversion.glbToGtaStatus
	errors = $errors
	warnings = $warnings
	valid = $errors.Count -eq 0
}

$result | ConvertTo-Json -Depth 8
if ($errors.Count) {
	exit 1
}
