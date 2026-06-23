<svelte:options runes={false} />
<script>
  // @ts-nocheck
  import { createEventDispatcher } from 'svelte'
  import ChangeReviewCard from './ChangeReviewCard.svelte'

  export let changes = []
  export let selectedChangeId = ''
  export let disabled = false

  const dispatch = createEventDispatcher()
</script>

<section class="ai-change-list">
  <div class="section-title-row">
    <div>
      <h2>Changes to review</h2>
      <p>Accept, regenerate, or skip each AI-generated change.</p>
    </div>
  </div>

  <div class="change-list-grid">
    {#each changes as change}
      <ChangeReviewCard
        {change}
        {disabled}
        selected={change.id === selectedChangeId}
        on:select={(event) => dispatch('selectChange', event.detail)}
        on:action={(event) => dispatch('changeAction', event.detail)}
      />
    {/each}
  </div>
</section>
