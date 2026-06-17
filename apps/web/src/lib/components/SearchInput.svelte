<script lang="ts">
	let {
		placeholder = 'Search…',
		value = $bindable(''),
		disabled = false,
		title = ''
	}: { placeholder?: string; value?: string; disabled?: boolean; title?: string } = $props();
</script>

<div class="search-wrap">
	<i class="ti ti-search" aria-hidden="true"></i>
	<input type="text" {placeholder} bind:value {disabled} {title} />
</div>

<style>
	.search-wrap { flex: 1; position: relative; min-width: 140px; }
	.search-wrap i { position: absolute; left: 10px; top: 50%; transform: translateY(-50%); color: var(--color-text-tertiary); font-size: 15px; }
	.search-wrap input { width: 100%; height: 30px; padding: 0 10px 0 32px; font-size: 12px; border: 1px solid var(--color-border-secondary); border-radius: 4px; background: var(--bg-input); color: var(--color-text-primary); outline: none; }
	.search-wrap input::placeholder { color: var(--text-faint); }
	.search-wrap input:focus { border-color: var(--orange-border); }
</style>
