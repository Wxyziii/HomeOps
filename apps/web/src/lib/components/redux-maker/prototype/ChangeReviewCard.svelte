<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'

  export let change
  export let selected = false
  export let disabled = false

  const dispatch = createEventDispatcher()

  $: statusText =
    change.status === 'accepted'
      ? 'Accepted ✓'
      : change.status === 'skipped'
        ? 'Skipped'
        : change.status === 'regenerating'
          ? 'Regenerating...'
          : 'Waiting for review'

  function action(type) {
    dispatch('action', { id: change.id, action: type })
  }
</script>

<article class:selected class={`change-card status-${change.status}`}>
  <button class="change-main" type="button" on:click={() => dispatch('select', change.id)}>
    <span class="change-category">{change.category === 'gunpack' ? 'Gunpack' : 'Visual'}</span>
    {#if selected}
      <span class="previewing-chip">Previewing</span>
    {/if}
    <h3>{change.title}</h3>
    <p>{change.description}</p>
    <span class="change-status">
      {#if change.status === 'accepted'}
        <svg class="icon status-check" width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 6 9 17l-5-5" /></svg>
      {/if}
      {statusText}
    </span>
  </button>

  <div class="change-actions">
    <button disabled={disabled} type="button" on:click={() => action(change.status === 'accepted' ? 'undo' : 'accept')}>
      {change.status === 'accepted' ? 'Undo' : 'Accept'}
    </button>
    <button disabled={disabled || change.status === 'regenerating'} type="button" on:click={() => action('regenerate')}>
      Regenerate
    </button>
    <button disabled={disabled || change.status === 'skipped'} type="button" on:click={() => action('skip')}>
      Skip
    </button>
  </div>
</article>
