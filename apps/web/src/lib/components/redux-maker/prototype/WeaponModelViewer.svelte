<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'

  export let change
  export let weaponViewMode = 'firstPerson'

  const dispatch = createEventDispatcher()
</script>

<div class="weapon-viewer">
  <div class="studio-grid"></div>
  <div class="studio-light"></div>

  <div class:firstPerson={weaponViewMode === 'firstPerson'} class="weapon-model" aria-label="Mock weapon model">
    <span class="weapon-stock"></span>
    <span class="weapon-body"></span>
    <span class="weapon-rail"></span>
    <span class="weapon-barrel"></span>
    <span class="weapon-grip"></span>
    <span class="weapon-mag"></span>
    <span class="weapon-sight"></span>
  </div>

  <div class="weapon-hint">Drag to rotate · Scroll to zoom</div>

  <section class="weapon-info">
    <p>Weapon: {change?.weapon ?? 'Carbine Rifle'}</p>
    <p>New model: {change?.model ?? 'Tactical M4-style rifle'}</p>
    <p>Textures: Loaded</p>
    <p>View: {weaponViewMode === 'firstPerson' ? 'First-person' : 'World model'}</p>
  </section>

  <section class="weapon-status">
    <span>Model preview ready</span>
    <span>Textures loaded</span>
    <span>Materials checked</span>
    <span>Ready for approval</span>
  </section>

  <div class="weapon-view-toggle">
    <button
      class:active={weaponViewMode === 'firstPerson'}
      type="button"
      on:click={() => dispatch('weaponView', 'firstPerson')}
    >
      First-person
    </button>
    <button
      class:active={weaponViewMode === 'worldModel'}
      type="button"
      on:click={() => dispatch('weaponView', 'worldModel')}
    >
      World model
    </button>
  </div>
</div>
