<script lang="ts">
	import type { Snippet } from 'svelte';
	let { title, flush = false, children }: { title: string; flush?: boolean; children?: Snippet } = $props();
</script>

<div class:flush class="topbar">
	<div class="page-title">{title}</div>
	{#if children}{@render children()}{/if}
</div>

<style>
	.topbar { min-height: 38px; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 14px; border-bottom: 1px solid var(--color-border-tertiary); background: var(--bg-surface); overflow: hidden; }
	.topbar.flush { padding: 0 14px; border-bottom: 1px solid var(--color-border-tertiary); background: var(--bg-surface); }
	.page-title { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; color: var(--accent); white-space: nowrap; }
	.page-title::before { content: 'HomeOps'; color: var(--color-text-tertiary); font-weight: 500; }
	.page-title::after { content: '● COMMAND CENTER'; padding: 1px 7px; border: 1px solid var(--orange-border); border-radius: 2px; background: var(--orange-bg); color: var(--accent); font-size: 10px; font-weight: 600; letter-spacing: 0.04em; }
	:global(.topbar-right), :global(.actions) { min-width: 0; }
</style>
