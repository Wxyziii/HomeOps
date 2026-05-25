<script lang="ts">
	import type { Snippet } from 'svelte';
	let { title, flush = false, children }: { title: string; flush?: boolean; children?: Snippet } = $props();
</script>

<div class:flush class="topbar">
	<div class="page-title">{title}</div>
	{#if children}{@render children()}{/if}
</div>

<style>
	.topbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
	.topbar.flush { padding: 16px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); }
	.page-title { font-size: 16px; font-weight: 500; color: var(--color-text-primary); white-space: nowrap; }
</style>
