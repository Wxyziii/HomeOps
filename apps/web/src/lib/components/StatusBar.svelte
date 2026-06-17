<script lang="ts">
	let { items }: { items: string[] } = $props();
</script>

<div class="status-bar">{#each items as item}<span>{item}</span>{/each}</div>

<style>
	.status-bar { padding: 6px 12px; border-top: 1px solid var(--color-border-tertiary); font-size: 11px; color: var(--color-text-tertiary); display: flex; gap: 16px; background: var(--bg-input); }
</style>
