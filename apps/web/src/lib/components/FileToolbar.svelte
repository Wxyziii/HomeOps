<script lang="ts">
	import Breadcrumb from './Breadcrumb.svelte';
	import SmallButton from './SmallButton.svelte';
</script>

<div class="toolbar">
	<Breadcrumb parts={['Home', 'data', 'media']} />
	<SmallButton icon="ti-layout-grid" title="Grid view" />
	<SmallButton icon="ti-list" title="List view" />
	<SmallButton icon="ti-adjustments-horizontal" label="Sort" />
</div>

<style>
	.toolbar { display: flex; align-items: center; gap: 8px; padding: 10px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); }
</style>
