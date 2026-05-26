<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
</script>

<svelte:head><title>Resources · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Resources" />
	<Panel title="Resource monitoring planned" icon="ti-info-circle">
		<p>No live resource backend exists yet, so this page does not show OK/healthy badges.</p>
		<ul>
			<li>CPU and RAM monitoring</li>
			<li>Disk volume metrics</li>
			<li>Network activity</li>
			<li>Process/task-manager view without unsafe kill controls</li>
		</ul>
	</Panel>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	p, li { color: var(--color-text-secondary); font-size: 12px; }
	p { margin: 0 0 10px; }
	ul { margin: 0; padding-left: 18px; }
</style>
