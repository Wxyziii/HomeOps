<svelte:options runes={false} />
<script>
  // @ts-nocheck
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
</script>

<aside class:collapsed={previewCollapsed} class:fullscreen class:zoomed class="preview-pane">
  <button class="collapse-preview" type="button" on:click={() => dispatch('toggleCollapse')} aria-label={previewCollapsed ? 'Expand preview pane' : 'Collapse preview pane'}>
    {#if previewCollapsed}
      <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg>
    {:else}
      <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg>
    {/if}
  </button>

  {#if !previewCollapsed}
    {#if isWeapon}
      <WeaponModelViewer
        change={selectedChange}
        {weaponViewMode}
        on:weaponView={(event) => dispatch('weaponView', event.detail)}
      />
    {:else}
      <BeforeAfterSlider label={selectedChange?.title ?? 'Visual Preview'} mode={visualPreviewMode} />
    {/if}
  {/if}

  {#if !previewCollapsed}
    <div class="preview-toolbar top-left">{previewLabel}</div>

    <div class="preview-toolbar top-right" aria-label="Preview controls">
      {#if !isWeapon}
        <button type="button" on:click={toggleVisualMode}>{visualModeLabel}</button>
      {/if}
      <button class:active={zoomed} type="button" on:click={() => (zoomed = !zoomed)}>Zoom</button>
      <button type="button" on:click={resetPreview}>Reset</button>
      <button class:active={fullscreen} type="button" on:click={() => (fullscreen = !fullscreen)}>
        {fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
      </button>
    </div>

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
  {:else}
    <div class="collapsed-label">Preview</div>
  {/if}
</aside>
