<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import DiskUsageRow from '$lib/components/DiskUsageRow.svelte';
	import { disks, metrics } from '$lib/data/mock';
	import { listJobs, listOperationLogs, type Job, type OperationLog } from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let activeJobs = $state<Job[]>([]);
	let recentLogs = $state<OperationLog[]>([]);
	let dashboardError = $state<string | null>(null);

	onMount(async () => {
		serverConnection.load();
		try {
			await serverConnection.testConnection();
			await loadDashboardData();
		} catch {
			// State is updated by the shared connection store.
		}
	});

	async function loadDashboardData() {
		dashboardError = null;
		try {
			const [jobsResponse, logsResponse] = await Promise.all([
				listJobs(serverConnection.serverUrl),
				listOperationLogs(serverConnection.serverUrl, 5)
			]);
			activeJobs = jobsResponse.jobs.slice(0, 5);
			recentLogs = logsResponse.logs;
		} catch (error) {
			dashboardError = error instanceof Error ? error.message : 'Dashboard backend data could not be loaded.';
		}
	}

	function statusClass(status: Job['status']) {
		return `status-${status}`;
	}
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
			<div class="chip"><i class="ti ti-clock" aria-hidden="true"></i> {serverConnection.lastCheckedAt ? `Checked ${new Date(serverConnection.lastCheckedAt).toLocaleTimeString()}` : 'Not checked yet'}</div>
		</div>
	</Topbar>
	<div class="placeholder-note">Resource cards are placeholders until resource monitoring is added.</div>
	<div class="metrics">{#each metrics as metric}<MetricCard {metric} />{/each}</div>
	{#if dashboardError}<div class="notice error">{dashboardError}</div>{/if}
	<div class="row2">
		<Panel title="Active jobs" icon="ti-player-play" action="See all →">
			{#each activeJobs as job}
				<div class="job-line">
					<div>
						<strong>{job.title}</strong>
						<span>{job.jobType} · {job.createdAt}</span>
					</div>
					<div class="job-right">
						<span class={`status ${statusClass(job.status)}`}>{job.status}</span>
						<span>{job.progress}%</span>
					</div>
				</div>
			{:else}
				<div class="empty-state">No backend jobs recorded yet.</div>
			{/each}
		</Panel>
		<Panel title="Recent logs" icon="ti-file-text" action="See all →">
			{#each recentLogs as log}
				<div class="log-line">
					<span>{log.ts}</span>
					<strong>[{log.source}] {log.message}</strong>
				</div>
			{:else}
				<div class="empty-state">No operation logs recorded yet.</div>
			{/each}
		</Panel>
	</div>
	<Panel title="Disk volumes" icon="ti-chart-bar">
		<div class="placeholder-note inner">Placeholder until resource monitoring is added.</div>
		{#each disks as disk}<DiskUsageRow {disk} />{/each}
	</Panel>
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
	.placeholder-note { color: var(--color-text-tertiary); font-size: 11px; margin: -4px 0 -6px; }
	.placeholder-note.inner { margin: 0 0 8px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.job-line { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.job-line strong, .log-line strong { display: block; color: var(--color-text-primary); font-weight: 600; }
	.job-line span, .log-line span { color: var(--color-text-tertiary); font-size: 11px; }
	.job-right { display: flex; align-items: center; gap: 8px; white-space: nowrap; }
	.status { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; padding: 2px 7px; color: var(--color-text-secondary); }
	.status-running { color: var(--accent); }
	.status-finished { color: var(--color-text-success); }
	.status-failed, .status-cancelled { color: var(--color-text-danger); }
	.log-line { display: grid; grid-template-columns: 148px 1fr; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 980px) { .metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .row2 { grid-template-columns: 1fr; } }
</style>
