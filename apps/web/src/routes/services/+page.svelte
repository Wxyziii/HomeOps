<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
</script>

<svelte:head><title>Services · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Services" />
	<Panel title="Service control planned" icon="ti-info-circle">
		<p>Service status and start/stop/restart actions are intentionally not exposed yet. This page will remain read-only or disabled until auth and safe service policy are designed.</p>
	</Panel>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	p { margin: 0; color: var(--color-text-secondary); font-size: 12px; line-height: 1.5; }
</style>
