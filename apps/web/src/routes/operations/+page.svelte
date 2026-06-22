<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import IconButton from '$lib/components/IconButton.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import {
		cancelJob,
		getJobLogs,
		listJobs,
		listOperationLogs,
		runTestFailJob,
		runTestSleepJob,
		type Job,
		type JobLog,
		type OperationLog
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	type Tab = 'jobs' | 'logs' | 'running' | 'failed' | 'history';

	let tab = $state<Tab>('jobs');
	let jobs = $state<Job[]>([]);
	let operationLogs = $state<OperationLog[]>([]);
	let selectedJobId = $state<string | null>(null);
	let jobLogs = $state<JobLog[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let paused = $state(false);
	let interval: ReturnType<typeof setInterval> | null = null;
	let lastRefreshAt = 0;

	const selectedJob = $derived(jobs.find((job) => job.id === selectedJobId) ?? jobs[0] ?? null);
	const activeCount = $derived(jobs.filter((job) => job.status === 'queued' || job.status === 'running').length);
	const runningCount = $derived(jobs.filter((job) => job.status === 'running').length);
	const failedCount = $derived(jobs.filter((job) => job.status === 'failed').length);
	const visibleJobs = $derived(filterJobs(jobs));
	const visibleOperationLogs = $derived(filterOperationLogs(operationLogs));

	onMount(() => {
		serverConnection.load();
		loadUrlTab();
		void refresh();
		interval = setInterval(() => {
			if (paused || document.visibilityState !== 'visible') return;
			if (activeCount > 0 || Date.now() - lastRefreshAt > 5000) void refresh(false);
		}, 1500);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh(showLoading = true) {
		lastRefreshAt = Date.now();
		if (showLoading) loading = true;
		error = null;
		try {
			const [jobsResponse, logsResponse] = await Promise.all([
				listJobs(serverConnection.serverUrl),
				listOperationLogs(serverConnection.serverUrl, 150)
			]);
			jobs = jobsResponse.jobs;
			operationLogs = logsResponse.logs;
			if (!selectedJobId && jobs.length) selectedJobId = jobs[0].id;
			if (selectedJobId) await loadJobLogs(selectedJobId);
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load operations.';
		} finally {
			loading = false;
		}
	}

	async function loadJobLogs(id: string) {
		try {
			const response = await getJobLogs(serverConnection.serverUrl, id, 500);
			jobLogs = response.logs;
		} catch {
			jobLogs = [];
		}
	}

	async function selectJob(job: Job) {
		selectedJobId = job.id;
		await loadJobLogs(job.id);
	}

	async function runSleep() {
		error = null;
		try {
			const response = await runTestSleepJob(serverConnection.serverUrl);
			selectedJobId = response.job.id;
			tab = 'running';
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start test job.';
		}
	}

	async function runFail() {
		error = null;
		try {
			const response = await runTestFailJob(serverConnection.serverUrl);
			selectedJobId = response.job.id;
			tab = 'jobs';
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start failing test.';
		}
	}

	async function requestCancel(job: Job) {
		if (job.status !== 'queued') {
			error = 'Only queued jobs can be cancelled. Running jobs are allowed to finish.';
			return;
		}
		try {
			await cancelJob(serverConnection.serverUrl, job.id);
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not cancel job.';
		}
	}

	function togglePaused() {
		paused = !paused;
	}

	function filterJobs(items: Job[]) {
		const byTab = items.filter((job) => {
			if (tab === 'running') return job.status === 'queued' || job.status === 'running';
			if (tab === 'failed') return job.status === 'failed';
			if (tab === 'history') return job.status === 'finished' || job.status === 'failed' || job.status === 'cancelled';
			return true;
		});
		const query = searchQuery.trim().toLowerCase();
		if (!query) return byTab;
		return byTab.filter((job) => [job.id, job.title, job.jobType, job.status, job.error ?? ''].some((value) => value.toLowerCase().includes(query)));
	}

	function filterOperationLogs(items: OperationLog[]) {
		const query = searchQuery.trim().toLowerCase();
		if (!query) return items;
		return items.filter((log) => [log.source, log.level, log.message].some((value) => value.toLowerCase().includes(query)));
	}

	function formatDate(value: string | null) {
		return value ? new Date(value).toLocaleString() : '-';
	}

	function loadUrlTab() {
		const value = new URLSearchParams(window.location.search).get('tab');
		if (value === 'logs' || value === 'running' || value === 'failed' || value === 'history') {
			tab = value;
		}
	}

	function logKind(level: string) {
		if (level === 'error') return 'err';
		if (level === 'warn') return 'warn';
		return 'info';
	}
</script>

<svelte:head><title>Operations · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Operations" flush>
		<SearchInput placeholder="Filter operations..." bind:value={searchQuery} />
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={() => refresh()} />
		<SmallButton icon={paused ? 'ti-player-play' : 'ti-player-pause'} label={paused ? 'Resume' : 'Pause'} onclick={togglePaused} />
		<SmallButton icon="ti-player-play" label="Diagnostics: test job" onclick={runSleep} />
		<SmallButton icon="ti-alert-triangle" label="Diagnostics: failing job" onclick={runFail} />
	</Topbar>

	<div class="tabs">
		<button class:active={tab === 'jobs'} type="button" onclick={() => (tab = 'jobs')}>Jobs</button>
		<button class:active={tab === 'logs'} type="button" onclick={() => (tab = 'logs')}>Logs</button>
		<button class:active={tab === 'running'} type="button" onclick={() => (tab = 'running')}>Running</button>
		<button class:active={tab === 'failed'} type="button" onclick={() => (tab = 'failed')}>Failed</button>
		<button class:active={tab === 'history'} type="button" onclick={() => (tab = 'history')}>History</button>
	</div>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		<div class="metrics">
			<div class="metric"><span>Total jobs</span><strong>{jobs.length}</strong><em>newest first</em></div>
			<div class="metric"><span>Running</span><strong>{runningCount}</strong><em>queued + active</em></div>
			<div class="metric"><span>Failed</span><strong class="danger">{failedCount}</strong><em>all recorded jobs</em></div>
			<div class="metric"><span>Operation logs</span><strong>{operationLogs.length}</strong><em>{paused ? 'paused' : 'live'}</em></div>
		</div>

		{#if tab === 'logs'}
			<div class="log-pane">
				{#each visibleOperationLogs as log, index}
					<div class="log-row {logKind(log.level)}">
						<span class="line">{visibleOperationLogs.length - index}</span>
						<span class="time">{new Date(log.ts).toLocaleTimeString()}</span>
						<span class="source">[{log.source}]</span>
						<span class="level">{log.level.toUpperCase()}</span>
						<span class="message">{log.message}</span>
					</div>
				{:else}
					<div class="empty">No operation logs match this view.</div>
				{/each}
			</div>
		{:else}
			<div class="job-layout">
				<div class="job-list">
					{#each visibleJobs as job}
						<button class:selected={selectedJobId === job.id} class="job-card" type="button" onclick={() => selectJob(job)}>
							{#if job.status === 'running'}<div class="progress"><div style={`width:${job.progress}%`}></div></div>{/if}
							<div class="job-header">
								<div class="job-icon {job.status}"><i class="ti ti-clock-play" aria-hidden="true"></i></div>
								<div class="job-title">
									<div class="job-name">{job.title}</div>
									<div class="job-cmd">{job.jobType} · {job.id}</div>
								</div>
								<StatusBadge status={job.status === 'running' ? `running · ${job.progress}%` : job.status} />
								<IconButton icon="ti-ban" label={job.status === 'queued' ? 'Cancel queued job' : 'Cancel only available for queued jobs'} onclick={() => requestCancel(job)} disabled={job.status !== 'queued'} />
							</div>
							<div class="job-body">
								<div><span>Created</span><strong>{formatDate(job.createdAt)}</strong></div>
								<div><span>Started</span><strong>{formatDate(job.startedAt)}</strong></div>
								<div><span>Finished</span><strong>{formatDate(job.finishedAt)}</strong></div>
								<div><span>Progress</span><strong>{job.progress}%</strong></div>
							</div>
							{#if job.error}<div class="job-footer">{job.error}</div>{/if}
						</button>
					{:else}
						<div class="empty">No jobs match this view.</div>
					{/each}
				</div>
				<div class="logs-panel">
					<div class="logs-title">Job logs {selectedJob ? `· ${selectedJob.id}` : ''}</div>
					{#each jobLogs as log}
						<div class="job-log-line"><span>{new Date(log.ts).toLocaleTimeString()}</span>{log.line}</div>
					{:else}
						<div class="empty">Select a job to view its logs.</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.tabs { display: flex; align-items: center; gap: 8px; padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-sidebar); }
	.tabs button { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; background: transparent; color: var(--color-text-secondary); font-size: 11px; padding: 3px 10px; cursor: pointer; }
	.tabs button.active { background: var(--bg-app); color: var(--color-text-primary); }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.metric { background: var(--bg-surface-2); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; }
	.metric span, .metric em { display: block; font-size: 11px; color: var(--color-text-secondary); font-style: normal; }
	.metric strong { display: block; margin-top: 5px; font-size: 20px; font-weight: 600; color: var(--color-text-primary); }
	.metric strong.danger { color: var(--color-text-danger); }
	.metric em { margin-top: 3px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.job-layout { display: grid; grid-template-columns: minmax(0, 1fr) 380px; gap: 14px; align-items: start; }
	.job-list { display: flex; flex-direction: column; gap: 10px; }
	.job-card { width: 100%; text-align: left; background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); overflow: hidden; color: inherit; padding: 0; cursor: pointer; }
	.job-card.selected { border-color: color-mix(in srgb, var(--accent) 60%, var(--color-border-tertiary)); }
	.progress { height: 3px; background: var(--bg-surface-2); }
	.progress div { height: 100%; background: var(--accent); }
	.job-header { display: flex; align-items: center; gap: 12px; padding: 14px 16px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.job-icon { width: 34px; height: 34px; border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; font-size: 16px; background: var(--bg-surface-2); color: var(--accent); }
	.job-icon.finished { color: var(--success); }
	.job-icon.failed { color: var(--danger); }
	.job-icon.cancelled { color: var(--warning); }
	.job-title { flex: 1; min-width: 0; }
	.job-name { font-size: 13px; font-weight: 600; color: var(--color-text-primary); }
	.job-cmd, .job-footer { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.job-body { display: grid; grid-template-columns: repeat(4, 1fr); }
	.job-body div { padding: 10px 16px; border-right: 0.5px solid var(--color-border-tertiary); }
	.job-body div:last-child { border-right: none; }
	.job-body span { display: block; font-size: 10px; color: var(--color-text-tertiary); margin-bottom: 3px; }
	.job-body strong { font-size: 12px; color: var(--color-text-primary); font-weight: 600; }
	.job-footer { padding: 8px 16px; background: var(--bg-sidebar); border-top: 0.5px solid var(--color-border-tertiary); color: var(--color-text-danger); }
	.logs-panel, .log-pane { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); overflow: auto; font-family: var(--font-mono); }
	.logs-panel { padding: 12px; min-height: 240px; max-height: 560px; }
	.log-pane { min-height: 360px; padding: 8px 0; }
	.logs-title { color: var(--color-text-primary); font: 600 12px var(--font-sans); margin-bottom: 8px; }
	.job-log-line, .log-row { font-size: 11px; color: var(--color-text-secondary); line-height: 1.6; }
	.job-log-line span { color: var(--color-text-tertiary); margin-right: 8px; }
	.log-row { display: flex; align-items: flex-start; padding: 4px 20px; border-left: 3px solid transparent; }
	.log-row.err { border-left-color: var(--danger); background: color-mix(in srgb, var(--danger) 8%, transparent); }
	.log-row.warn { border-left-color: var(--warning); }
	.log-row.info { border-left-color: var(--accent); }
	.line { color: var(--color-text-tertiary); min-width: 36px; user-select: none; }
	.time { color: var(--color-text-tertiary); min-width: 96px; }
	.source { min-width: 128px; font-weight: 600; }
	.level { min-width: 54px; font-weight: 600; }
	.message { color: var(--color-text-secondary); flex: 1; }
	.empty { color: var(--color-text-tertiary); font-size: 12px; padding: 12px; }
	@media (max-width: 1100px) { .job-layout, .metrics { grid-template-columns: 1fr; } }
</style>
