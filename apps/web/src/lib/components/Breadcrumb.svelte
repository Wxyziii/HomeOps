<script lang="ts">
	let { parts, onnavigate }: { parts: string[]; onnavigate?: (index: number) => void } = $props();
</script>

<nav class="breadcrumb" aria-label="Breadcrumb">
	{#each parts as part, index}
		<button type="button" onclick={() => onnavigate?.(index)}>{part}</button>{#if index < parts.length - 1}<span>/</span>{/if}
	{/each}
</nav>

<style>
	.breadcrumb { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--color-text-secondary); flex: 1; min-width: 0; }
	.breadcrumb span { color: var(--color-text-tertiary); }
	.breadcrumb button { border: 0; padding: 0; background: transparent; color: var(--color-text-info); cursor: pointer; white-space: nowrap; font: inherit; }
	.breadcrumb button:hover { color: var(--color-text-primary); }
</style>
