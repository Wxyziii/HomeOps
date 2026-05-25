<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import JobRow from '$lib/components/JobRow.svelte';
	import LogRow from '$lib/components/LogRow.svelte';
	import DiskUsageRow from '$lib/components/DiskUsageRow.svelte';
	import { disks, jobs, logs, metrics } from '$lib/data/mock';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	onMount(async () => {
		serverConnection.load();
		try {
			await serverConnection.testConnection();
		} catch {
			// State is updated by the shared connection store.
		}
	});
</script>

<svelte:head><title>Dashboard · HomeOps Panel</title></svelte:head>
<h2 class="sr-only">Home server dashboard showing system health, active jobs, recent logs, and resource usage</h2>
<div class="dashboard-page">
	<Topbar title="Dashboard">
		<div class="topbar-right">
			<div class="connection-stack">
				<div class="chip">
					<div class:checking={serverConnection.connectionStatus === 'checking'} class:bad={serverConnection.connectionStatus === 'disconnected'} class="dot"></div>
					{#if serverConnection.connectionStatus === 'connected'}
						Connected · {serverConnection.lastHealth?.service ?? 'server-agent'}
					{:else if serverConnection.connectionStatus === 'checking'}
						Checking server
					{:else}
						Disconnected
					{/if}
				</div>
				<div class="server-url">{serverConnection.serverUrl}</div>
			</div>
			<div class="chip"><i class="ti ti-clock" aria-hidden="true"></i> Updated just now</div>
		</div>
	</Topbar>
	<div class="metrics">{#each metrics as metric}<MetricCard {metric} />{/each}</div>
	<div class="row2">
		<Panel title="Active jobs" icon="ti-player-play" action="See all →">{#each jobs as job}<JobRow {job} />{/each}</Panel>
		<Panel title="Recent logs" icon="ti-file-text" action="See all →">{#each logs as log}<LogRow {log} />{/each}</Panel>
	</div>
	<Panel title="Disk volumes" icon="ti-chart-bar">{#each disks as disk}<DiskUsageRow {disk} />{/each}</Panel>
</div>

<style>
	.dashboard-page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.topbar-right { display: flex; align-items: center; gap: 8px; }
	.connection-stack { display: flex; flex-direction: column; align-items: flex-end; gap: 3px; }
	.chip { display: flex; align-items: center; gap: 5px; font-size: 12px; padding: 5px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); background: var(--color-background-secondary); }
	.chip i { font-size: 13px; }
	.dot { width: 6px; height: 6px; border-radius: 50%; background: #1d9e75; }
	.dot.checking { background: var(--warning); }
	.dot.bad { background: #d85a30; }
	.server-url { color: var(--color-text-tertiary); font-size: 10px; max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.row2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
	@media (max-width: 980px) { .metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .row2 { grid-template-columns: 1fr; } }
</style>
