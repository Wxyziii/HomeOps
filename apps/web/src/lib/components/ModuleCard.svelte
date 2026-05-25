<script lang="ts">
	import StatusBadge from './StatusBadge.svelte';
	let { module }: { module: { name: string; href: string; icon: string; status: string; description: string } } = $props();
</script>

<a class="module-card" href={module.href}>
	<div class="module-icon"><i class="ti {module.icon}" aria-hidden="true"></i></div>
	<div class="module-body">
		<div class="module-head"><span>{module.name}</span><StatusBadge status={module.status} /></div>
		<p>{module.description}</p>
	</div>
</a>

<style>
	.module-card { display: flex; gap: 10px; padding: 12px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); background: var(--bg-surface); min-height: 92px; }
	.module-card:hover { border-color: var(--color-border-secondary); background: var(--color-background-tertiary); }
	.module-icon { width: 32px; height: 32px; border-radius: var(--border-radius-md); background: var(--color-background-info); color: var(--color-text-info); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
	.module-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; font-size: 13px; font-weight: 500; }
	.module-body { flex: 1; min-width: 0; }
	p { margin: 6px 0 0; color: var(--color-text-secondary); font-size: 12px; line-height: 1.45; }
</style>
