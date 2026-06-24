<svelte:options runes={false} />

<script>
  // @ts-nocheck
  import { onDestroy, onMount } from 'svelte';
  import * as THREE from 'three';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

  export let change;
  export let weaponViewMode = 'firstPerson';
  export let manifestUrl = '/redux-previews/demo-rifle/weapon_preview_manifest.json';

  const mockMaterialSlots = [
    { slotName: 'diffuse', textureName: 'weapon_diffuse', status: 'converter not connected', notes: 'Fallback slot' },
    { slotName: 'normal', textureName: 'weapon_normal', status: 'converter not connected', notes: 'Fallback slot' },
    { slotName: 'specular', textureName: 'weapon_spec', status: 'converter not connected', notes: 'Fallback slot' },
    { slotName: 'tint', textureName: 'weapon_tint', status: 'waiting for converter', notes: 'Fallback slot' }
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
  let activeModelUrl = '';
  let manifest = null;
  let manifestLoading = false;
  let manifestError = '';
  let loadedManifestUrl = '';
  let manualModelName = '';
  let selectedEntryKey = 'carbine';
  let activeTab = 'preview';
  let loading = false;
  let loadProgress = 0;
  let errorText = '';
  let usingPlaceholder = false;
  let showGrid = true;
  let autoRotate = false;
  let focusMode = false;
  let showInspector = false;
  let showViewerTools = false;
  let lightPreset = 'studio';
  let webglUnavailable = false;
  let rendererReady = false;
  let activeKeyHint = '';
  let keyHintTimer;
  let mouseLookActive = false;
  let yaw = 0;
  let pitch = 0;
  let lastFrameTime = 0;
  const pressedKeys = new Set();

  $: sourceFiles = manifest?.sourceFiles ?? [];
  $: modelPreview = manifest?.modelPreview ?? manifest?.previewModel ?? null;
  $: textureDictionaries = manifest?.textureDictionaries?.length
    ? manifest.textureDictionaries
    : sourceFiles
        .filter((file) => file.extension?.toLowerCase?.() === '.ytd')
        .map((file) => ({ fileName: file.fileName, textureNames: [], status: 'texture mapping not available yet' }));
  $: materialSlots = manifest?.materialSlots?.length
    ? manifest.materialSlots
    : manifest?.textureSlots?.length
      ? manifest.textureSlots
      : mockMaterialSlots;
  $: conversion = manifest?.conversion ?? {
    gtaToGlbStatus: 'not_configured',
    glbToGtaStatus: 'unsupported',
    converterName: null,
    converterVersion: null,
    logsPath: null,
    warnings: []
  };
  $: reverseTemplate = manifest?.reverseTemplate ?? {
    templateWeaponName: manifest?.weaponPrefix ?? '',
    originalSourceFiles: [],
    requiredFiles: [],
    stagedOutputDir: null,
    validationStatus: 'unsupported'
  };
  $: isRealConvertedModel = Boolean(
    modelPreview?.url &&
      conversion?.gtaToGlbStatus === 'ready' &&
      ['clean_gta_export', 'gunpack_export'].includes(manifest?.sourceKind)
  );
  $: manifestWarnings = [
    ...(manifest?.warnings ?? []),
    ...(conversion?.warnings ?? []),
    ...(manifestError ? [manifestError] : []),
    ...(usingPlaceholder && !manifest?.warnings?.some((warning) => warning.toLowerCase().includes('placeholder'))
      ? ['Showing procedural preview-safe placeholder.']
      : [])
  ];
  $: modelInfo = {
    ...fallbackEntries[selectedEntryKey],
    label: manifest?.weaponName || change?.weapon || fallbackEntries[selectedEntryKey].label,
    model: manifest?.displayName || change?.model || 'Clean tactical replacement',
    title: manifest?.displayName || change?.title || 'Gunpack Model Preview',
    source: manifest?.sourceLabel || fallbackEntries[selectedEntryKey].source,
    sourceKind: manifest?.sourceKind || 'demo_placeholder',
    sourceFormat: sourceFiles.length
      ? [...new Set(sourceFiles.map((file) => file.extension).filter(Boolean))].join(' + ')
      : fallbackEntries[selectedEntryKey].sourceFormat,
    previewFormat: modelPreview?.format?.toUpperCase?.() || (usingPlaceholder ? 'Procedural placeholder' : 'GLB/GLTF preview'),
    modelStatus: manifest?.previewStatus || fallbackEntries[selectedEntryKey].modelStatus,
    warnings: manifestWarnings
  };
  $: phaseMessage = manifestLoading
    ? 'Loading weapon preview manifest'
    : isRealConvertedModel
      ? 'Real converted GTA model'
      : manifest?.previewStatus === 'ready' && modelPreview?.url
        ? 'Manifest GLB/GLTF preview ready'
      : manifest?.previewStatus === 'unsupported_source_format'
        ? 'GTA source format unsupported in browser - GLB/GLTF required'
        : manifest?.previewStatus === 'converter_not_configured' || manifest?.previewStatus === 'converter_not_connected'
          ? 'Converter not configured - showing demo placeholder.'
          : manifest?.previewStatus === 'converter_failed'
            ? 'Converter failed - showing preview-safe fallback.'
          : manifestError
            ? 'Manifest unavailable - showing demo placeholder.'
            : '3D preview shell - real GTA conversion not connected yet';
  $: statusLabel = loading
    ? `Loading ${loadProgress}%`
    : manifestLoading
      ? 'Loading manifest'
      : manualModelName
        ? 'Manual GLB ready'
        : isRealConvertedModel
          ? 'Real GTA GLB'
        : usingPlaceholder
          ? 'Placeholder preview'
        : manifest?.previewStatus === 'ready'
            ? 'Manifest GLB ready'
            : manifest?.previewStatus
              ? manifest.previewStatus.replaceAll('_', ' ')
              : 'Preview ready';
  $: showMockWeaponPicker = !manifest || manifest?.sourceKind === 'demo_placeholder';
  $: compactStats = [
    ['Preview', cleanStatus(modelInfo.modelStatus)],
    ['GTA->GLB', cleanStatus(conversion.gtaToGlbStatus)],
    ['GLB->GTA', cleanStatus(conversion.glbToGtaStatus)]
  ];

  $: if (controls) {
    controls.autoRotate = autoRotate;
  }

  $: if (scene && keyLight) {
    applyLightingPreset(lightPreset);
  }

  onMount(() => {
    initScene();
    if (!webglUnavailable) {
      loadManifest(manifestUrl);
    }
  });

  onDestroy(() => {
    cleanup();
  });

  $: if (rendererReady && manifestUrl && manifestUrl !== loadedManifestUrl && !manualModelName) {
    loadManifest(manifestUrl);
  }

  function initScene() {
    try {
      scene = new THREE.Scene();
      scene.background = new THREE.Color(0x08090b);

      const { width, height } = mountEl.getBoundingClientRect();
      camera = new THREE.PerspectiveCamera(42, Math.max(width, 1) / Math.max(height, 1), 0.01, 100);
      camera.position.set(3.6, 2.1, 5.2);

      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: 'high-performance' });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      renderer.setSize(Math.max(width, 1), Math.max(height, 1));
      renderer.domElement.style.width = '100%';
      renderer.domElement.style.height = '100%';
      renderer.outputColorSpace = THREE.SRGBColorSpace;
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 0.82;
      mountEl.appendChild(renderer.domElement);

      controls = new OrbitControls(camera, renderer.domElement);
      controls.enableDamping = true;
      controls.dampingFactor = 0.08;
      controls.enablePan = true;
      controls.enableRotate = false;
      controls.minDistance = 0.6;
      controls.maxDistance = 60;
      controls.zoomSpeed = 1.15;
      controls.target.set(0, 0.65, 0);
      syncLookAnglesFromCamera();

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

  async function loadManifest(url) {
    loadedManifestUrl = url || '';
    manifestLoading = true;
    manifestError = '';
    manualModelName = '';

    if (!url) {
      manifest = null;
      manifestLoading = false;
      errorText = 'No preview manifest was provided. Showing a generated preview-safe placeholder.';
      createPlaceholderModel();
      return;
    }

    try {
      const response = await fetch(url, { cache: 'no-store' });
      if (!response.ok) {
        throw new Error(`Manifest request failed with ${response.status}`);
      }

      const nextManifest = await response.json();
      manifest = normalizeManifest(nextManifest);
      manifestLoading = false;

      const preview = manifest?.modelPreview ?? manifest?.previewModel ?? null;
      const previewUrl = resolveManifestAssetUrl(url, preview?.url);
      const format = preview?.format?.toLowerCase?.();
      if (previewUrl && ['glb', 'gltf'].includes(format)) {
        activeModelUrl = previewUrl;
        loadPreviewModel(activeModelUrl, 'manifest');
        return;
      }

      activeModelUrl = '';
      errorText =
        manifest?.previewStatus === 'converter_not_configured' ||
        manifest?.previewStatus === 'converter_not_connected' ||
        manifest?.sourceKind === 'demo_placeholder'
          ? 'Converter not configured - showing demo placeholder.'
          : manifest?.previewStatus === 'converter_failed'
            ? 'Converter failed before producing a GLB/GLTF. Showing a generated preview-safe placeholder.'
          : 'No GLB/GLTF preview model is attached to this manifest. Showing a generated placeholder.';
      createPlaceholderModel();
    } catch (error) {
      manifest = null;
      manifestLoading = false;
      manifestError = `Preview manifest could not be loaded: ${error?.message ?? 'unknown error'}`;
      errorText = `${manifestError} Showing a generated preview-safe placeholder.`;
      createPlaceholderModel();
    }
  }

  function normalizeManifest(value) {
    return {
      manifestVersion: value?.manifestVersion ?? '1.0.0',
      previewId: value?.previewId ?? 'unknown-preview',
      displayName: value?.displayName ?? change?.title ?? 'Weapon Model Preview',
      weaponName: value?.weaponName ?? change?.weapon ?? 'Selected weapon',
      weaponPrefix: value?.weaponPrefix ?? null,
      sourceLabel: value?.sourceLabel ?? 'Unknown source',
      sourceKind: value?.sourceKind ?? 'demo_placeholder',
      previewStatus: value?.previewStatus ?? 'converter_not_configured',
      modelPreview: value?.modelPreview ?? value?.previewModel ?? null,
      previewModel: value?.previewModel ?? value?.modelPreview ?? null,
      sourceFiles: Array.isArray(value?.sourceFiles) ? value.sourceFiles : [],
      textureDictionaries: Array.isArray(value?.textureDictionaries) ? value.textureDictionaries : [],
      materialSlots: Array.isArray(value?.materialSlots)
        ? value.materialSlots
        : Array.isArray(value?.textureSlots)
          ? value.textureSlots
          : [],
      textureSlots: Array.isArray(value?.textureSlots)
        ? value.textureSlots
        : Array.isArray(value?.materialSlots)
          ? value.materialSlots
          : [],
      conversion: {
        gtaToGlbStatus: value?.conversion?.gtaToGlbStatus ?? 'not_configured',
        glbToGtaStatus: value?.conversion?.glbToGtaStatus ?? 'unsupported',
        converterName: value?.conversion?.converterName ?? null,
        converterVersion: value?.conversion?.converterVersion ?? null,
        logsPath: value?.conversion?.logsPath ?? null,
        warnings: Array.isArray(value?.conversion?.warnings) ? value.conversion.warnings : []
      },
      reverseTemplate: {
        templateWeaponName: value?.reverseTemplate?.templateWeaponName ?? value?.weaponPrefix ?? null,
        originalSourceFiles: Array.isArray(value?.reverseTemplate?.originalSourceFiles)
          ? value.reverseTemplate.originalSourceFiles
          : [],
        requiredFiles: Array.isArray(value?.reverseTemplate?.requiredFiles) ? value.reverseTemplate.requiredFiles : [],
        stagedOutputDir: value?.reverseTemplate?.stagedOutputDir ?? null,
        validationStatus: value?.reverseTemplate?.validationStatus ?? 'unsupported'
      },
      warnings: Array.isArray(value?.warnings) ? value.warnings : [],
      notes: Array.isArray(value?.notes) ? value.notes : [],
      buildReady: false
    };
  }

  function resolveManifestAssetUrl(baseManifestUrl, assetUrl) {
    if (!assetUrl) return '';
    if (/^(blob:|https?:\/\/|\/)/i.test(assetUrl)) return assetUrl;

    try {
      return new URL(assetUrl, new URL(baseManifestUrl, window.location.origin)).toString();
    } catch {
      return assetUrl;
    }
  }

  function cleanStatus(value) {
    return value ? String(value).replaceAll('_', ' ') : 'unknown';
  }

  function formatBytes(value) {
    if (!Number.isFinite(value)) return '';
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${(value / 1024 / 1024).toFixed(2)} MB`;
  }

  function addStudio() {
    scene.add(new THREE.HemisphereLight(0xd8e6ff, 0x111318, 0.92));

    keyLight = new THREE.DirectionalLight(0xffffff, 1.55);
    keyLight.position.set(4, 7, 4);
    scene.add(keyLight);

    fillLight = new THREE.DirectionalLight(0x8fbfff, 0.54);
    fillLight.position.set(-5, 3.5, 2);
    scene.add(fillLight);

    rimLight = new THREE.DirectionalLight(0xa9c8ff, 0.92);
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
      keyLight.intensity = 2.1;
      fillLight.intensity = 0.9;
      rimLight.intensity = 0.65;
      renderer.toneMappingExposure = 0.92;
      return;
    }

    keyLight.intensity = 1.55;
    fillLight.intensity = 0.54;
    rimLight.intensity = 0.92;
    renderer.toneMappingExposure = 0.82;
  }

  function renderFrame() {
    if (!renderer || !scene || !camera) return;
    updateHeldKeyCamera();
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
    renderer.domElement.style.width = '100%';
    renderer.domElement.style.height = '100%';
  }

  function showKeyHint(message) {
    activeKeyHint = message;
    clearTimeout(keyHintTimer);
    keyHintTimer = setTimeout(() => {
      activeKeyHint = '';
    }, 1100);
  }

  function getTarget() {
    return controls?.target ?? new THREE.Vector3(0, 0.65, 0);
  }

  function orbitCamera(thetaDelta, phiDelta) {
    if (!camera || !controls) return;
    const target = getTarget();
    const offset = camera.position.clone().sub(target);
    const spherical = new THREE.Spherical().setFromVector3(offset);
    spherical.theta += thetaDelta;
    spherical.phi = Math.max(0.08, Math.min(Math.PI - 0.08, spherical.phi + phiDelta));
    camera.position.copy(target).add(new THREE.Vector3().setFromSpherical(spherical));
    camera.lookAt(target);
    controls.update();
  }

  function panCamera(xDelta, yDelta) {
    if (!camera || !controls) return;
    const target = getTarget();
    const distance = Math.max(camera.position.distanceTo(target), 1);
    const scale = distance * 0.045;
    const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0).multiplyScalar(xDelta * scale);
    const up = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 1).multiplyScalar(yDelta * scale);
    const pan = right.add(up);
    camera.position.add(pan);
    controls.target.add(pan);
    controls.update();
  }

  function moveCamera(xDelta, zDelta) {
    if (!camera || !controls) return;
    const target = getTarget();
    const distance = Math.max(camera.position.distanceTo(target), 1);
    const scale = distance * 0.075;
    const forward = target.clone().sub(camera.position).normalize().multiplyScalar(zDelta * scale);
    const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0).normalize().multiplyScalar(xDelta * scale);
    const move = right.add(forward);
    camera.position.add(move);
    controls.target.add(move);
    controls.update();
  }

  function syncLookAnglesFromCamera() {
    if (!camera || !controls) return;
    const direction = controls.target.clone().sub(camera.position).normalize();
    yaw = Math.atan2(direction.x, direction.z);
    pitch = Math.asin(Math.max(-0.98, Math.min(0.98, direction.y)));
  }

  function applyLookAngles() {
    if (!camera || !controls) return;
    const forward = new THREE.Vector3(
      Math.sin(yaw) * Math.cos(pitch),
      Math.sin(pitch),
      Math.cos(yaw) * Math.cos(pitch)
    ).normalize();
    const distance = Math.max(camera.position.distanceTo(controls.target), 1);
    controls.target.copy(camera.position).add(forward.multiplyScalar(distance));
  }

  function moveFpsCamera(deltaSeconds = 1 / 60) {
    if (!camera || !controls) return;
    const fast = pressedKeys.has('shift');
    const speed = (fast ? 3.4 : 1.45) * deltaSeconds;
    const forward = new THREE.Vector3();
    camera.getWorldDirection(forward);
    forward.y = 0;
    if (forward.lengthSq() < 0.0001) forward.set(0, 0, -1);
    forward.normalize();
    const right = new THREE.Vector3().crossVectors(forward, camera.up).normalize();
    const movement = new THREE.Vector3();

    if (pressedKeys.has('w')) movement.add(forward);
    if (pressedKeys.has('s')) movement.sub(forward);
    if (pressedKeys.has('d')) movement.add(right);
    if (pressedKeys.has('a')) movement.sub(right);
    if (pressedKeys.has('e')) movement.y += 1;
    if (pressedKeys.has('q')) movement.y -= 1;

    if (movement.lengthSq() > 0) {
      movement.normalize().multiplyScalar(speed);
      camera.position.add(movement);
      controls.target.add(movement);
      controls.update();
    }
  }

  function dollyCamera(multiplier) {
    if (!camera || !controls) return;
    const target = getTarget();
    const offset = camera.position.clone().sub(target);
    const nextDistance = Math.max(controls.minDistance, Math.min(controls.maxDistance, offset.length() * multiplier));
    offset.setLength(nextDistance);
    camera.position.copy(target).add(offset);
    controls.update();
  }

  function snapCameraView(view) {
    if (!camera || !controls) return;
    const target = getTarget();
    const distance = Math.max(camera.position.distanceTo(target), 5);
    const positions = {
      front: [0, 0.3, distance],
      back: [0, 0.3, -distance],
      right: [distance, 0.3, 0],
      left: [-distance, 0.3, 0],
      top: [0, distance, 0.01],
      bottom: [0, -distance, 0.01]
    };
    const next = positions[view];
    if (!next) return;
    camera.position.set(target.x + next[0], target.y + next[1], target.z + next[2]);
    camera.lookAt(target);
    syncLookAnglesFromCamera();
    controls.update();
    showKeyHint(`${view} view`);
  }

  function handleViewerPointerDown(event) {
    mountEl?.focus?.();
    if (!camera || !controls || event.button !== 0) return;
    mouseLookActive = true;
    syncLookAnglesFromCamera();
    mountEl?.setPointerCapture?.(event.pointerId);
    event.preventDefault();
    event.stopPropagation();
  }

  function handleViewerPointerMove(event) {
    if (!mouseLookActive || !camera || !controls) return;
    const sensitivity = 0.0032;
    yaw -= event.movementX * sensitivity;
    pitch = Math.max(-1.35, Math.min(1.35, pitch - event.movementY * sensitivity));
    applyLookAngles();
    controls.update();
    event.preventDefault();
  }

  function handleViewerPointerUp(event) {
    if (!mouseLookActive) return;
    mouseLookActive = false;
    mountEl?.releasePointerCapture?.(event.pointerId);
    event.preventDefault();
  }

  function handleViewerWheel(event) {
    if (!event.shiftKey) return;
    const direction = event.deltaY > 0 ? 1 : -1;
    camera.position.y += direction * 0.08;
    controls.target.y += direction * 0.08;
    controls.update();
    event.preventDefault();
  }

  function updateHeldKeyCamera() {
    if (!camera || !controls || !pressedKeys.size) return;
    const now = performance.now();
    const deltaSeconds = lastFrameTime ? Math.min((now - lastFrameTime) / 1000, 0.05) : 1 / 60;
    lastFrameTime = now;
    moveFpsCamera(deltaSeconds);
  }

  function handleViewerKeydown(event) {
    if (!camera || !controls) return;
    const key = event.key.toLowerCase();
    const orbitStep = event.altKey ? 0.34 : 0.16;
    const panStep = event.altKey ? 2.2 : 1;
    let handled = true;

    if (!event.ctrlKey && !event.metaKey && ['w', 'a', 's', 'd', 'q', 'e'].includes(key)) {
      pressedKeys.add(key);
      if (event.shiftKey) pressedKeys.add('shift');
      showKeyHint('WASD move + mouse look');
    } else if (event.shiftKey && key === 'arrowleft') {
      panCamera(1 * panStep, 0);
      showKeyHint('pan left');
    } else if (event.shiftKey && key === 'arrowright') {
      panCamera(-1 * panStep, 0);
      showKeyHint('pan right');
    } else if (event.shiftKey && key === 'arrowup') {
      panCamera(0, -1 * panStep);
      showKeyHint('pan up');
    } else if (event.shiftKey && key === 'arrowdown') {
      panCamera(0, 1 * panStep);
      showKeyHint('pan down');
    } else if ((event.ctrlKey || event.metaKey) && key === 'arrowup') {
      dollyCamera(0.86);
      showKeyHint('zoom in');
    } else if ((event.ctrlKey || event.metaKey) && key === 'arrowdown') {
      dollyCamera(1.16);
      showKeyHint('zoom out');
    } else if (key === 'arrowleft') {
      orbitCamera(-orbitStep, 0);
      showKeyHint('orbit left');
    } else if (key === 'arrowright') {
      orbitCamera(orbitStep, 0);
      showKeyHint('orbit right');
    } else if (key === 'arrowup') {
      orbitCamera(0, -orbitStep);
      showKeyHint('orbit up');
    } else if (key === 'arrowdown') {
      orbitCamera(0, orbitStep);
      showKeyHint('orbit down');
    } else if (key === '+' || key === '=' || key === 'add') {
      dollyCamera(0.86);
      showKeyHint('zoom in');
    } else if (key === '-' || key === '_' || key === 'subtract') {
      dollyCamera(1.16);
      showKeyHint('zoom out');
    } else if (key === '1') {
      snapCameraView(event.ctrlKey || event.metaKey ? 'back' : 'front');
    } else if (key === '3') {
      snapCameraView(event.ctrlKey || event.metaKey ? 'left' : 'right');
    } else if (key === '7') {
      snapCameraView(event.ctrlKey || event.metaKey ? 'bottom' : 'top');
    } else if (key === 'f' || key === 'home') {
      resetCamera();
      showKeyHint('frame model');
    } else {
      handled = false;
    }

    if (handled) {
      event.preventDefault();
      event.stopPropagation();
    }
  }

  function handleViewerKeyup(event) {
    const key = event.key.toLowerCase();
    if (['w', 'a', 's', 'd', 'q', 'e'].includes(key)) {
      pressedKeys.delete(key);
      event.preventDefault();
      event.stopPropagation();
    }

    if (key === 'shift') {
      pressedKeys.delete('shift');
    }
  }

  function handleViewerBlur() {
    pressedKeys.clear();
    mouseLookActive = false;
    lastFrameTime = 0;
  }

  function loadPreviewModel(url, source = 'manifest') {
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
        errorText =
          source === 'manual'
            ? 'Local GLB/GLTF could not be loaded. Showing a generated preview-safe placeholder.'
            : 'Manifest GLB/GLTF was not found or could not be loaded. Showing a generated preview-safe placeholder.';
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
    controls.maxDistance = Math.max(60, distance * 3.5);
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
      syncLookAnglesFromCamera();
      return;
    }

    camera?.position.set(3.6, 2.1, 5.2);
    controls?.target.set(0, 0.65, 0);
    syncLookAnglesFromCamera();
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
    manualModelName = file.name;
    manifest = normalizeManifest({
      manifestVersion: '1.0.0',
      previewId: `manual-${file.name}`,
      displayName: file.name,
      weaponName: file.name,
      sourceLabel: 'Manual local GLB/GLTF',
      sourceKind: 'manual_glb',
      previewStatus: 'ready',
      modelPreview: {
        url: activeModelUrl,
        format: file.name.toLowerCase().endsWith('.gltf') ? 'gltf' : 'glb',
        sizeBytes: file.size,
        sha256: null,
        generatedAt: new Date().toISOString()
      },
      sourceFiles: [],
      textureDictionaries: [],
      materialSlots: materialSlots,
      conversion: {
        gtaToGlbStatus: 'ready',
        glbToGtaStatus: 'unsupported',
        converterName: 'manual browser file',
        converterVersion: null,
        logsPath: null,
        warnings: ['Manual GLB/GLTF load only; no GTA converter ran.']
      },
      reverseTemplate: {
        templateWeaponName: null,
        originalSourceFiles: [],
        requiredFiles: [],
        stagedOutputDir: null,
        validationStatus: 'unsupported'
      },
      warnings: ['Manual local model loaded for preview only. No files were uploaded or converted.'],
      notes: ['This object URL exists only for the current browser session.']
    });
    loadPreviewModel(activeModelUrl, 'manual');
    event.currentTarget.value = '';
  }

  function cleanup() {
    clearTimeout(keyHintTimer);
    pressedKeys.clear();
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

<div class:focus-mode={focusMode} class="weapon-viewer" data-mode={weaponViewMode}>
  <div
    bind:this={mountEl}
    class="viewer-canvas"
    role="button"
    tabindex="0"
    aria-label="Interactive 3D weapon showroom"
    on:keydown={handleViewerKeydown}
    on:keyup={handleViewerKeyup}
    on:blur={handleViewerBlur}
    on:pointerdown={handleViewerPointerDown}
    on:pointermove={handleViewerPointerMove}
    on:pointerup={handleViewerPointerUp}
    on:pointercancel={handleViewerPointerUp}
    on:wheel={handleViewerWheel}
  ></div>

  {#if !focusMode}
    <div class="viewer-quickbar" aria-label="Showroom display controls">
      <button class:active={showInspector} type="button" on:click={() => (showInspector = !showInspector)}>Info</button>
      <button class:active={showViewerTools} type="button" on:click={() => (showViewerTools = !showViewerTools)}>Tools</button>
      <button type="button" on:click={() => (focusMode = true)}>Focus</button>
    </div>
  {/if}

  {#if showInspector && !focusMode}
  <div class="showroom-top">
    <div>
      <h2>{modelInfo.label}</h2>
      <small>{isRealConvertedModel ? 'Real GTA GLB preview' : cleanStatus(modelInfo.modelStatus)}</small>
    </div>
    <span class:real={isRealConvertedModel} class:warn={usingPlaceholder || errorText} class="status-badge">
      {statusLabel}
    </span>
  </div>
  {/if}

  {#if focusMode}
    <button
      class="focus-toggle"
      type="button"
      aria-pressed={focusMode}
      title="Show showroom UI"
      on:click={() => (focusMode = false)}
    >
      Show UI
    </button>
  {/if}

  {#if activeKeyHint}
    <div class="key-hint">{activeKeyHint}</div>
  {/if}

  {#if showInspector && !focusMode}
  <div class="tab-row" aria-label="Viewer mode">
    <button class:active={activeTab === 'preview'} type="button" on:click={() => (activeTab = 'preview')}>Preview</button>
    <button class:active={activeTab === 'textures'} type="button" on:click={() => (activeTab = 'textures')}>Textures</button>
    <button class:active={activeTab === 'details'} type="button" on:click={() => (activeTab = 'details')}>Details</button>
    <button class:active={activeTab === 'reverse'} type="button" on:click={() => (activeTab = 'reverse')}>Reverse</button>
  </div>

  <section class="showroom-panel" aria-label="Weapon model information">
    {#if activeTab === 'preview'}
      <div class="panel-head">
        <p class="panel-title">{modelInfo.title}</p>
        <p>{modelInfo.source}</p>
      </div>
      <div class="quick-stats" aria-label="Conversion status">
        {#each compactStats as stat}
          <span><strong>{stat[0]}</strong>{stat[1]}</span>
        {/each}
      </div>
      <dl>
        <div><dt>Model</dt><dd>{modelInfo.model}</dd></div>
        <div><dt>Format</dt><dd>{modelInfo.previewFormat}</dd></div>
      </dl>
      {#if isRealConvertedModel}
        <p class="real-model-note">Real converted GTA model</p>
      {/if}
    {:else if activeTab === 'textures'}
      <p class="panel-title">Texture dictionaries</p>
      {#if textureDictionaries.length}
        <div class="source-file-list">
          {#each textureDictionaries as dictionary}
            <div class="source-file">
              <span class="file-extension">YTD</span>
              <div>
                <strong>{dictionary.fileName}</strong>
                <small>{dictionary.textureNames?.length ? dictionary.textureNames.join(', ') : 'texture names unavailable'} - {dictionary.status}</small>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="empty-copy">No texture dictionaries were listed in this manifest.</p>
      {/if}

      <p class="panel-title">Material slots</p>
      {#if materialSlots.length}
        <div class="slot-list">
          {#each materialSlots as slot}
            <div class="texture-slot">
              <span class="slot-swatch"></span>
              <div>
                <strong>{slot.slotName}</strong>
                <small>{slot.textureName || 'unmapped'} - {slot.status}</small>
                {#if slot.sourceYtd}
                  <small>{slot.sourceYtd}</small>
                {/if}
                {#if slot.notes}
                  <small>{slot.notes}</small>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="empty-copy">Material mapping is not available yet.</p>
      {/if}
    {:else}
      {#if activeTab === 'details'}
        <p class="panel-title">Pipeline details</p>
        <dl>
          <div><dt>Source format</dt><dd>{modelInfo.sourceFormat}</dd></div>
          <div><dt>Model status</dt><dd>{cleanStatus(modelInfo.modelStatus)}</dd></div>
          <div><dt>Source kind</dt><dd>{cleanStatus(modelInfo.sourceKind)}</dd></div>
          <div><dt>Converter</dt><dd>{conversion.converterName || 'not configured'}</dd></div>
          <div><dt>Version</dt><dd>{conversion.converterVersion || 'unknown'}</dd></div>
          <div><dt>Logs</dt><dd>{conversion.logsPath || 'none'}</dd></div>
        </dl>
        {#if sourceFiles.length}
          <p class="panel-title">Source files</p>
          <div class="source-file-list">
            {#each sourceFiles as file}
              <div class="source-file">
                <span class="file-extension">{file.extension}</span>
                <div>
                  <strong>{file.fileName}</strong>
                  <small>{cleanStatus(file.role)} - {cleanStatus(file.status)}{file.sizeBytes ? ` - ${formatBytes(file.sizeBytes)}` : ''}</small>
                  {#if file.relativePath}
                    <small>{file.relativePath}</small>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else}
        <p class="panel-title">Reverse pipeline</p>
        <dl>
          <div><dt>Template</dt><dd>{reverseTemplate.templateWeaponName || manifest?.weaponPrefix || 'not selected'}</dd></div>
          <div><dt>Validation</dt><dd>{cleanStatus(reverseTemplate.validationStatus)}</dd></div>
          <div><dt>Staged dir</dt><dd>{reverseTemplate.stagedOutputDir || 'not created'}</dd></div>
          <div><dt>Output</dt><dd>{conversion.glbToGtaStatus === 'ready' ? 'staged GTA files' : 'unsupported until a safe exporter is configured'}</dd></div>
        </dl>
        {#if reverseTemplate.requiredFiles?.length}
          <p class="panel-title">Required template files</p>
          <div class="source-file-list">
            {#each reverseTemplate.requiredFiles as fileName}
              <div class="source-file">
                <span class="file-extension">SRC</span>
                <div>
                  <strong>{fileName}</strong>
                  <small>required original template</small>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
      <ul class="warning-list">
        {#each modelInfo.warnings as warning}
          <li>{warning}</li>
        {/each}
      </ul>
    {/if}
  </section>
  {/if}

  {#if showViewerTools && !focusMode}
  <div class="showroom-controls" aria-label="3D viewer controls">
    <button type="button" on:click={resetCamera}>Reset view</button>
    <button class:active={showGrid} type="button" on:click={toggleGrid}>Grid</button>
    <button class:active={autoRotate} type="button" on:click={toggleAutoRotate}>Auto-rotate</button>
    <button type="button" on:click={toggleLighting}>{lightPreset === 'studio' ? 'Studio light' : 'Inspection light'}</button>
    <button type="button" on:click={openFilePicker}>Load GLB</button>
  </div>
  {/if}

  {#if showMockWeaponPicker}
    <label class="weapon-view-select">
      <span>Preview set</span>
      <select bind:value={selectedEntryKey}>
        {#each Object.entries(fallbackEntries) as [key, entry]}
          <option value={key}>{entry.label}</option>
        {/each}
      </select>
    </label>
  {/if}

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
  .viewer-quickbar,
  .focus-toggle,
  .key-hint,
  .tab-row,
  .showroom-panel,
  .showroom-controls,
  .weapon-view-select,
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

  .viewer-quickbar {
    top: 16px;
    left: 16px;
    z-index: 7;
    display: inline-flex;
    gap: 4px;
    padding: 5px;
    border-radius: 10px;
  }

  .weapon-viewer.focus-mode .showroom-top,
  .weapon-viewer.focus-mode .viewer-quickbar,
  .weapon-viewer.focus-mode .key-hint,
  .weapon-viewer.focus-mode .tab-row,
  .weapon-viewer.focus-mode .showroom-panel,
  .weapon-viewer.focus-mode .showroom-controls,
  .weapon-viewer.focus-mode .weapon-view-select {
    opacity: 0;
    pointer-events: none;
    transform: translateY(-6px);
  }

  .weapon-viewer.focus-mode .showroom-controls {
    transform: translate(-50%, 8px);
  }

  .showroom-top {
    top: 16px;
    left: 16px;
    right: 104px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 12px;
    border-radius: 12px;
  }

  .showroom-top small {
    display: block;
    color: var(--text-3, #737373);
    font-size: 11px;
    line-height: 1.35;
  }

  h2 {
    margin: 0;
    color: #f5f5f5;
    font-size: 16px;
    font-weight: 600;
    line-height: 1.25;
  }

  .status-badge {
    border-radius: 999px;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .focus-toggle {
    top: 16px;
    right: 16px;
    z-index: 7;
    min-height: 34px;
    padding: 7px 10px;
    border-radius: 9px;
    color: #f5f5f5;
    font-size: 12px;
    cursor: pointer;
    transition:
      opacity 180ms ease,
      background 180ms ease,
      color 180ms ease;
  }

  .focus-toggle:hover,
  .focus-toggle[aria-pressed='true'] {
    background: rgba(255, 255, 255, 0.12);
  }

  .key-hint {
    left: 50%;
    top: 50%;
    padding: 8px 11px;
    border-radius: 999px;
    color: #fff;
    font-size: 12px;
    transform: translate(-50%, -50%);
    pointer-events: none;
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

  .status-badge.real {
    background: rgba(71, 209, 108, 0.16);
    color: #8ef0aa;
  }

  .tab-row {
    top: 86px;
    left: 16px;
    display: inline-flex;
    gap: 4px;
    padding: 5px;
    border-radius: 10px;
  }

  .tab-row button,
  .viewer-quickbar button,
  .showroom-controls button,
  .weapon-view-select select {
    min-height: 30px;
    padding: 6px 9px;
    border-radius: 7px;
    color: var(--text-2, #a1a1a1);
    cursor: pointer;
  }

  .tab-row button:hover,
  .viewer-quickbar button:hover,
  .showroom-controls button:hover,
  .weapon-view-select select:hover,
  .tab-row button.active,
  .viewer-quickbar button.active,
  .showroom-controls button.active {
    background: rgba(255, 255, 255, 0.11);
    color: #fff;
  }

  .showroom-panel {
    top: 138px;
    left: 16px;
    bottom: auto;
    width: min(292px, calc(100% - 32px));
    max-height: min(360px, calc(100% - 236px));
    display: grid;
    gap: 12px;
    padding: 12px;
    border-radius: 12px;
    overflow: auto;
  }

  .panel-head {
    display: grid;
    gap: 3px;
  }

  .panel-head p {
    margin: 0;
    color: var(--text-3, #737373);
    font-size: 11.5px;
    line-height: 1.35;
  }

  .panel-title {
    margin: 0;
    color: #f1f1f1;
    font-size: 13px;
    font-weight: 600;
  }

  .real-model-note,
  .empty-copy {
    margin: 0;
    color: var(--text-3, #737373);
    font-size: 12px;
    line-height: 1.4;
  }

  .real-model-note {
    color: #8ef0aa;
  }

  dl {
    display: grid;
    gap: 7px;
    margin: 0;
  }

  dl div {
    display: grid;
    grid-template-columns: 70px minmax(0, 1fr);
    gap: 8px;
  }

  .quick-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
  }

  .quick-stats span {
    display: grid;
    gap: 2px;
    min-width: 0;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.035);
    color: var(--text-2, #a1a1a1);
    font-size: 11px;
    overflow-wrap: anywhere;
  }

  .quick-stats strong {
    color: var(--text-3, #737373);
    font-size: 10px;
    font-weight: 500;
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

  .slot-list,
  .source-file-list {
    display: grid;
    gap: 8px;
  }

  .texture-slot,
  .source-file {
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

  .file-extension {
    display: inline-grid;
    place-items: center;
    min-width: 26px;
    height: 18px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: #d7e9ff;
    font-size: 9px;
    text-transform: uppercase;
  }

  .texture-slot strong,
  .source-file strong {
    display: block;
    color: #f1f1f1;
    font-size: 12px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .texture-slot small,
  .source-file small {
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

  .showroom-controls {
    left: 50%;
    bottom: 58px;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 5px;
    width: min(520px, calc(100% - 44px));
    padding: 6px;
    border-radius: 12px;
    transform: translateX(-50%);
  }

  .weapon-view-select {
    right: 16px;
    top: 86px;
    display: grid;
    gap: 3px;
    padding: 6px 8px;
    border-radius: 10px;
    font-size: 10px;
    color: var(--text-3, #737373);
  }

  .weapon-view-select select {
    width: 190px;
    border: 0;
    outline: none;
    background: rgba(255, 255, 255, 0.06);
    font: inherit;
    font-size: 12px;
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
      top: 12px;
    }

    .weapon-view-select {
      right: auto;
      left: 16px;
      top: 126px;
      max-width: calc(100% - 32px);
    }

    .showroom-panel {
      top: 178px;
    }

    .showroom-controls {
      bottom: 76px;
    }
  }

  @container (max-width: 520px) {
    .showroom-panel {
      width: min(292px, calc(100% - 32px));
      max-height: min(300px, calc(100% - 220px));
    }

    .showroom-controls {
      width: min(360px, calc(100% - 44px));
    }
  }
</style>
