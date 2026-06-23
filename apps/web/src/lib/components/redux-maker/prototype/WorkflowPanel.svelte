<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'
  import StatusPill from './StatusPill.svelte'
  import WorkflowSteps from './WorkflowSteps.svelte'
  import AiChangeList from './AiChangeList.svelte'

  export let prompt = ''
  export let workflowState = 'idle'
  export let changes = []
  export let selectedChangeId = ''
  export let validationItems = []
  export let buildProgress = 0
  export let permissionMode = 'safe'

  const dispatch = createEventDispatcher()
  let readyMessage = ''

  const permissionLabels = {
    safe: 'Safe Mode',
    autoApprove: 'Auto Approve',
    autoPilot: 'Auto Pilot',
  }

  $: title =
    workflowState === 'planning'
      ? 'AI is creating grouped changes...'
      : workflowState === 'reviewing'
        ? 'AI Changes'
        : workflowState === 'validating'
          ? 'Validating changes...'
          : workflowState === 'building'
            ? 'Building update.rpf...'
            : workflowState === 'ready'
              ? 'Your update.rpf is ready'
              : 'AI Changes'
</script>

<article class="workflow-panel">
  <WorkflowSteps {workflowState} />

  <div class="workflow-header">
    <div>
      <p class="eyebrow">Redux Maker</p>
      <h1>{title}</h1>
    </div>
    <StatusPill tone={workflowState === 'ready' ? 'good' : 'neutral'}>
      {workflowState === 'ready' ? 'Ready' : workflowState === 'reviewing' ? permissionLabels[permissionMode] : 'Safe AI Build'}
    </StatusPill>
  </div>

  {#if workflowState === 'planning'}
    <section class="workflow-section">
      <h2>Prompt</h2>
      <p class="prompt-summary">“{prompt}”</p>
    </section>
    <div class="loading-lines" aria-label="Planning">
      <span></span>
      <span></span>
      <span></span>
    </div>
  {:else if workflowState === 'reviewing'}
    <section class="workflow-section">
      <h2>Prompt</h2>
      <p class="prompt-summary">“{prompt}”</p>
    </section>

    <AiChangeList
      {changes}
      {selectedChangeId}
      on:selectChange={(event) => dispatch('selectChange', event.detail)}
      on:changeAction={(event) => dispatch('changeAction', event.detail)}
    />

    <details class="advanced-details">
      <summary>Advanced details</summary>
      <p>Internal patch information stays hidden in this prototype. No real files are changed.</p>
    </details>

    <p class="auto-note">
      Review all changes. Accepted changes will be included in update.rpf.
    </p>
    {#if permissionMode === 'autoApprove'}
      <p class="auto-note">Auto Approve handled gunpack changes. Visual changes still need your review.</p>
    {:else if permissionMode === 'autoPilot'}
      <p class="auto-note">Auto Pilot is handling review, validation, and build automatically.</p>
    {/if}
  {:else if workflowState === 'validating'}
    <section class="workflow-section">
      <h2>Safety Check</h2>
      <ul class="check-list">
        {#each validationItems as item}
          <li>{item}</li>
        {/each}
      </ul>
    </section>
    <div class="loading-lines short" aria-label="Validation running">
      <span></span>
      <span></span>
    </div>
  {:else if workflowState === 'building'}
    <section class="workflow-section">
      <h2>Safety Check</h2>
      <ul class="check-list">
        {#each validationItems as item}
          <li>{item}</li>
        {/each}
      </ul>
    </section>
    <div class="progress-block">
      <div class="progress-label">
        <span>Preparing update.rpf</span>
        <span>{buildProgress}%</span>
      </div>
      <div class="progress-track">
        <span style={`width: ${buildProgress}%`}></span>
      </div>
    </div>
  {:else if workflowState === 'ready'}
    <section class="ready-card">
      <div class="ready-icon">
        <svg class="icon" width="28" height="28" viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5" /></svg>
      </div>
      <h2>Ready</h2>
      <ul class="check-list ready-checklist">
        <li>Built successfully</li>
        <li>Validated</li>
        <li>Ready to install</li>
        <li>Output: <code>update.rpf</code></li>
      </ul>
      <details class="advanced-details ready-details">
        <summary>Advanced details</summary>
        <p>Use your normal mods folder / OpenIV install workflow. No real files are written by this prototype.</p>
      </details>
      <div class="workflow-actions">
        <button class="primary-button" type="button" on:click={() => (readyMessage = 'Download prepared: update.rpf')}>Download update.rpf</button>
        <button class="secondary-button" type="button" on:click={() => (readyMessage = 'Output folder opened')}>Open output folder</button>
        <button class="secondary-button" type="button" on:click={() => dispatch('restart')}>Create another version</button>
      </div>
      {#if readyMessage}
        <p class="ready-message">{readyMessage}</p>
      {/if}
    </section>
  {/if}
</article>
