<svelte:options runes={false} />
<script>
  // @ts-nocheck
  export let workflowState = 'idle'

  const steps = [
    { id: 'idle', label: 'Prompt' },
    { id: 'planning', label: 'AI Changes' },
    { id: 'reviewing', label: 'Review' },
    { id: 'validating', label: 'Validate' },
    { id: 'building', label: 'Auto Build' },
    { id: 'ready', label: 'Ready' },
  ]

  $: activeIndex = Math.max(
    0,
    steps.findIndex((step) => step.id === workflowState),
  )
</script>

<ol class="workflow-steps" aria-label="Redux Maker workflow">
  {#each steps as step, index}
    <li class:active={index === activeIndex} class:complete={index < activeIndex}>
      <span>{index + 1}</span>
      {step.label}
    </li>
  {/each}
</ol>
