<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import {
		getResourceSnapshot,
		getWorkspaceStatus,
		listJobs,
		listOperationLogs,
		type Job,
		type OperationLog,
		type ResourceSnapshotResponse,
		type StorageRootStatus
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let jobs = $state<Job[]>([]);
	let recentLogs = $state<OperationLog[]>([]);
	let resources = $state<ResourceSnapshotResponse | null>(null);
	let storageRootOptions = $state<StorageRootStatus[]>([]);
	let dashboardError = $state<string | null>(null);
	let interval: ReturnType<typeof setInterval> | null = null;

	const queuedCount = $derived(jobs.filter((job) => job.status === 'queued').length);
	const runningCount = $derived(jobs.filter((job) => job.status === 'running').length);
	const finishedCount = $derived(jobs.filter((job) => job.status === 'finished').length);
	const failedCount = $derived(jobs.filter((job) => job.status === 'failed').length);

	onMount(async () => {
		serverConnection.load();
		await refreshDashboard();
		interval = setInterval(() => {
			if (document.visibilityState === 'visible') void refreshDashboard(false);
		}, 3000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refreshDashboard(testConnection = true) {
		dashboardError = null;
		try {
			if (testConnection) await serverConnection.testConnection();
			const [resourceResponse, jobsResponse, logsResponse, workspaceResponse] = await Promise.all([
				getResourceSnapshot(serverConnection.serverUrl),
				listJobs(serverConnection.serverUrl),
				listOperationLogs(serverConnection.serverUrl, 5),
				getWorkspaceStatus(serverConnection.serverUrl)
			]);
			resources = resourceResponse;
			jobs = jobsResponse.jobs;
			recentLogs = logsResponse.logs;
			storageRootOptions = workspaceResponse.storage_roots;
		} catch (error) {
			dashboardError = error instanceof Error ? error.message : 'Dashboard backend data could not be loaded.';
		}
	}

	function formatBytes(bytes: number | null | undefined) {
		if (bytes === null || bytes === undefined) return 'Unknown';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	function percent(used: number, total: number) {
		if (total <= 0) return 0;
		return (used / total) * 100;
	}
</script>

<svelte:head><title>Dashboard · HomeOps Panel</title></svelte:head>
<div class="dashboard-page">
	<Topbar title="Dashboard">
		<div class="topbar-right">
			<div class="connection-stack">
				<div class="chip">
					<div class:checking={serverConnection.connectionStatus === 'checking'} class:bad={serverConnection.connectionStatus === 'disconnected'} class="dot"></div>
					{serverConnection.connectionStatus === 'connected' ? 'Connected' : serverConnection.connectionStatus}
				</div>
				<div class="server-url">{serverConnection.serverUrl}</div>
			</div>
			<div class="chip"><i class="ti ti-refresh" aria-hidden="true"></i> Auto-refresh 3s</div>
		</div>
	</Topbar>

	{#if dashboardError}<div class="notice error">{dashboardError}</div>{/if}

	<div class="metrics">
		<div class="metric"><span>CPU</span><strong>{resources ? `${resources.summary.cpuUsagePercent.toFixed(1)}%` : 'Loading'}</strong><em>{resources?.summary.cpuCoreCount ?? 0} cores</em></div>
		<div class="metric"><span>Memory</span><strong>{resources ? `${percent(resources.summary.memoryUsedBytes, resources.summary.memoryTotalBytes).toFixed(1)}%` : 'Loading'}</strong><em>{resources ? `${formatBytes(resources.summary.memoryUsedBytes)} / ${formatBytes(resources.summary.memoryTotalBytes)}` : 'Waiting for backend'}</em></div>
		<div class="metric"><span>Workspace disk</span><strong>{resources ? `${resources.workspace.usagePercent.toFixed(1)}%` : 'Loading'}</strong><em>{resources ? `${formatBytes(resources.workspace.freeBytes)} free` : 'Waiting for backend'}</em></div>
		<div class="metric"><span>Jobs</span><strong>{queuedCount + runningCount}</strong><em>{queuedCount} queued / {runningCount} running</em></div>
	</div>

	<div class="row2">
		<Panel title="Job summary" icon="ti-player-play">
			<div class="summary-grid">
				<div><span>Queued</span><strong>{queuedCount}</strong></div>
				<div><span>Running</span><strong>{runningCount}</strong></div>
				<div><span>Finished</span><strong>{finishedCount}</strong></div>
				<div><span>Failed</span><strong class="danger">{failedCount}</strong></div>
			</div>
			{#each jobs.slice(0, 4) as job}
				<div class="job-line"><strong>{job.title}</strong><span>{job.status} · {job.progress}%</span></div>
			{:else}
				<div class="empty-state">No backend jobs recorded yet.</div>
			{/each}
		</Panel>

		<Panel title="Recent logs" icon="ti-file-text">
			{#each recentLogs as log}
				<div class="log-line"><span>{new Date(log.ts).toLocaleTimeString()}</span><strong>[{log.source}] {log.message}</strong></div>
			{:else}
				<div class="empty-state">No operation logs recorded yet.</div>
			{/each}
		</Panel>
	</div>

	<Panel title="Configured storage roots" icon="ti-database">
		<div class="storage-list">
			{#each storageRootOptions as root}
				<div class="storage-row">
					<div><strong>{root.label}</strong><span>{root.path}</span></div>
					<div><strong>{root.usagePercent?.toFixed(1) ?? '0.0'}%</strong><span>{formatBytes(root.freeBytes)} free / {formatBytes(root.totalBytes)} total</span></div>
					<div class="bar"><div style={`width:${Math.min(root.usagePercent ?? 0, 100)}%`}></div></div>
				</div>
			{:else}
				<div class="empty-state">Storage root status unavailable.</div>
			{/each}
		</div>
	</Panel>
</div>

<style>
	.dashboard-page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.topbar-right { display: flex; align-items: center; gap: 8px; }
	.connection-stack { display: flex; flex-direction: column; align-items: flex-end; gap: 3px; }
	.chip { display: flex; align-items: center; gap: 5px; font-size: 12px; padding: 5px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); background: var(--color-background-secondary); }
	.dot { width: 6px; height: 6px; border-radius: 50%; background: #1d9e75; }
	.dot.checking { background: var(--warning); }
	.dot.bad { background: #d85a30; }
	.server-url { color: var(--color-text-tertiary); font-size: 10px; max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.metric { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; }
	.metric span, .metric em, .storage-row span { color: var(--color-text-secondary); font-size: 11px; font-style: normal; }
	.metric strong { display: block; margin-top: 5px; color: var(--color-text-primary); font-size: 20px; }
	.row2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.summary-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-bottom: 10px; }
	.summary-grid div { background: var(--bg-surface-2); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 8px; }
	.summary-grid span { display: block; color: var(--color-text-secondary); font-size: 10px; }
	.summary-grid strong { color: var(--color-text-primary); font-size: 16px; }
	.summary-grid strong.danger { color: var(--color-text-danger); }
	.job-line, .log-line, .storage-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.job-line strong, .log-line strong, .storage-row strong { color: var(--color-text-primary); font-weight: 600; }
	.job-line span, .log-line span { color: var(--color-text-tertiary); font-size: 11px; }
	.log-line { display: grid; grid-template-columns: 88px 1fr; justify-content: initial; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	.storage-list { display: flex; flex-direction: column; gap: 2px; }
	.storage-row { display: grid; grid-template-columns: minmax(0, 1fr) 220px 160px; }
	.storage-row div { min-width: 0; }
	.storage-row span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: 3px; }
	.bar { height: 6px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; }
	.bar div { height: 100%; background: var(--accent); }
	@media (max-width: 980px) { .metrics, .row2, .storage-row { grid-template-columns: 1fr; } }
</style>
