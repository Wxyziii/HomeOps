<script lang="ts">
	let {
		icon = '',
		label = '',
		title = label,
		onclick,
		disabled = false
	}: { icon?: string; label?: string; title?: string; onclick?: () => void | Promise<void>; disabled?: boolean } = $props();
</script>

<button class="btn-sm" type="button" {title} {onclick} {disabled}>
	{#if icon}<i class="ti {icon}" aria-hidden="true"></i>{/if}
	{#if label}<span>{label}</span>{/if}
</button>

<style>
	.btn-sm { display: inline-flex; align-items: center; gap: 6px; min-height: 30px; font-size: 12px; font-weight: 600; padding: 0 10px; border: 1px solid var(--color-border-secondary); border-radius: 4px; background: var(--bg-surface); color: var(--color-text-secondary); cursor: pointer; white-space: nowrap; }
	.btn-sm:hover:not(:disabled) { background: var(--orange-bg); border-color: var(--orange-border); color: var(--color-text-primary); }
	.btn-sm:disabled { cursor: not-allowed; opacity: 0.48; }
	.btn-sm i { font-size: 14px; }
</style>
