<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { onDestroy } from 'svelte'
  import { createEventDispatcher } from 'svelte'
  import BeforeAfterSlider from './BeforeAfterSlider.svelte'
  import WeaponModelViewer from './WeaponModelViewer.svelte'

  export let workflowState = 'idle'
  export let selectedChange
  export let weaponViewMode = 'firstPerson'
  export let visualPreviewMode = 'comparison'
  export let previewCollapsed = false

  const dispatch = createEventDispatcher()
  let zoomed = false
  let fullscreen = false
  let previewMessage = ''
  let previewWidth = 520
  let resizing = false

  $: canReview = workflowState === 'reviewing' && selectedChange
  $: previewLabel = selectedChange ? `Previewing: ${selectedChange.title}` : 'Preview: Visual Redux'
  $: isWeapon = selectedChange?.previewMode === 'weapon3d'
  $: visualModeLabel = visualPreviewMode === 'comparison' ? 'Show tracers' : 'Show before/after'

  function action(type) {
    if (!selectedChange) return
    dispatch('changeAction', { id: selectedChange.id, action: type })
  }

  function toggleVisualMode() {
    dispatch('visualPreviewMode', visualPreviewMode === 'comparison' ? 'tracers' : 'comparison')
    previewMessage = ''
  }

  function resetPreview() {
    zoomed = false
    fullscreen = false
    previewMessage = 'Preview reset'
    dispatch('visualPreviewMode', 'comparison')
    dispatch('weaponView', 'firstPerson')
  }

  function clampPreviewWidth(width) {
    const maxWidth = Math.min(window.innerWidth - 360, 980)
    return Math.max(360, Math.min(width, Math.max(420, maxWidth)))
  }

  function resizePreview(event) {
    if (!resizing) return
    previewWidth = clampPreviewWidth(window.innerWidth - event.clientX)
  }

  function stopResize() {
    resizing = false
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('pointermove', resizePreview)
    window.removeEventListener('pointerup', stopResize)
    window.removeEventListener('pointercancel', stopResize)
  }

  function startResize(event) {
    if (fullscreen) return
    resizing = true
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    event.currentTarget.setPointerCapture?.(event.pointerId)
    window.addEventListener('pointermove', resizePreview)
    window.addEventListener('pointerup', stopResize)
    window.addEventListener('pointercancel', stopResize)
  }

  onDestroy(stopResize)
</script>

{#if previewCollapsed}
  <button class="preview-reopen" type="button" on:click={() => dispatch('toggleCollapse')} aria-label="Expand preview pane">
    <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg>
    <span>Preview</span>
  </button>
{:else}
<aside class:fullscreen class:resizing class:weaponPreview={isWeapon} class:zoomed class="preview-pane" style:flex-basis={fullscreen ? undefined : `${previewWidth}px`}>
  <button class="preview-resize-handle" type="button" on:pointerdown={startResize} aria-label="Resize preview pane"></button>

  <button class="collapse-preview" type="button" on:click={() => dispatch('toggleCollapse')} aria-label="Collapse preview pane">
    <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg>
  </button>

    {#if isWeapon}
      <WeaponModelViewer
        change={selectedChange}
        {weaponViewMode}
        manifestUrl={selectedChange?.previewManifestUrl}
        on:weaponView={(event) => dispatch('weaponView', event.detail)}
      />
    {:else}
      <BeforeAfterSlider label={selectedChange?.title ?? 'Visual Preview'} mode={visualPreviewMode} />
    {/if}

    {#if !isWeapon}
      <div class="preview-toolbar top-left">{previewLabel}</div>
    {/if}

    {#if !isWeapon}
    <div class="preview-toolbar top-right" aria-label="Preview controls">
      <button type="button" on:click={toggleVisualMode}>{visualModeLabel}</button>
      <button class:active={zoomed} type="button" on:click={() => (zoomed = !zoomed)}>Zoom</button>
      <button type="button" on:click={resetPreview}>Reset</button>
      <button class:active={fullscreen} type="button" on:click={() => (fullscreen = !fullscreen)}>
        {fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
      </button>
    </div>
    {/if}

    <div class="preview-toolbar bottom-center">
      {#if workflowState === 'ready'}
        <button class="primary-button" type="button" on:click={() => (previewMessage = 'Download prepared: update.rpf')}>Download update.rpf</button>
        <button class="ghost-button" type="button" on:click={() => dispatch('restart')}>New version</button>
      {:else if canReview}
        <button class="primary-button" type="button" on:click={() => action('accept')}>Accept</button>
        <button class="ghost-button" type="button" on:click={() => action('regenerate')}>Regenerate</button>
        <button class="ghost-button" type="button" on:click={() => action('skip')}>Skip</button>
      {:else}
        <button class="ghost-button" type="button" on:click={() => (previewMessage = 'Safe AI Build is active')}>Safe AI Build</button>
      {/if}
    </div>
    {#if previewMessage}
      <div class="preview-message">{previewMessage}</div>
    {/if}
</aside>
{/if}

<style>
  .preview-resize-handle {
    position: absolute;
    z-index: 14;
    top: 0;
    bottom: 0;
    left: 0;
    width: 10px;
    cursor: col-resize;
  }

  .preview-resize-handle::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 3px;
    width: 3px;
    height: 54px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.18);
    transform: translateY(-50%);
    opacity: 0;
  }

  .preview-resize-handle:hover::after,
  .preview-pane.resizing .preview-resize-handle::after {
    opacity: 1;
  }

  .preview-pane.fullscreen .preview-resize-handle {
    display: none;
  }

  .preview-pane.weaponPreview .preview-toolbar.bottom-center {
    bottom: 14px;
  }
</style>
