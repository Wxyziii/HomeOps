<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'
  import AiChangeList from './AiChangeList.svelte'

  export let activeView = 'new'
  export let messages = []
  export let changes = []
  export let selectedChangeId = ''
  export let workflowState = 'idle'
  export let validationItems = []
  export let buildProgress = 0
  export let permissionMode = 'safe'
  export let packs = []
  export let packTitle = 'Redux Pack'

  const dispatch = createEventDispatcher()

  const viewCopy = {
    packs: {
      title: 'My Packs',
      body: 'Built Redux packs will appear here after a build completes.',
      action: 'Start a new Redux',
    },
    recent: {
      title: 'Recent Builds',
      body: 'No recent builds yet. Send a prompt to create the first build.',
      action: 'New build',
    },
    templates: {
      title: 'Templates',
      body: 'Templates are empty. Build a pack first, then save a setup as a template.',
      action: 'Create from chat',
    },
    history: {
      title: 'History',
      body: 'No chat or build history has been saved in this prototype.',
      action: 'Open chat',
    },
    settings: {
      title: 'Settings',
      body: 'Prototype settings are local to this screen. Choose permission mode in the chat box before sending.',
      action: 'Back to chat',
    },
  }

  $: isChatView = activeView === 'new'
  $: disabled = workflowState === 'validating' || workflowState === 'building' || workflowState === 'ready'
  $: currentView = viewCopy[activeView]
  $: acceptedCount = changes.filter((change) => change.status === 'accepted').length
  $: skippedCount = changes.filter((change) => change.status === 'skipped').length
  $: sessionState =
    workflowState === 'validating'
      ? 'Validating'
      : workflowState === 'building'
        ? 'Building update.rpf'
        : workflowState === 'ready'
          ? 'Ready to install'
          : workflowState === 'planning'
            ? 'Generating changes'
            : 'Reviewing'
  $: sessionCount = workflowState === 'reviewing' && changes.length ? ` · ${changes.length} changes` : ''

  function modeLabel(mode) {
    if (mode === 'autoPilot') return 'Auto Pilot'
    if (mode === 'autoApprove') return 'Auto Approve'
    return 'Safe Mode'
  }
</script>

<section class:empty-state={isChatView && messages.length === 0} class="chat-workspace">
  {#if isChatView}
    <div class="chat-scroll">
      {#if messages.length === 0}
        <div class="empty-chat">
          <p class="eyebrow">Redux Maker</p>
          <h1>What Redux do you want to make?</h1>
          <p>Describe the visuals, tracer style, gunpack changes, or optimization goals. The AI workspace will build a reviewable pack from your chat.</p>
        </div>
      {:else}
        <div class="session-status" aria-label="Pack status">
          <span>{packTitle}</span>
          <span>{modeLabel(permissionMode)}</span>
          {#if sessionCount}
            <span>{changes.length} changes</span>
          {/if}
          <span>{sessionState}</span>
        </div>

        {#each messages as message}
          <article class={`chat-message ${message.role}`}>
            <div class="message-avatar">{message.role === 'user' ? 'You' : 'AI'}</div>
            <div class="message-bubble">
              <p>{message.text}</p>

              {#if message.kind === 'changes'}
                <AiChangeList
                  {changes}
                  {selectedChangeId}
                  {disabled}
                  on:selectChange={(event) => dispatch('selectChange', event.detail)}
                  on:changeAction={(event) => dispatch('changeAction', event.detail)}
                />
                <p class="chat-helper">Review all changes. Accepted changes will be included in update.rpf.</p>
              {/if}

              {#if message.kind === 'validating'}
                <ul class="chat-checklist">
                  {#each validationItems as item}
                    <li>{item}</li>
                  {/each}
                </ul>
              {/if}

              {#if message.kind === 'building'}
                <div class="progress-block chat-progress">
                  <div class="progress-label">
                    <span>Building update.rpf</span>
                    <span>{buildProgress}%</span>
                  </div>
                  <div class="progress-track"><span style={`width: ${buildProgress}%`}></span></div>
                </div>
              {/if}

              {#if message.kind === 'ready'}
                <ul class="ready-checklist chat-ready">
                  <li>Built successfully</li>
                  <li>Validated</li>
                  <li>Ready to install</li>
                  <li>Output: <code>update.rpf</code></li>
                </ul>
                <details class="advanced-details ready-details">
                  <summary>Advanced details</summary>
                  <p>OpenIV/mods-folder install workflow is represented only as prototype metadata. No files are written by this UI.</p>
                </details>
                <div class="workflow-actions">
                  <button class="primary-button" type="button" on:click={() => dispatch('navigate', 'packs')}>View pack</button>
                  <button class="secondary-button" type="button" on:click={() => dispatch('newRedux')}>New Redux</button>
                </div>
              {/if}
            </div>
          </article>
        {/each}

        {#if workflowState === 'reviewing' && changes.length}
          <div class="chat-status-line">
            <span>{acceptedCount} accepted</span>
            <span>{skippedCount} skipped</span>
            <span>{modeLabel(permissionMode)}</span>
          </div>
        {/if}
      {/if}
    </div>
  {:else}
    <div class="workspace-view">
      <p class="eyebrow">Redux Maker</p>
      <h1>{currentView.title}</h1>

      {#if activeView === 'packs' && packs.length > 0}
        <div class="pack-list">
          {#each packs as pack}
            <article class="pack-card">
              <h2>{pack.name}</h2>
              <p>{pack.output}</p>
              <span>{pack.status}</span>
            </article>
          {/each}
        </div>
      {:else}
        <p>{currentView.body}</p>
      {/if}

      <button class="secondary-button" type="button" on:click={() => dispatch('newRedux')}>{currentView.action}</button>
    </div>
  {/if}
</section>
