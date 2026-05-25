<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import IconButton from '$lib/components/IconButton.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import {
		cancelJob,
		getJobLogs,
		listJobs,
		runTestFailJob,
		runTestSleepJob,
		type Job,
		type JobLog
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let jobs = $state<Job[]>([]);
	let selectedJobId = $state<string | null>(null);
	let logs = $state<JobLog[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let interval: ReturnType<typeof setInterval> | null = null;

	const selectedJob = $derived(jobs.find((job) => job.id === selectedJobId) ?? jobs[0] ?? null);
	const runningCount = $derived(jobs.filter((job) => job.status === 'running').length);
	const failedCount = $derived(jobs.filter((job) => job.status === 'failed').length);

	onMount(() => {
		serverConnection.load();
		void refresh();
		interval = setInterval(() => void refresh(false), 1500);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh(showLoading = true) {
		if (showLoading) loading = true;
		error = null;
		try {
			const response = await listJobs(serverConnection.serverUrl);
			jobs = response.jobs;
			if (!selectedJobId && jobs.length) selectedJobId = jobs[0].id;
			if (selectedJobId) await loadLogs(selectedJobId);
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load jobs.';
		} finally {
			loading = false;
		}
	}

	async function loadLogs(id: string) {
		try {
			const response = await getJobLogs(serverConnection.serverUrl, id, 500);
			logs = response.logs;
		} catch {
			logs = [];
		}
	}

	async function selectJob(job: Job) {
		selectedJobId = job.id;
		await loadLogs(job.id);
	}

	async function runSleep() {
		error = null;
		try {
			const response = await runTestSleepJob(serverConnection.serverUrl);
			selectedJobId = response.job.id;
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
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start failing test.';
		}
	}

	async function requestCancel(job: Job) {
		try {
			await cancelJob(serverConnection.serverUrl, job.id);
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not cancel job.';
		}
	}

	function formatDate(value: string | null) {
		return value ? new Date(value).toLocaleString() : '—';
	}
</script>

<svelte:head><title>Jobs · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Jobs" flush>
		<div class="actions">
			<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={() => refresh()} />
			<SmallButton icon="ti-player-play" label="Run test job" onclick={runSleep} />
			<SmallButton icon="ti-alert-triangle" label="Run failing test" onclick={runFail} />
		</div>
	</Topbar>
	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		<div class="metrics">
			<div class="metric"><div class="metric-label">Total jobs</div><div class="metric-value">{jobs.length}</div><div class="metric-sub">newest first</div></div>
			<div class="metric"><div class="metric-label">Running now</div><div class="metric-value">{runningCount}</div><div class="metric-sub">in-process runner</div></div>
			<div class="metric"><div class="metric-label">Failed</div><div class="metric-value danger">{failedCount}</div><div class="metric-sub">all recorded jobs</div></div>
			<div class="metric"><div class="metric-label">Selected</div><div class="metric-value small">{selectedJob?.progress ?? 0}%</div><div class="metric-sub">{selectedJob?.id ?? 'none'}</div></div>
		</div>
		<div class="job-layout">
			<div class="job-list">
				{#each jobs as job}
					<button class:selected={selectedJobId === job.id} class="job-card" type="button" onclick={() => selectJob(job)}>
						{#if job.status === 'running'}<div class="progress"><div style={`width:${job.progress}%`}></div></div>{/if}
						<div class="job-header">
							<div class="job-icon {job.status}"><i class="ti ti-clock-play" aria-hidden="true"></i></div>
							<div class="job-title">
								<div class="job-name">{job.title}</div>
								<div class="job-cmd">{job.jobType} · {job.id}</div>
							</div>
							<StatusBadge status={job.status === 'running' ? `running · ${job.progress}%` : job.status} />
							<div class="job-actions">
								<IconButton icon="ti-ban" label="Cancel" onclick={() => requestCancel(job)} />
							</div>
						</div>
						<div class="job-body">
							<div><span>Created</span><strong>{formatDate(job.createdAt)}</strong></div>
							<div><span>Started</span><strong>{formatDate(job.startedAt)}</strong></div>
							<div><span>Finished</span><strong>{formatDate(job.finishedAt)}</strong></div>
							<div><span>Progress</span><strong>{job.progress}%</strong></div>
						</div>
						{#if job.error}<div class="job-footer"><span>{job.error}</span></div>{/if}
					</button>
				{/each}
				{#if jobs.length === 0}<div class="empty">No jobs yet.</div>{/if}
			</div>
			<div class="logs-panel">
				<div class="logs-title">Job logs</div>
				{#each logs as log}
					<div class="log-line"><span>{new Date(log.ts).toLocaleTimeString()}</span>{log.line}</div>
				{:else}
					<div class="empty">Select or run a job to view logs.</div>
				{/each}
			</div>
		</div>
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.actions { display: flex; gap: 8px; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.metric { background: var(--bg-surface-2); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; }
	.metric-label, .metric-sub { font-size: 11px; color: var(--color-text-secondary); }
	.metric-value { margin-top: 5px; font-size: 20px; font-weight: 600; color: var(--color-text-primary); }
	.metric-value.small { font-size: 16px; }
	.metric-value.danger { color: var(--color-text-danger); }
	.metric-sub { color: var(--color-text-tertiary); margin-top: 3px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.job-layout { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 14px; align-items: start; }
	.job-list { display: flex; flex-direction: column; gap: 10px; }
	.job-card { width: 100%; text-align: left; background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; color: inherit; padding: 0; cursor: pointer; }
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
	.job-actions { display: flex; gap: 6px; }
	.job-body { display: grid; grid-template-columns: repeat(4, 1fr); }
	.job-body div { padding: 10px 16px; border-right: 0.5px solid var(--color-border-tertiary); }
	.job-body div:last-child { border-right: none; }
	.job-body span { display: block; font-size: 10px; color: var(--color-text-tertiary); margin-bottom: 3px; }
	.job-body strong { font-size: 12px; color: var(--color-text-primary); font-weight: 600; }
	.job-footer { padding: 8px 16px; background: var(--bg-sidebar); border-top: 0.5px solid var(--color-border-tertiary); color: var(--color-text-danger); }
	.logs-panel { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px; min-height: 240px; max-height: 520px; overflow: auto; font-family: var(--font-mono); }
	.logs-title { color: var(--color-text-primary); font: 600 12px var(--font-sans); margin-bottom: 8px; }
	.log-line { font-size: 11px; color: var(--color-text-secondary); line-height: 1.6; }
	.log-line span { color: var(--color-text-tertiary); margin-right: 8px; }
	.empty { color: var(--color-text-tertiary); font-size: 12px; padding: 12px; }
	@media (max-width: 1100px) { .job-layout, .metrics { grid-template-columns: 1fr; } }
</style>
