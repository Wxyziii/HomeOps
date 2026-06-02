<script lang="ts">
	import Breadcrumb from './Breadcrumb.svelte';
	import SmallButton from './SmallButton.svelte';
	let {
		parts = ['Home'],
		onnavigate,
		onrefresh
	}: {
		parts?: string[];
		onnavigate?: (index: number) => void;
		onrefresh?: () => void | Promise<void>;
	} = $props();
</script>

<div class="toolbar">
	<Breadcrumb {parts} {onnavigate} />
	<SmallButton icon="ti-refresh" title="Refresh" onclick={onrefresh} />
</div>

<style>
	.toolbar { display: flex; align-items: center; gap: 8px; padding: 10px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); }
</style>
