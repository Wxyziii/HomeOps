<script lang="ts">
	let { placeholder = 'Search…' }: { placeholder?: string } = $props();
</script>

<div class="search-wrap">
	<i class="ti ti-search" aria-hidden="true"></i>
	<input type="text" {placeholder} />
</div>

<style>
	.search-wrap { flex: 1; position: relative; min-width: 140px; }
	.search-wrap i { position: absolute; left: 10px; top: 50%; transform: translateY(-50%); color: var(--color-text-tertiary); font-size: 15px; }
	.search-wrap input { width: 100%; padding: 8px 10px 8px 32px; font-size: 13px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-primary); outline: none; }
	.search-wrap input:focus { border-color: color-mix(in srgb, var(--accent) 70%, var(--color-border-tertiary)); }
</style>
