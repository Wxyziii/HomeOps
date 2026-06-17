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
	.panel { background: var(--bg-surface-2); border: 1px solid var(--color-border-tertiary); border-radius: 4px; overflow: hidden; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 10px; margin: 0; border-bottom: 1px solid var(--color-border-tertiary); background: var(--bg-surface); }
	.panel-title { font-size: 11.5px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--color-text-tertiary); display: flex; align-items: center; gap: 7px; }
	.panel-title::before { content: ''; width: 7px; height: 7px; border-radius: 50%; background: var(--accent); flex: none; }
	.panel-title i { font-size: 14px; color: var(--accent); }
	.see-all { font-size: 11px; color: var(--accent); cursor: default; }
	:global(.panel > :not(.panel-head)) { padding: 10px; }
</style>
