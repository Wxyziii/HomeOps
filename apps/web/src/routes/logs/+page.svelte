<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import { listOperationLogs, type OperationLog } from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let logs = $state<OperationLog[]>([]);
	let error = $state<string | null>(null);
	let paused = $state(false);
	let searchQuery = $state('');
	let levelFilter = $state<'all' | 'error' | 'warn' | 'info'>('all');
	let interval: ReturnType<typeof setInterval> | null = null;

	const visibleLogs = $derived(
		logs.filter((log) => {
			const matchesLevel = levelFilter === 'all' || log.level === levelFilter;
			const query = searchQuery.trim().toLowerCase();
			const matchesSearch =
				!query ||
				log.message.toLowerCase().includes(query) ||
				log.source.toLowerCase().includes(query) ||
				log.level.toLowerCase().includes(query);
			return matchesLevel && matchesSearch;
		})
	);

	onMount(() => {
		serverConnection.load();
		void refresh();
		interval = setInterval(() => {
			if (!paused && document.visibilityState === 'visible') void refresh();
		}, 4000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh() {
		try {
			const response = await listOperationLogs(serverConnection.serverUrl, 100);
			logs = response.logs;
			error = null;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load operation logs.';
		}
	}

	function kind(level: string) {
		if (level === 'error') return 'err';
		if (level === 'warn') return 'warn';
		return 'info';
	}

	function togglePaused() {
		paused = !paused;
	}
</script>

<svelte:head><title>Logs · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Logs" flush>
		<SearchInput placeholder="Filter operation logs..." bind:value={searchQuery} />
		<SmallButton icon="ti-refresh" label="Refresh" onclick={refresh} />
		<SmallButton icon={paused ? 'ti-player-play' : 'ti-player-pause'} label={paused ? 'Resume' : 'Pause'} onclick={togglePaused} />
	</Topbar>
	<div class="filterbar">
		<span class="active">Operations</span>
		<button class:active={levelFilter === 'all'} type="button" onclick={() => (levelFilter = 'all')}>All</button>
		<button class:active={levelFilter === 'error'} type="button" onclick={() => (levelFilter = 'error')}>ERR</button>
		<button class:active={levelFilter === 'warn'} type="button" onclick={() => (levelFilter = 'warn')}>WARN</button>
		<button class:active={levelFilter === 'info'} type="button" onclick={() => (levelFilter = 'info')}>INFO</button>
		<span class="disabled" title="Job log aggregation is available on the Jobs page">Jobs filter later</span>
		<div class="spacer"></div>
		<select disabled title="Log limit is fixed to last 100 for MVP"><option>Last 100</option></select>
	</div>
	{#if error}<div class="notice error">{error}</div>{/if}
	<div class="log-pane">
		{#each visibleLogs as log, index}
			<div class="log-row {kind(log.level)}">
				<span class="line">{visibleLogs.length - index}</span>
				<span class="time">{new Date(log.ts).toLocaleTimeString()}</span>
				<span class="source">[{log.source}]</span>
				<span class="level">{log.level.toUpperCase()}</span>
				<span class="message">{log.message}</span>
			</div>
		{:else}
			<div class="empty">No operation logs yet. Run a test job to create events.</div>
		{/each}
	</div>
	<div class="statusbar"><span><i></i>{paused ? 'Paused' : 'Live'}</span><span>{visibleLogs.length} shown / {logs.length} loaded</span><span>Source: operations</span></div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.filterbar { display: flex; align-items: center; gap: 8px; padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-sidebar); }
	.filterbar span, .filterbar button { font-size: 11px; padding: 3px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); background: transparent; }
	.filterbar button { cursor: pointer; }
	.filterbar .active { background: var(--bg-app); color: var(--color-text-primary); }
	.filterbar .disabled { opacity: 0.5; }
	.spacer { flex: 1; }
	select { font-size: 12px; padding: 4px 8px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-secondary); }
	select:disabled { opacity: 0.5; }
	.notice { padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.log-pane { flex: 1; overflow: auto; background: var(--bg-surface); font-family: var(--font-mono); font-size: 12px; padding: 8px 0; }
	.log-row { display: flex; align-items: flex-start; gap: 0; padding: 4px 20px; line-height: 1.6; border-left: 3px solid transparent; }
	.log-row:hover { background: var(--bg-app); }
	.log-row.err { border-left-color: var(--danger); background: color-mix(in srgb, var(--danger) 8%, transparent); }
	.log-row.warn { border-left-color: var(--warning); }
	.log-row.info { border-left-color: var(--accent); }
	.line { color: var(--color-text-tertiary); min-width: 36px; user-select: none; font-size: 11px; }
	.time { color: var(--color-text-tertiary); min-width: 96px; }
	.source { min-width: 128px; font-weight: 600; }
	.level { min-width: 54px; font-weight: 600; }
	.err .source, .err .level { color: var(--color-text-danger); }
	.warn .source, .warn .level { color: var(--color-text-warning); }
	.info .source, .info .level { color: var(--color-text-info); }
	.message { color: var(--color-text-secondary); flex: 1; }
	.empty { color: var(--color-text-tertiary); padding: 12px 20px; }
	.statusbar { padding: 6px 20px; border-top: 0.5px solid var(--color-border-tertiary); font-size: 11px; color: var(--color-text-tertiary); display: flex; gap: 16px; background: var(--bg-sidebar); }
	.statusbar i { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: var(--success); margin-right: 4px; vertical-align: middle; }
</style>
