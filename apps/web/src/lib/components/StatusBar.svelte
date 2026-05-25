<script lang="ts">
	let { items }: { items: string[] } = $props();
</script>

<div class="status-bar">{#each items as item}<span>{item}</span>{/each}</div>

<style>
	.status-bar { padding: 8px 20px; border-top: 0.5px solid var(--color-border-tertiary); font-size: 11px; color: var(--color-text-tertiary); display: flex; gap: 16px; background: var(--bg-surface); }
</style>
