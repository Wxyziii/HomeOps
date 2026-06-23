<svelte:options runes={false} />
<script>
  // @ts-nocheck
  export let tone = 'neutral'
</script>

<span class:tone-good={tone === 'good'} class:tone-warn={tone === 'warn'} class="status-pill">
  <slot />
</span>
