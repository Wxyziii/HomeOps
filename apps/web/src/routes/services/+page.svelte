<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
</script>

<svelte:head><title>Services · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Services" />
	<Panel title="Services" icon="ti-info-circle">
		<p>This HomeOps Panel page is reserved for a later phase.</p>
	</Panel>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	p { margin: 0; color: var(--color-text-secondary); font-size: 12px; }
</style>
