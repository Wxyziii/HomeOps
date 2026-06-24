# Redux Maker 3D Conversion Pipeline

Redux Maker now has a real local GTA drawable preview bridge inside HomeOps. The pipeline is still development-only and writes all GTA-derived assets under ignored local folders.

## Current Status

- GTA `.ydr` to browser `.glb`: working for the clean Heavy Pistol sample through `HomeOps CodeWalker.Core GLB bridge`.
- GLB validation: working; the validator reads GLB 2.0 JSON chunks and checks meshes, primitives, buffers, and POSITION accessors.
- HomeOps viewer: consumes the generated manifest and loads `preview.glb` through the existing Three.js GLB path.
- GLB back to GTA `.ydr`/`.ytd`: incomplete. The reverse helper parses the GLB and writes status, but it does not create staged GTA resources until a safe drawable writer exists.

## Local Source Used

The task referenced a sibling source export folder, but that path was not present on disk. The available local export is:

```text
C:\Users\Marcel\Documents\GitHub\HomeOpsPanel\.local\redux-maker\source-exports\clean-heavypistol
```

The selected source model is the high drawable:

```text
files/update/update.rpf/weapons.rpf/w_pi_heavypistol_hi.ydr
```

The converter copies this file into an ignored converter workspace before parsing it:

```text
.local/redux-maker/converter-workspaces/clean-heavypistol/input/w_pi_heavypistol_hi.ydr
```

## Forward Conversion

Run:

```powershell
$env:CODEWALKER_CORE_DIR = "C:\Users\Marcel\Downloads\CodeWalkerFull\CodeWalker.Core\bin\Debug\netstandard2.0"
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\convert_gta_to_glb.ps1 `
  -PreviewDir ".local\redux-maker\previews\clean-heavypistol" `
  -SourceDir ".local\redux-maker\source-exports\clean-heavypistol" `
  -StaticPreviewRoot "apps\web\static\redux-previews\local"
```

Generated local files:

```text
.local/redux-maker/previews/clean-heavypistol/preview.glb
.local/redux-maker/previews/clean-heavypistol/weapon_preview_manifest.json
.local/redux-maker/previews/clean-heavypistol/converter_status.json
.local/redux-maker/previews/clean-heavypistol/codewalker_bridge_status.json
```

The local browser mirror is ignored by git:

```text
apps/web/static/redux-previews/local/clean-heavypistol/
```

Latest successful Heavy Pistol output:

- mesh count: 1
- primitive count: 5
- vertex count: 8538
- index count: 10941
- material count: 5
- `previewStatus`: `ready`
- `conversion.gtaToGlbStatus`: `ready`

Texture dictionaries are detected in the manifest, but YTD texture decode is not implemented yet. The generated GLB uses material placeholders.

## Reverse Conversion

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\convert_glb_to_gta_staged.ps1 `
  -InputGlb ".local\redux-maker\previews\clean-heavypistol\preview.glb" `
  -TemplateManifest ".local\redux-maker\previews\clean-heavypistol\weapon_preview_manifest.json" `
  -OutDir ".local\redux-maker\staged-gta-output\clean-heavypistol" `
  -StaticPreviewRoot "apps\web\static\redux-previews\local"
```

Current reverse status is `incomplete`. The script parses the GLB and records mesh/primitive/material counts, but it creates no `.ydr`, `.ytd`, or `.rpf` files. The remaining blocker is a safe writer that can replace drawable geometry and serialize GTA resource headers, shader/material bindings, texture dictionary links, scale/origin rules, and attachment metadata without corrupting the original template.

## Validation

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\validate_weapon_preview_bundle.ps1 `
  -ManifestPath ".local\redux-maker\previews\clean-heavypistol\weapon_preview_manifest.json" `
  -SourceRoot ".local\redux-maker\source-exports\clean-heavypistol" `
  -PreviewDir ".local\redux-maker\previews\clean-heavypistol"
```

Expected result:

```json
{
  "previewStatus": "ready",
  "gtaToGlbStatus": "ready",
  "glbToGtaStatus": "incomplete",
  "valid": true
}
```

## Safety Rules

- Never modify original GTA files.
- Never modify original scanner exports.
- Never write RPF archives.
- Never execute downloaded converter executables.
- Never expose arbitrary shell execution through the UI.
- Never commit raw `.ydr`, `.yft`, `.ydd`, `.ytd`, `.rpf`, or GTA-derived generated GLB assets.
- Keep generated assets under ignored `.local/` or `apps/web/static/redux-previews/local/`.
- Keep `buildReady=false` until a separate install/build workflow exists.
