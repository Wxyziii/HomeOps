import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..', '..');

const sourceKinds = new Set([
  'clean_gta_export',
  'gunpack_export',
  'manual_glb',
  'demo_placeholder',
  'unsupported_gta_resource',
  'clean_gta_sample',
  'gunpack_sample'
]);

const previewStatuses = new Set([
  'ready',
  'missing_glb',
  'converter_not_configured',
  'converter_failed',
  'unsupported_source_format',
  'template_export_ready',
  'reverse_export_ready',
  'converter_not_connected',
  'failed'
]);

const conversionStatuses = new Set(['ready', 'unsupported', 'failed', 'not_configured', 'incomplete']);
const sourceRoles = new Set([
  'drawable_model',
  'high_drawable_model',
  'fragment_model',
  'drawable_dictionary',
  'texture_dictionary',
  'metadata',
  'unknown',
  'model',
  'high_model',
  'fragment'
]);

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), 'utf8'));
}

function validateManifest(manifest) {
  for (const field of [
    'manifestVersion',
    'previewId',
    'displayName',
    'weaponName',
    'sourceKind',
    'previewStatus',
    'sourceFiles',
    'textureDictionaries',
    'materialSlots',
    'conversion',
    'reverseTemplate',
    'warnings',
    'notes'
  ]) {
    assert.ok(Object.hasOwn(manifest, field), `missing field ${field}`);
  }

  assert.ok(sourceKinds.has(manifest.sourceKind), `invalid sourceKind ${manifest.sourceKind}`);
  assert.ok(previewStatuses.has(manifest.previewStatus), `invalid previewStatus ${manifest.previewStatus}`);
  assert.ok(conversionStatuses.has(manifest.conversion.gtaToGlbStatus), 'invalid gtaToGlbStatus');
  assert.ok(conversionStatuses.has(manifest.conversion.glbToGtaStatus), 'invalid glbToGtaStatus');
  assert.equal(manifest.buildReady, false, 'buildReady must remain false');
  assert.ok(Array.isArray(manifest.sourceFiles), 'sourceFiles must be an array');
  assert.ok(Array.isArray(manifest.textureDictionaries), 'textureDictionaries must be an array');
  assert.ok(Array.isArray(manifest.materialSlots), 'materialSlots must be an array');
  assert.ok(Array.isArray(manifest.conversion.warnings), 'conversion warnings must be an array');

  for (const file of manifest.sourceFiles) {
    assert.ok(file.fileName, 'source fileName is required');
    assert.ok(file.extension?.startsWith('.'), `source extension must start with dot: ${file.fileName}`);
    assert.ok(sourceRoles.has(file.role), `invalid source role ${file.role}`);
  }

  if (manifest.previewStatus === 'ready') {
    assert.ok(manifest.modelPreview?.url, 'ready manifests require modelPreview.url');
    assert.ok(['glb', 'gltf'].includes(manifest.modelPreview.format), 'ready manifests require GLB/GLTF format');
  }
}

function walkFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    return entry.isDirectory() ? walkFiles(fullPath) : [fullPath];
  });
}

const demoManifest = readJson('apps/web/static/redux-previews/demo-rifle/weapon_preview_manifest.json');
validateManifest(demoManifest);
assert.equal(demoManifest.previewStatus, 'converter_not_configured');
assert.equal(demoManifest.modelPreview, null);

validateManifest({
  ...demoManifest,
  previewId: 'ready-glb-fixture',
  sourceKind: 'clean_gta_export',
  previewStatus: 'ready',
  modelPreview: {
    url: 'preview.glb',
    format: 'glb',
    sizeBytes: 128,
    sha256: '0'.repeat(64),
    generatedAt: '2026-06-24T00:00:00.000Z'
  },
  conversion: {
    ...demoManifest.conversion,
    gtaToGlbStatus: 'ready'
  }
});

validateManifest({
  ...demoManifest,
  previewId: 'converter-failed-fixture',
  previewStatus: 'converter_failed',
  conversion: {
    ...demoManifest.conversion,
    gtaToGlbStatus: 'failed',
    warnings: ['converter exited without GLB']
  }
});

validateManifest({
  ...demoManifest,
  previewId: 'reverse-incomplete-fixture',
  conversion: {
    ...demoManifest.conversion,
    glbToGtaStatus: 'incomplete'
  },
  reverseTemplate: {
    ...demoManifest.reverseTemplate,
    validationStatus: 'incomplete'
  }
});

const staticPreviewFiles = walkFiles(path.join(repoRoot, 'apps/web/static/redux-previews'));
const rawStatic = staticPreviewFiles.filter((file) => ['.ydr', '.yft', '.ydd', '.ytd'].includes(path.extname(file).toLowerCase()));
assert.deepEqual(rawStatic, [], 'raw GTA source files must not be under static redux previews');

const frontendFiles = walkFiles(path.join(repoRoot, 'apps/web/src')).filter((file) => /\.(svelte|ts|js)$/.test(file));
const hardcodedWindowsPaths = frontendFiles.filter((file) => /[A-Za-z]:\\/.test(fs.readFileSync(file, 'utf8')));
assert.deepEqual(hardcodedWindowsPaths, [], 'runtime frontend code must not hardcode absolute Windows paths');

console.log('Redux preview manifest tests passed');
