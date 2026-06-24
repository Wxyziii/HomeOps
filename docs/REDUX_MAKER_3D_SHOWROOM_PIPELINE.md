# Redux Maker 3D Showroom Pipeline

HomeOps is currently the development shell for Redux Maker. The 3D showroom lives in the HomeOps web app so the scanner, local preview workspace, manifest contract, and frontend viewer can mature before Redux Maker is split into a standalone app.

The current goal is preview-only: convert or stage exported GTA weapon resources into browser-safe GLB/GLTF previews, then show honest status in `/redux-maker`. HomeOps must never scan the full GTA corpus here, write RPFs, modify original GTA files, or label fallback geometry as a real converted model.

See `docs/REDUX_MAKER_3D_CONVERSION_PIPELINE.md` for the current Heavy Pistol conversion report. The forward `.ydr` to `.glb` bridge is now working for the local clean Heavy Pistol export; reverse GLB-to-GTA staging remains incomplete.

## Formats

GTA source formats are not browser render formats:

- `.ydr`: drawable/model resource
- `.yft`: fragment or physics-capable model resource
- `.ydd`: drawable dictionary
- `.ytd`: texture dictionary
- `.meta` / `.ymt`: metadata/configuration

Three.js loads only `.glb` or `.gltf` in this pipeline. Raw `.ydr`, `.yft`, `.ydd`, and `.ytd` files are listed as source metadata and copied only into ignored local workspaces.

## Workspace Layout

Use an ignored local workspace:

```text
.local/redux-maker/
  source-exports/
    clean-heavypistol/
      files/...
  previews/
    clean-heavypistol/
      weapon_preview_manifest.json
      converter_status.json
  staged-gta-output/
    clean-heavypistol/
      reverse_converter_status.json
```

For browser-local development, scripts may also publish safe manifest/status files under the ignored static folder:

```text
apps/web/static/redux-previews/local/<previewId>/
  weapon_preview_manifest.json
  converter_status.json
```

Do not commit `.local/`, `apps/web/static/redux-previews/local/`, raw GTA assets, or GTA-derived generated GLB files unless a separate explicit local-only policy allows it.

## Manifest Contract

`WeaponPreviewManifest` is defined in `apps/web/src/lib/types/reduxPreview.ts`. The viewer consumes:

- `manifestVersion`, `previewId`, `displayName`, `weaponName`, `weaponPrefix`
- `sourceKind`: `clean_gta_export`, `gunpack_export`, `manual_glb`, `demo_placeholder`, `unsupported_gta_resource`
- `previewStatus`: `ready`, `missing_glb`, `converter_not_configured`, `converter_failed`, `unsupported_source_format`, `template_export_ready`, `reverse_export_ready`
- `modelPreview`: GLB/GLTF URL, format, size, SHA-256, generated timestamp
- `sourceFiles`: filename, relative path, extension, role, size, SHA-256, status
- `textureDictionaries`: detected `.ytd` files and texture names when known
- `materialSlots`: material/texture mapping when known
- `conversion`: GTA-to-GLB and GLB-to-GTA status, converter identity, log path, warnings
- `reverseTemplate`: original template files, required files, staged output directory, validation status
- `warnings`, `notes`, and `buildReady=false`

The showroom shows the real model badge only when:

```text
sourceKind is clean_gta_export or gunpack_export
conversion.gtaToGlbStatus is ready
modelPreview.url exists
```

Manual GLB loads are preview-only and do not get the real GTA badge.

## GTA to GLB

Import the selected scanner export:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\import_selected_rpf_export.ps1 `
  -InputFolder "C:\Users\Marcel\Downloads\ReduxScannerEngine_GitHubRepo\.tmp\selected-rpf-exports\clean-heavypistol" `
  -OutDir ".local\redux-maker\source-exports\clean-heavypistol" `
  -PreviewDir ".local\redux-maker\previews\clean-heavypistol" `
  -WeaponPrefix "w_pi_heavypistol" `
  -PreviewId "clean-heavypistol" `
  -StaticPreviewRoot "apps\web\static\redux-previews\local"
```

Run converter detection/status:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\convert_gta_to_glb.ps1 `
  -PreviewDir ".local\redux-maker\previews\clean-heavypistol" `
  -StaticPreviewRoot "apps\web\static\redux-previews\local"
```

Current implementation uses the local HomeOps CodeWalker.Core GLB bridge. It parses the selected copied `.ydr`, writes a real `preview.glb`, updates `modelPreview`, sets `previewStatus=ready`, sets `conversion.gtaToGlbStatus=ready`, and publishes only ignored local preview assets for browser development.

## GLB to GTA Staged Reverse

The reverse path is template-based by design:

```text
edited.glb + edited textures + original GTA template files
  -> validation
  -> staged output under .local/redux-maker/staged-gta-output
```

Run the current status helper:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\convert_glb_to_gta_staged.ps1 `
  -InputGlb ".local\redux-maker\previews\clean-heavypistol\edited.glb" `
  -TemplateManifest ".local\redux-maker\previews\clean-heavypistol\weapon_preview_manifest.json" `
  -OutDir ".local\redux-maker\staged-gta-output\clean-heavypistol" `
  -StaticPreviewRoot "apps\web\static\redux-previews\local"
```

Current reverse export status is `incomplete`. The helper validates and inspects the GLB geometry, writes `reverse_converter_status.json`, and creates no fake `.ydr` or `.ytd` files. A safe writer still needs to replace template drawable geometry and serialize GTA resource headers, shader/material names, texture dictionary links, scale/origin rules, attachment metadata, and high model handling.

## Validation

Validate a bundle:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\redux-preview\validate_weapon_preview_bundle.ps1 `
  -ManifestPath ".local\redux-maker\previews\clean-heavypistol\weapon_preview_manifest.json" `
  -SourceRoot ".local\redux-maker\source-exports\clean-heavypistol" `
  -PreviewDir ".local\redux-maker\previews\clean-heavypistol"
```

Run manifest contract tests:

```powershell
npm run test:redux-preview
```

Run HomeOps web checks:

```powershell
npm run check
npm run build
```

## Showroom Integration

The `/redux-maker` route uses `WeaponModelViewer.svelte`. The existing GLB loader, placeholder model, orbit/zoom/pan/reset/grid/lighting controls, prompt workflow, change cards, and non-weapon before/after previews remain intact.

For local Heavy Pistol browser smoke, prompt for a gun/weapon/pistol change in `/redux-maker`, then use the ignored static manifest URL:

```text
/redux-previews/local/clean-heavypistol/weapon_preview_manifest.json
```

If no real GLB exists, the viewer shows the generated fallback with converter blockers, source `.ydr`/`.ytd` details, texture dictionary status, and reverse status.

## Safety Rules

- Do not write to original GTA files.
- Do not write to original scanner exports.
- Do not write RPF files.
- Do not scan the full 605 GB corpus in this pipeline.
- Do not extract packages here.
- Do not execute downloaded converter binaries or modpack tools.
- Do not expose arbitrary shell execution through the UI.
- Do not commit raw `.ydr`, `.yft`, `.ydd`, `.ytd`, or GTA-derived generated GLB assets.
- Keep `buildReady=false` until a separate install/build workflow exists.

## Future Gunpack Previews

Gunpack previews should plug in by producing the same manifest shape with `sourceKind=gunpack_export`. The pipeline should copy source files to an ignored temp workspace, convert to local `preview.glb`, update `modelPreview`, validate the GLB, and publish only ignored local static preview assets for the browser.
