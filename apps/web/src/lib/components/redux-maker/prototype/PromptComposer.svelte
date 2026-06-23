<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher, tick } from 'svelte'

  export let prompt = ''
  export let selectedPreset = ''
  export let permissionMode = 'safe'
  export let compact = false

  const dispatch = createEventDispatcher()
  let permissionOpen = false
  let permissionButton
  let permissionDirection = 'down'
  let permissionMenuStyle = ''

  const permissionModes = [
    {
      id: 'safe',
      label: 'Safe Mode',
      description: 'Review every change before build.',
    },
    {
      id: 'autoApprove',
      label: 'Auto Approve',
      description: 'Auto-accept safe technical changes.',
    },
    {
      id: 'autoPilot',
      label: 'Auto Pilot',
      description: 'Generate, validate, and build automatically.',
    },
  ]

  $: activePermission = permissionModes.find((mode) => mode.id === permissionMode) ?? permissionModes[0]
  const presets = [
    'Clean FPS Redux',
    'Vertical Tracers',
    'Better Hit Effect',
    'Kill Effect',
    'Optimize update.rpf',
    'PvP Visual Pack',
  ]

  function choosePreset(preset) {
    dispatch('preset', preset)
  }

  function submit() {
    dispatch('submit')
  }

  function addContext() {
    const addition = 'Include a clean tactical gunpack preview.'
    prompt = prompt.trim() ? `${prompt.trim()} ${addition}` : addition
  }

  function choosePermission(mode) {
    permissionMode = mode
    permissionOpen = false
  }

  async function togglePermission() {
    if (permissionOpen) {
      permissionOpen = false
      return
    }

    placePermissionMenu()
    permissionOpen = true
    await tick()
    placePermissionMenu()
  }

  function placePermissionMenu() {
    if (!permissionButton) return

    const gap = 8
    const margin = 12
    const button = permissionButton.getBoundingClientRect()
    const viewportHeight = window.innerHeight
    const spaceBelow = viewportHeight - button.bottom - margin
    const spaceAbove = button.top - margin
    const menuHeight = 154
    const openDown = spaceBelow >= menuHeight || spaceBelow >= spaceAbove
    const maxHeight = Math.max(142, Math.min(260, openDown ? spaceBelow - gap : spaceAbove - gap))

    permissionDirection = openDown ? 'down' : 'up'
    permissionMenuStyle = `max-height: ${maxHeight}px;`
  }
</script>

<svelte:window on:resize={placePermissionMenu} on:scroll={placePermissionMenu} />

<div class:compact class="composer">
  <textarea
    bind:value={prompt}
    placeholder="Opisz wygląd, efekty, tracery, optymalizację albo styl paczki..."
    rows={compact ? 1 : 3}
    on:keydown={(event) => {
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) submit()
    }}
  ></textarea>

  <div class="composer-actions">
    <button class="icon-button" type="button" aria-label="Add context" on:click={addContext}>
      <svg class="icon" width="18" height="18" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
    </button>
    <div class="permission-control">
      <button bind:this={permissionButton} class:active={permissionOpen} class="access-pill" type="button" on:click={togglePermission}>
      <svg class="icon" width="15" height="15" viewBox="0 0 24 24"><path d="M12 9v4M12 17h.01" /><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" /></svg>
      {activePermission.label}
      <svg class="icon" width="13" height="13" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" /></svg>
      </button>
      {#if permissionOpen}
        <div class:opens-up={permissionDirection === 'up'} class="permission-menu" style={permissionMenuStyle}>
          {#each permissionModes as mode}
            <button class:active={permissionMode === mode.id} type="button" on:click={() => choosePermission(mode.id)}>
              <span>{mode.label}</span>
              <small>{mode.description}</small>
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <span class="composer-spacer"></span>
    <span class="model-pill"><b>AI</b> Changes</span>
    <button class="send-button" type="button" on:click={submit} aria-label="Start AI workflow">
      <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M12 19V5M6 11l6-6 6 6" /></svg>
    </button>
  </div>

  {#if !compact}
    <div class="preset-row" aria-label="Quick presets">
      {#each presets as preset}
        <button class:active={selectedPreset === preset} type="button" on:click={() => choosePreset(preset)}>
          {preset}
        </button>
      {/each}
    </div>
  {/if}
</div>
