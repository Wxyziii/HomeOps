<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import { getResourceSnapshot, type ResourceSnapshotResponse } from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	type ProcessRow = ResourceSnapshotResponse['processes'][number];
	type SortKey = 'cpu' | 'memory' | 'pid' | 'name';

	let snapshot = $state<ResourceSnapshotResponse | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let paused = $state(false);
	let searchQuery = $state('');
	let sortKey = $state<SortKey>('cpu');
	let interval: ReturnType<typeof setInterval> | null = null;

	const filteredProcesses = $derived(sortProcesses(filterProcesses(snapshot?.processes ?? []), sortKey));

	onMount(() => {
		serverConnection.load();
		void refresh();
		interval = setInterval(() => {
			if (!paused) void refresh(false);
		}, 2500);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh(showLoading = true) {
		if (showLoading) loading = true;
		try {
			snapshot = await getResourceSnapshot(serverConnection.serverUrl);
			error = null;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load resource snapshot.';
		} finally {
			loading = false;
		}
	}

	function filterProcesses(processes: ProcessRow[]) {
		const query = searchQuery.trim().toLowerCase();
		if (!query) return processes;
		return processes.filter((process) =>
			[process.pid.toString(), process.name, process.command, process.user, process.status]
				.some((value) => value.toLowerCase().includes(query))
		);
	}

	function sortProcesses(processes: ProcessRow[], key: SortKey) {
		return [...processes].sort((a, b) => {
			if (key === 'cpu') return b.cpuUsagePercent - a.cpuUsagePercent || b.memoryBytes - a.memoryBytes;
			if (key === 'memory') return b.memoryBytes - a.memoryBytes || b.cpuUsagePercent - a.cpuUsagePercent;
			if (key === 'pid') return a.pid - b.pid;
			return a.name.localeCompare(b.name) || a.pid - b.pid;
		});
	}

	function formatBytes(bytes: number) {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	function formatUptime(seconds: number) {
		const days = Math.floor(seconds / 86400);
		const hours = Math.floor((seconds % 86400) / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		return `${days}d ${hours}h ${minutes}m`;
	}

	function percent(used: number, total: number) {
		if (total <= 0) return 0;
		return Math.round((used / total) * 1000) / 10;
	}

	function togglePaused() {
		paused = !paused;
	}
</script>

<svelte:head><title>Resources · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Resources" flush>
		<SearchInput placeholder="Filter processes..." bind:value={searchQuery} />
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={() => refresh()} />
		<SmallButton icon={paused ? 'ti-player-play' : 'ti-player-pause'} label={paused ? 'Resume' : 'Pause'} onclick={togglePaused} />
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if snapshot}
			<div class="server-line">
				<strong>{snapshot.summary.hostname || 'unknown host'}</strong>
				<span>{snapshot.summary.os || 'OS unavailable'}</span>
				<span>Updated {new Date(snapshot.timestamp).toLocaleTimeString()}</span>
				<span>{paused ? 'Auto-refresh paused' : 'Auto-refresh 2.5s'}</span>
			</div>

			<div class="cards">
				<div class="card">
					<div class="label">CPU</div>
					<div class="value">{snapshot.summary.cpuUsagePercent.toFixed(1)}%</div>
					<div class="sub">{snapshot.summary.cpuCoreCount} cores</div>
				</div>
				<div class="card">
					<div class="label">Memory</div>
					<div class="value">{percent(snapshot.summary.memoryUsedBytes, snapshot.summary.memoryTotalBytes).toFixed(1)}%</div>
					<div class="sub">{formatBytes(snapshot.summary.memoryUsedBytes)} / {formatBytes(snapshot.summary.memoryTotalBytes)}</div>
				</div>
				<div class="card">
					<div class="label">Swap</div>
					<div class="value">{snapshot.summary.swapTotalBytes ? percent(snapshot.summary.swapUsedBytes, snapshot.summary.swapTotalBytes).toFixed(1) : '0.0'}%</div>
					<div class="sub">{formatBytes(snapshot.summary.swapUsedBytes)} / {formatBytes(snapshot.summary.swapTotalBytes)}</div>
				</div>
				<div class="card">
					<div class="label">Uptime / Load</div>
					<div class="value small">{formatUptime(snapshot.summary.uptimeSeconds)}</div>
					<div class="sub">{snapshot.summary.loadAverage.map((load) => load.toFixed(2)).join(' / ')}</div>
				</div>
				<div class="card">
					<div class="label">Workspace disk</div>
					<div class="value">{snapshot.workspace.usagePercent.toFixed(1)}%</div>
					<div class="sub">{formatBytes(snapshot.workspace.freeBytes)} free</div>
				</div>
			</div>

			<section class="panel">
				<div class="panel-head">
					<div>Disks</div>
					<span>{snapshot.disks.length} mounted volumes</span>
				</div>
				<table>
					<thead><tr><th>Mount</th><th>Filesystem</th><th>Used</th><th>Free</th><th>Total</th><th>Usage</th></tr></thead>
					<tbody>
						{#each snapshot.disks as disk}
							<tr>
								<td>{disk.mountPoint}</td>
								<td class="mono">{disk.fileSystem || 'unknown'}</td>
								<td>{formatBytes(disk.usedBytes)}</td>
								<td>{formatBytes(disk.freeBytes)}</td>
								<td>{formatBytes(disk.totalBytes)}</td>
								<td><div class="bar"><div style={`width:${Math.min(disk.usagePercent, 100)}%`}></div></div><span>{disk.usagePercent.toFixed(1)}%</span></td>
							</tr>
						{/each}
					</tbody>
				</table>
			</section>

			<section class="panel process-panel">
				<div class="panel-head">
					<div>Processes</div>
					<div class="sorts">
						<span>{filteredProcesses.length} shown / {snapshot.processes.length} loaded</span>
						<button class:active={sortKey === 'cpu'} type="button" onclick={() => (sortKey = 'cpu')}>CPU</button>
						<button class:active={sortKey === 'memory'} type="button" onclick={() => (sortKey = 'memory')}>Memory</button>
						<button class:active={sortKey === 'pid'} type="button" onclick={() => (sortKey = 'pid')}>PID</button>
						<button class:active={sortKey === 'name'} type="button" onclick={() => (sortKey = 'name')}>Name</button>
					</div>
				</div>
				<table>
					<thead><tr><th>PID</th><th>Name</th><th>CPU</th><th>Memory</th><th>Status</th><th>User</th><th>Command</th></tr></thead>
					<tbody>
						{#each filteredProcesses as process}
							<tr>
								<td class="mono">{process.pid}</td>
								<td>{process.name}</td>
								<td>{process.cpuUsagePercent.toFixed(1)}%</td>
								<td>{formatBytes(process.memoryBytes)}</td>
								<td>{process.status}</td>
								<td>{process.user || 'unknown'}</td>
								<td class="command">{process.command}</td>
							</tr>
						{:else}
							<tr><td colspan="7" class="empty">No matching processes.</td></tr>
						{/each}
					</tbody>
				</table>
			</section>
		{:else if loading}
			<div class="notice">Loading resource snapshot...</div>
		{:else}
			<div class="notice">No resource snapshot loaded yet.</div>
		{/if}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.server-line { display: flex; align-items: center; gap: 14px; color: var(--color-text-secondary); font-size: 12px; }
	.server-line strong { color: var(--color-text-primary); font-size: 13px; }
	.cards { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 10px; }
	.card, .panel { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); }
	.card { padding: 12px 14px; }
	.label, .sub, .panel-head span { color: var(--color-text-secondary); font-size: 11px; }
	.value { margin-top: 5px; color: var(--color-text-primary); font-size: 20px; font-weight: 600; }
	.value.small { font-size: 16px; }
	.sub { margin-top: 3px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.panel { overflow: hidden; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 13px; font-weight: 600; }
	table { width: 100%; border-collapse: collapse; table-layout: fixed; }
	th, td { padding: 8px 10px; border-bottom: 0.5px solid var(--color-border-tertiary); text-align: left; font-size: 12px; color: var(--color-text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	th { color: var(--color-text-tertiary); font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; background: var(--bg-sidebar); }
	tr:hover td { background: var(--bg-surface-2); }
	.mono, .command { font-family: var(--font-mono); }
	.command { color: var(--color-text-tertiary); }
	.bar { display: inline-block; vertical-align: middle; width: 72px; height: 5px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; margin-right: 8px; }
	.bar div { height: 100%; background: var(--accent); }
	.sorts { display: flex; align-items: center; gap: 6px; }
	.sorts button { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; background: transparent; color: var(--color-text-secondary); font-size: 11px; padding: 3px 8px; cursor: pointer; }
	.sorts button.active { background: var(--bg-surface-2); color: var(--color-text-primary); }
	.empty { text-align: center; color: var(--color-text-tertiary); }
	.process-panel th:nth-child(1), .process-panel td:nth-child(1) { width: 70px; }
	.process-panel th:nth-child(3), .process-panel td:nth-child(3) { width: 70px; }
	.process-panel th:nth-child(4), .process-panel td:nth-child(4) { width: 90px; }
	.process-panel th:nth-child(5), .process-panel td:nth-child(5) { width: 90px; }
	.process-panel th:nth-child(6), .process-panel td:nth-child(6) { width: 90px; }
	@media (max-width: 1100px) { .cards { grid-template-columns: repeat(2, minmax(0, 1fr)); } .server-line, .panel-head { flex-wrap: wrap; } }
</style>
