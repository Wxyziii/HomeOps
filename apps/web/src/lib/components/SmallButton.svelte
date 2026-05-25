<script lang="ts">
	let {
		icon = '',
		label = '',
		title = label,
		onclick
	}: { icon?: string; label?: string; title?: string; onclick?: () => void | Promise<void> } = $props();
</script>

<button class="btn-sm" type="button" {title} {onclick}>
	{#if icon}<i class="ti {icon}" aria-hidden="true"></i>{/if}
	{#if label}<span>{label}</span>{/if}
</button>

<style>
	.btn-sm { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: transparent; color: var(--color-text-primary); cursor: pointer; white-space: nowrap; }
	.btn-sm:hover { background: var(--bg-surface-2); color: var(--color-text-primary); }
	.btn-sm i { font-size: 14px; }
</style>
