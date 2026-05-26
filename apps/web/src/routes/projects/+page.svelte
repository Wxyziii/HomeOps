<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
</script>

<svelte:head><title>Projects · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Projects" />
	<Panel title="Project workspace planned" icon="ti-info-circle">
		<p>Projects will organize workspace folders and app modules later. There are no project actions in this MVP, so create/edit/delete controls are intentionally hidden.</p>
	</Panel>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	p { margin: 0; color: var(--color-text-secondary); font-size: 12px; line-height: 1.5; }
</style>
