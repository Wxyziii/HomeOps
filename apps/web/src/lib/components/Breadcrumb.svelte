<script lang="ts">
	let { parts }: { parts: string[] } = $props();
</script>

<nav class="breadcrumb" aria-label="Breadcrumb">
	{#each parts as part, index}
		<a href="/files">{part}</a>{#if index < parts.length - 1}<span>/</span>{/if}
	{/each}
</nav>

<style>
	.breadcrumb { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--color-text-secondary); flex: 1; min-width: 0; }
	.breadcrumb span { color: var(--color-text-tertiary); }
	.breadcrumb a { color: var(--color-text-info); cursor: pointer; white-space: nowrap; }
</style>
