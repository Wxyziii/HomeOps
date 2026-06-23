<svelte:options runes={false} />

<script>
  // @ts-nocheck
  import { onDestroy, onMount } from 'svelte';
  import * as THREE from 'three';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

  export let change;
  export let weaponViewMode = 'firstPerson';

  const demoModelUrl = '/models/redux-demo-weapon.glb';
  const mockTextureSlots = [
    { name: 'diffuse', file: 'weapon_diffuse.ytd slot', status: 'mock mapped' },
    { name: 'normal', file: 'weapon_normal.ytd slot', status: 'mock mapped' },
    { name: 'specular', file: 'weapon_spec.ytd slot', status: 'mock mapped' },
    { name: 'tint', file: 'weapon_tint.ytd slot', status: 'waiting for converter' }
  ];
  const fallbackEntries = {
    carbine: {
      label: 'Carbine Rifle Replacement',
      modelStatus: 'preview_available',
      sourceFormat: '.ydr + .ytd later',
      previewFormat: 'GLB preview',
      source: 'Demo GLB or generated placeholder'
    },
    pistol: {
      label: 'Heavy Pistol Replacement',
      modelStatus: 'waiting_for_converter',
      sourceFormat: '.yft + .ytd later',
      previewFormat: 'GLB preview',
      source: 'Prototype metadata'
    },
    ap: {
      label: 'AP Pistol Replacement',
      modelStatus: 'unavailable',
      sourceFormat: '.ydr + .ytd later',
      previewFormat: 'GLB preview',
      source: 'Prototype metadata'
    }
  };

  let mountEl;
  let fileInput;
  let scene;
  let camera;
  let renderer;
  let controls;
  let loader;
  let resizeObserver;
  let modelRoot;
  let gridHelper;
  let floorMesh;
  let fillLight;
  let keyLight;
  let rimLight;
  let activeObjectUrl = '';
  let activeModelUrl = demoModelUrl;
  let selectedEntryKey = 'carbine';
  let activeTab = 'preview';
  let loading = false;
  let loadProgress = 0;
  let errorText = '';
  let usingPlaceholder = false;
  let showGrid = true;
  let autoRotate = false;
  let lightPreset = 'studio';
  let webglUnavailable = false;
  let rendererReady = false;

  $: modelInfo = {
    ...fallbackEntries[selectedEntryKey],
    label: change?.weapon || fallbackEntries[selectedEntryKey].label,
    model: change?.model || 'Clean tactical replacement',
    title: change?.title || 'Gunpack Model Preview',
    warnings: [
      'converter not connected yet',
      usingPlaceholder ? 'preview asset is generated placeholder' : 'preview asset is demo/simulated'
    ]
  };

  $: if (controls) {
    controls.autoRotate = autoRotate;
  }

  $: if (scene && keyLight) {
    applyLightingPreset(lightPreset);
  }

  onMount(() => {
    initScene();
    if (!webglUnavailable) {
      loadPreviewModel(activeModelUrl);
    }
  });

  onDestroy(() => {
    cleanup();
  });

  function initScene() {
    try {
      scene = new THREE.Scene();
      scene.background = new THREE.Color(0x08090b);
      scene.fog = new THREE.Fog(0x08090b, 8, 26);

      const { width, height } = mountEl.getBoundingClientRect();
      camera = new THREE.PerspectiveCamera(42, Math.max(width, 1) / Math.max(height, 1), 0.01, 100);
      camera.position.set(3.6, 2.1, 5.2);

      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: 'high-performance' });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      renderer.setSize(Math.max(width, 1), Math.max(height, 1));
      renderer.outputColorSpace = THREE.SRGBColorSpace;
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 1.12;
      mountEl.appendChild(renderer.domElement);

      controls = new OrbitControls(camera, renderer.domElement);
      controls.enableDamping = true;
      controls.dampingFactor = 0.08;
      controls.enablePan = true;
      controls.minDistance = 1.2;
      controls.maxDistance = 16;
      controls.target.set(0, 0.65, 0);

      loader = new GLTFLoader();
      addStudio();
      resizeObserver = new ResizeObserver(resizeRenderer);
      resizeObserver.observe(mountEl);
      renderer.setAnimationLoop(renderFrame);
      rendererReady = true;
    } catch (error) {
      webglUnavailable = true;
      errorText = 'WebGL is unavailable in this browser context. 3D preview cannot start.';
      createPlaceholderWithoutRenderer();
    }
  }

  function addStudio() {
    scene.add(new THREE.HemisphereLight(0xb8d7ff, 0x111318, 1.8));

    keyLight = new THREE.DirectionalLight(0xffffff, 3.2);
    keyLight.position.set(4, 7, 4);
    scene.add(keyLight);

    fillLight = new THREE.DirectionalLight(0x7db7ff, 1.2);
    fillLight.position.set(-5, 3.5, 2);
    scene.add(fillLight);

    rimLight = new THREE.DirectionalLight(0xa9c8ff, 2);
    rimLight.position.set(0, 3.5, -5);
    scene.add(rimLight);

    floorMesh = new THREE.Mesh(
      new THREE.CircleGeometry(7.5, 96),
      new THREE.MeshStandardMaterial({
        color: 0x0d1115,
        roughness: 0.82,
        metalness: 0.08
      })
    );
    floorMesh.rotation.x = -Math.PI / 2;
    floorMesh.position.y = -0.02;
    scene.add(floorMesh);

    gridHelper = new THREE.GridHelper(12, 32, 0x3a78b8, 0x1e2935);
    gridHelper.material.transparent = true;
    gridHelper.material.opacity = 0.42;
    scene.add(gridHelper);
  }

  function applyLightingPreset(preset) {
    if (preset === 'inspection') {
      keyLight.intensity = 4.1;
      fillLight.intensity = 2.2;
      rimLight.intensity = 1.2;
      renderer.toneMappingExposure = 1.22;
      return;
    }

    keyLight.intensity = 3.2;
    fillLight.intensity = 1.2;
    rimLight.intensity = 2;
    renderer.toneMappingExposure = 1.12;
  }

  function renderFrame() {
    if (!renderer || !scene || !camera) return;
    controls?.update();
    renderer.render(scene, camera);
  }

  function resizeRenderer() {
    if (!mountEl || !renderer || !camera) return;
    const { width, height } = mountEl.getBoundingClientRect();
    const nextWidth = Math.max(width, 1);
    const nextHeight = Math.max(height, 1);
    camera.aspect = nextWidth / nextHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(nextWidth, nextHeight, false);
  }

  function loadPreviewModel(url) {
    if (!loader || !scene || !url) {
      createPlaceholderModel();
      return;
    }

    loading = true;
    loadProgress = 0;
    errorText = '';
    usingPlaceholder = false;
    clearModel();

    loader.load(
      url,
      (gltf) => {
        clearModel();
        modelRoot = gltf.scene;
        normalizeMaterials(modelRoot);
        scene.add(modelRoot);
        fitCameraToObject(modelRoot);
        loading = false;
        loadProgress = 100;
      },
      (event) => {
        if (event.total) {
          loadProgress = Math.round((event.loaded / event.total) * 100);
        } else {
          loadProgress = 35;
        }
      },
      () => {
        errorText = 'Demo GLB was not found or could not be loaded. Showing a generated preview-safe placeholder.';
        loading = false;
        createPlaceholderModel();
      }
    );
  }

  function normalizeMaterials(root) {
    root.traverse((node) => {
      if (!node.isMesh) return;
      node.castShadow = true;
      node.receiveShadow = true;
      const materials = Array.isArray(node.material) ? node.material : [node.material];
      materials.forEach((material) => {
        if (!material) return;
        material.envMapIntensity = 0.8;
        material.needsUpdate = true;
      });
    });
  }

  function createPlaceholderModel() {
    if (!scene) return;
    clearModel();
    usingPlaceholder = true;

    const group = new THREE.Group();
    const darkMetal = new THREE.MeshStandardMaterial({ color: 0x2b333d, roughness: 0.46, metalness: 0.72 });
    const blueMetal = new THREE.MeshStandardMaterial({ color: 0x526a82, roughness: 0.38, metalness: 0.64 });
    const accent = new THREE.MeshStandardMaterial({ color: 0x93c5ff, roughness: 0.28, metalness: 0.32 });

    const body = new THREE.Mesh(new THREE.BoxGeometry(2.25, 0.42, 0.46), darkMetal);
    body.position.set(-0.15, 0.82, 0);
    group.add(body);

    const stock = new THREE.Mesh(new THREE.BoxGeometry(0.9, 0.32, 0.38), darkMetal);
    stock.position.set(-1.7, 0.78, 0);
    stock.rotation.z = -0.12;
    group.add(stock);

    const barrel = new THREE.Mesh(new THREE.CylinderGeometry(0.08, 0.08, 1.75, 32), blueMetal);
    barrel.rotation.z = Math.PI / 2;
    barrel.position.set(1.55, 0.88, 0);
    group.add(barrel);

    const rail = new THREE.Mesh(new THREE.BoxGeometry(1.55, 0.08, 0.52), accent);
    rail.position.set(0.05, 1.1, 0);
    group.add(rail);

    const grip = new THREE.Mesh(new THREE.BoxGeometry(0.28, 0.78, 0.32), darkMetal);
    grip.position.set(-0.55, 0.28, 0);
    grip.rotation.z = -0.24;
    group.add(grip);

    const mag = new THREE.Mesh(new THREE.BoxGeometry(0.38, 0.7, 0.34), blueMetal);
    mag.position.set(0.2, 0.23, 0);
    mag.rotation.z = 0.08;
    group.add(mag);

    const sight = new THREE.Mesh(new THREE.BoxGeometry(0.48, 0.28, 0.38), accent);
    sight.position.set(-0.22, 1.36, 0);
    group.add(sight);

    group.rotation.y = -0.32;
    modelRoot = group;
    scene.add(modelRoot);
    fitCameraToObject(modelRoot);
  }

  function createPlaceholderWithoutRenderer() {
    usingPlaceholder = true;
  }

  function fitCameraToObject(object) {
    const box = new THREE.Box3().setFromObject(object);
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z, 1);
    const fov = (camera.fov * Math.PI) / 180;
    const distance = Math.abs(maxDim / Math.sin(fov / 2)) * 0.82;

    object.position.sub(center);
    object.position.y += size.y * 0.5;
    camera.position.set(distance * 0.7, distance * 0.42, distance * 0.92);
    controls.target.set(0, Math.max(size.y * 0.35, 0.45), 0);
    controls.update();
  }

  function clearModel() {
    if (!modelRoot || !scene) return;
    scene.remove(modelRoot);
    disposeObject(modelRoot);
    modelRoot = null;
  }

  function disposeObject(object) {
    object.traverse((node) => {
      if (node.geometry) node.geometry.dispose();
      if (node.material) {
        const materials = Array.isArray(node.material) ? node.material : [node.material];
        materials.forEach((material) => {
          Object.values(material).forEach((value) => {
            if (value?.isTexture) value.dispose();
          });
          material.dispose();
        });
      }
    });
  }

  function resetCamera() {
    if (modelRoot) {
      fitCameraToObject(modelRoot);
      return;
    }

    camera?.position.set(3.6, 2.1, 5.2);
    controls?.target.set(0, 0.65, 0);
    controls?.update();
  }

  function toggleGrid() {
    showGrid = !showGrid;
    if (gridHelper) gridHelper.visible = showGrid;
    if (floorMesh) floorMesh.visible = showGrid;
  }

  function toggleAutoRotate() {
    autoRotate = !autoRotate;
  }

  function toggleLighting() {
    lightPreset = lightPreset === 'studio' ? 'inspection' : 'studio';
  }

  function openFilePicker() {
    fileInput?.click();
  }

  function handleModelFile(event) {
    const file = event.currentTarget.files?.[0];
    if (!file) return;

    if (!file.name.toLowerCase().match(/\.(glb|gltf)$/)) {
      errorText = 'Only preview-safe .glb or .gltf files can be loaded in this phase.';
      event.currentTarget.value = '';
      return;
    }

    if (activeObjectUrl) URL.revokeObjectURL(activeObjectUrl);
    activeObjectUrl = URL.createObjectURL(file);
    activeModelUrl = activeObjectUrl;
    loadPreviewModel(activeModelUrl);
    event.currentTarget.value = '';
  }

  function cleanup() {
    resizeObserver?.disconnect();
    if (activeObjectUrl) URL.revokeObjectURL(activeObjectUrl);
    renderer?.setAnimationLoop(null);
    controls?.dispose();
    clearModel();

    if (gridHelper) {
      scene?.remove(gridHelper);
      gridHelper.geometry?.dispose();
      gridHelper.material?.dispose();
    }

    if (floorMesh) {
      scene?.remove(floorMesh);
      floorMesh.geometry?.dispose();
      floorMesh.material?.dispose();
    }

    renderer?.dispose();
    renderer?.domElement?.remove();
  }
</script>

<div class="weapon-viewer" data-mode={weaponViewMode}>
  <div bind:this={mountEl} class="viewer-canvas" aria-label="Interactive 3D weapon showroom"></div>

  <div class="showroom-top">
    <div>
      <span class="eyebrow">Redux Maker showroom</span>
      <h2>{modelInfo.label}</h2>
    </div>
    <span class:warn={usingPlaceholder || errorText} class="status-badge">
      {loading ? `Loading ${loadProgress}%` : usingPlaceholder ? 'Placeholder preview' : 'Demo GLB ready'}
    </span>
  </div>

  <div class="phase-badge">3D preview shell - real GTA conversion not connected yet</div>

  <div class="tab-row" aria-label="Viewer mode">
    <button class:active={activeTab === 'preview'} type="button" on:click={() => (activeTab = 'preview')}>Preview</button>
    <button class:active={activeTab === 'textures'} type="button" on:click={() => (activeTab = 'textures')}>Textures</button>
    <button class:active={activeTab === 'details'} type="button" on:click={() => (activeTab = 'details')}>Details</button>
  </div>

  <section class="showroom-panel" aria-label="Weapon model information">
    {#if activeTab === 'preview'}
      <p class="panel-title">{modelInfo.title}</p>
      <dl>
        <div><dt>Model</dt><dd>{modelInfo.model}</dd></div>
        <div><dt>Source</dt><dd>{modelInfo.source}</dd></div>
        <div><dt>Format</dt><dd>{modelInfo.previewFormat}</dd></div>
        <div><dt>Status</dt><dd>Preview only</dd></div>
      </dl>
    {:else if activeTab === 'textures'}
      <p class="panel-title">Material slots</p>
      <div class="slot-list">
        {#each mockTextureSlots as slot}
          <div class="texture-slot">
            <span class="slot-swatch"></span>
            <div>
              <strong>{slot.name}</strong>
              <small>{slot.file} - {slot.status}</small>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="panel-title">Pipeline details</p>
      <dl>
        <div><dt>Source format</dt><dd>{modelInfo.sourceFormat}</dd></div>
        <div><dt>Model status</dt><dd>{modelInfo.modelStatus}</dd></div>
        <div><dt>Conversion</dt><dd>No GTA model conversion in this phase</dd></div>
      </dl>
      <ul class="warning-list">
        {#each modelInfo.warnings as warning}
          <li>{warning}</li>
        {/each}
      </ul>
    {/if}
  </section>

  <div class="weapon-status">
    <span>Ready for converted weapon models</span>
    <span>GLB/GLTF only</span>
    <span>No RPF scanning</span>
  </div>

  <div class="showroom-controls" aria-label="3D viewer controls">
    <button type="button" on:click={resetCamera}>Reset view</button>
    <button class:active={showGrid} type="button" on:click={toggleGrid}>Grid</button>
    <button class:active={autoRotate} type="button" on:click={toggleAutoRotate}>Auto-rotate</button>
    <button type="button" on:click={toggleLighting}>{lightPreset === 'studio' ? 'Studio light' : 'Inspection light'}</button>
    <button type="button" on:click={openFilePicker}>Load GLB</button>
  </div>

  <div class="weapon-view-toggle" aria-label="Mock metadata set">
    <button class:active={selectedEntryKey === 'carbine'} type="button" on:click={() => (selectedEntryKey = 'carbine')}>Carbine</button>
    <button class:active={selectedEntryKey === 'pistol'} type="button" on:click={() => (selectedEntryKey = 'pistol')}>Heavy pistol</button>
    <button class:active={selectedEntryKey === 'ap'} type="button" on:click={() => (selectedEntryKey = 'ap')}>AP pistol</button>
  </div>

  <input
    bind:this={fileInput}
    class="model-input"
    type="file"
    accept=".glb,.gltf,model/gltf-binary,model/gltf+json"
    on:change={handleModelFile}
  />

  {#if loading}
    <div class="viewer-state">
      <strong>Loading preview-safe model</strong>
      <span>{loadProgress}%</span>
    </div>
  {/if}

  {#if errorText}
    <div class="viewer-error">
      <strong>Preview notice</strong>
      <span>{errorText}</span>
    </div>
  {/if}

  {#if webglUnavailable}
    <div class="viewer-empty">
      <strong>3D viewer unavailable</strong>
      <span>WebGL did not initialize. No backend or converter was called.</span>
    </div>
  {:else if rendererReady && !loading && !modelRoot}
    <div class="viewer-empty">
      <strong>No model selected</strong>
      <span>Load a local .glb/.gltf or use the generated preview shell.</span>
    </div>
  {/if}
</div>

<style>
  .weapon-viewer {
    position: absolute;
    inset: 0;
    container-type: inline-size;
    overflow: hidden;
    background:
      radial-gradient(circle at 50% 24%, rgba(90, 130, 180, 0.16), transparent 28%),
      linear-gradient(180deg, #0b0d10, #050608 62%, #0a0b0d);
    transition: transform 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .viewer-canvas {
    position: absolute;
    inset: 0;
  }

  .viewer-canvas :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }

  .viewer-canvas :global(canvas:active) {
    cursor: grabbing;
  }

  .showroom-top,
  .phase-badge,
  .tab-row,
  .showroom-panel,
  .showroom-controls,
  .weapon-status,
  .weapon-view-toggle,
  .viewer-state,
  .viewer-error,
  .viewer-empty {
    position: absolute;
    z-index: 5;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(10, 12, 15, 0.68);
    color: var(--text, #ededed);
    backdrop-filter: blur(12px);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.28);
  }

  .showroom-top {
    top: 58px;
    left: 22px;
    right: 22px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 13px 14px;
    border-radius: 12px;
  }

  .eyebrow {
    display: block;
    margin-bottom: 4px;
    color: var(--text-3, #737373);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h2 {
    margin: 0;
    color: #f5f5f5;
    font-size: 16px;
    font-weight: 600;
    line-height: 1.25;
  }

  .status-badge,
  .phase-badge {
    border-radius: 999px;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .status-badge {
    padding: 6px 9px;
    background: rgba(71, 209, 108, 0.12);
    color: #76e69a;
  }

  .status-badge.warn {
    background: rgba(217, 163, 79, 0.12);
    color: #ffcf74;
  }

  .phase-badge {
    left: 22px;
    top: 132px;
    max-width: calc(100% - 44px);
    padding: 7px 10px;
    color: #d8d8d8;
  }

  .tab-row {
    top: 178px;
    left: 22px;
    display: inline-flex;
    gap: 4px;
    padding: 5px;
    border-radius: 10px;
  }

  .tab-row button,
  .showroom-controls button,
  .weapon-view-toggle button {
    min-height: 30px;
    padding: 6px 9px;
    border-radius: 7px;
    color: var(--text-2, #a1a1a1);
    cursor: pointer;
  }

  .tab-row button:hover,
  .showroom-controls button:hover,
  .weapon-view-toggle button:hover,
  .tab-row button.active,
  .showroom-controls button.active,
  .weapon-view-toggle button.active {
    background: rgba(255, 255, 255, 0.11);
    color: #fff;
  }

  .showroom-panel {
    left: 22px;
    bottom: 152px;
    width: min(300px, calc(100% - 44px));
    display: grid;
    gap: 10px;
    padding: 14px;
    border-radius: 12px;
  }

  .panel-title {
    margin: 0;
    color: #f1f1f1;
    font-size: 13px;
    font-weight: 600;
  }

  dl {
    display: grid;
    gap: 8px;
    margin: 0;
  }

  dl div {
    display: grid;
    grid-template-columns: 86px minmax(0, 1fr);
    gap: 8px;
  }

  dt {
    color: var(--text-3, #737373);
    font-size: 12px;
  }

  dd {
    min-width: 0;
    margin: 0;
    color: var(--text-2, #a1a1a1);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .slot-list {
    display: grid;
    gap: 8px;
  }

  .texture-slot {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    align-items: center;
  }

  .slot-swatch {
    width: 18px;
    height: 18px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    background: linear-gradient(135deg, #6a7b8d, #20262d);
  }

  .texture-slot strong {
    display: block;
    color: #f1f1f1;
    font-size: 12px;
    font-weight: 600;
  }

  .texture-slot small {
    display: block;
    color: var(--text-3, #737373);
    font-size: 11px;
    line-height: 1.35;
  }

  .warning-list {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
    color: #ffcf74;
    font-size: 12px;
  }

  .warning-list li::before {
    content: "";
    display: inline-block;
    width: 5px;
    height: 5px;
    margin-right: 8px;
    border-radius: 50%;
    background: currentColor;
    vertical-align: middle;
  }

  .weapon-status {
    right: 22px;
    bottom: 152px;
    display: grid;
    gap: 8px;
    padding: 14px;
    border-radius: 12px;
    color: #76e69a;
    font-size: 12px;
  }

  .weapon-status span::before {
    content: "";
    display: inline-block;
    width: 8px;
    height: 5px;
    margin-right: 8px;
    border-left: 2px solid currentColor;
    border-bottom: 2px solid currentColor;
    transform: rotate(-45deg) translateY(-2px);
  }

  .showroom-controls {
    left: 50%;
    bottom: 78px;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 5px;
    width: min(520px, calc(100% - 44px));
    padding: 6px;
    border-radius: 12px;
    transform: translateX(-50%);
  }

  .weapon-view-toggle {
    right: 22px;
    top: 178px;
    display: flex;
    gap: 4px;
    padding: 5px;
    border-radius: 10px;
  }

  .model-input {
    display: none;
  }

  .viewer-state,
  .viewer-error,
  .viewer-empty {
    left: 50%;
    top: 50%;
    display: grid;
    gap: 6px;
    width: min(360px, calc(100% - 56px));
    padding: 14px;
    border-radius: 12px;
    transform: translate(-50%, -50%);
    text-align: center;
  }

  .viewer-state strong,
  .viewer-error strong,
  .viewer-empty strong {
    color: #f5f5f5;
    font-size: 13px;
  }

  .viewer-state span,
  .viewer-error span,
  .viewer-empty span {
    color: var(--text-3, #737373);
    font-size: 12px;
    line-height: 1.4;
  }

  .viewer-error {
    top: auto;
    right: 22px;
    bottom: 226px;
    left: auto;
    width: min(320px, calc(100% - 44px));
    transform: none;
    text-align: left;
    border-color: rgba(217, 163, 79, 0.22);
  }

  @media (max-width: 720px) {
    .showroom-top {
      top: 54px;
    }

    .weapon-view-toggle {
      right: auto;
      left: 22px;
      top: 222px;
      max-width: calc(100% - 44px);
      overflow-x: auto;
    }

    .showroom-panel,
    .weapon-status {
      bottom: 146px;
    }

    .weapon-status {
      display: none;
    }

    .showroom-controls {
      bottom: 76px;
    }
  }

  @container (max-width: 520px) {
    .weapon-status {
      display: none;
    }

    .showroom-panel {
      width: min(300px, calc(100% - 44px));
    }

    .showroom-controls {
      width: min(360px, calc(100% - 44px));
    }
  }
</style>
