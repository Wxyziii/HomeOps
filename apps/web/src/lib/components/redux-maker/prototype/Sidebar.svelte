<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'

  export let activeView = 'new'
  export let packs = []

  const dispatch = createEventDispatcher()

  const navItems = [
    { id: 'new', label: 'New Redux', icon: 'edit' },
    { id: 'packs', label: 'My Packs', icon: 'grid' },
    { id: 'recent', label: 'Recent Builds', icon: 'clock' },
    { id: 'templates', label: 'Templates', icon: 'spark' },
    { id: 'history', label: 'History', icon: 'history' }
  ]

  function navigate(id) {
    if (id === 'new') {
      dispatch('newRedux')
      return
    }

    dispatch('navigate', id)
  }
</script>

<aside class="sidebar">
  <div class="sidebar-scroll">
    {#each navItems as item}
      <button class:active={activeView === item.id} class="sidebar-row" type="button" on:click={() => navigate(item.id)}>
        <span class="sidebar-icon">
          {#if item.icon === 'edit'}
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
          {:else if item.icon === 'grid'}
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></svg>
          {:else if item.icon === 'clock'}
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>
          {:else if item.icon === 'spark'}
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M12 3l1.8 5.1L19 10l-5.2 1.9L12 17l-1.8-5.1L5 10l5.2-1.9z" /><path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z" /></svg>
          {:else}
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 3-6.7" /><path d="M3 4v6h6" /></svg>
          {/if}
        </span>
        {item.label}
      </button>
    {/each}

    <div class="sidebar-label">Recent packs</div>

    {#if packs.length}
      {#each packs as pack}
        <button class="pack-row" type="button" on:click={() => dispatch('navigate', 'packs')}>
          <span class="sidebar-icon">
            <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
          </span>
          <span class="pack-name">{pack.name}</span>
          <span class="blue-dot" aria-hidden="true"></span>
        </button>
      {/each}
    {:else}
      <div class="sidebar-empty">
        <span>No packs yet</span>
        <small>Finished Redux packs will appear here.</small>
      </div>
    {/if}

    <div class="sidebar-label">Saved styles</div>
    <div class="sidebar-empty">
      <span>No saved styles</span>
      <small>Save a generated style to reuse it later.</small>
    </div>
  </div>

  <button class:active={activeView === 'settings'} class="sidebar-footer" type="button" on:click={() => dispatch('navigate', 'settings')}>
    <span class="sidebar-icon">
      <svg class="icon" width="16" height="16" viewBox="0 0 24 24"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
    </span>
    <span>Settings</span>
    <span class="footer-spacer"></span>
    <svg class="icon" width="15" height="15" viewBox="0 0 24 24"><rect x="4" y="3" width="16" height="18" rx="2" /><path d="M9 3v18" /></svg>
  </button>
</aside>
