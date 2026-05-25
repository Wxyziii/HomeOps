<script lang="ts">
	import type { Snippet } from 'svelte';
	let { title, icon = '', action = '', children }: { title: string; icon?: string; action?: string; children: Snippet } = $props();
</script>

<section class="panel">
	<div class="panel-head">
		<div class="panel-title">{#if icon}<i class="ti {icon}" aria-hidden="true"></i>{/if}{title}</div>
		{#if action}<span class="see-all">{action}</span>{/if}
	</div>
	{@render children()}
</section>

<style>
	.panel { background: var(--color-background-primary); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 14px; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; gap: 12px; }
	.panel-title { font-size: 13px; font-weight: 500; color: var(--color-text-primary); display: flex; align-items: center; gap: 6px; }
	.panel-title i { font-size: 15px; color: var(--color-text-secondary); }
	.see-all { font-size: 11px; color: var(--accent); cursor: default; }
</style>
