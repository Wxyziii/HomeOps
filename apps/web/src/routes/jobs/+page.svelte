<script lang="ts">
	import IconButton from '$lib/components/IconButton.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import { scheduledJobs } from '$lib/data/mock';

	const metrics = [
		['Total jobs', '6', '4 active'],
		['Running now', '2', 'nightly, log-rotate'],
		['Failed (24h)', '1', 'db-snapshot'],
		['Next scheduled', '3h', 'log-rotate']
	];
</script>

<svelte:head><title>Jobs · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Jobs" flush>
		<div class="actions"><SmallButton icon="ti-history" label="Run history" /><SmallButton icon="ti-plus" label="New job" /></div>
	</Topbar>
	<div class="content">
		<div class="metrics">
			{#each metrics as metric}
				<div class="metric">
					<div class="metric-label">{metric[0]}</div>
					<div class:danger={metric[0].startsWith('Failed')} class="metric-value">{metric[1]}</div>
					<div class="metric-sub">{metric[2]}</div>
				</div>
			{/each}
		</div>
		{#each scheduledJobs as job}
			<div class="job-card">
				{#if job.status === 'running'}<div class="progress"><div style={`width:${job.progress}%`}></div></div>{/if}
				<div class="job-header">
					<div class="job-icon {job.tone}"><i class="ti {job.icon}" aria-hidden="true"></i></div>
					<div class="job-title">
						<div class="job-name">{job.name}</div>
						<div class="job-cmd">{job.command}</div>
					</div>
					<StatusBadge status={job.status === 'running' ? `running · ${job.progress}%` : job.status} />
					<div class="job-actions">
						<IconButton icon={job.status === 'running' ? 'ti-player-pause' : 'ti-player-play'} label="Job action placeholder" />
						<IconButton icon="ti-edit" label="Edit placeholder" />
					</div>
				</div>
				<div class="job-body">
					<div><span>Schedule</span><strong>{job.schedule}</strong></div>
					<div><span>Last run</span><strong>{job.lastRun}</strong></div>
					<div><span>Duration</span><strong>{job.duration}</strong></div>
					<div><span>Next run</span><strong>{job.nextRun}</strong></div>
				</div>
				<div class="job-footer"><span>cron: {job.schedule}</span><span class:error-note={job.status === 'failed'}>{job.note}</span></div>
			</div>
		{/each}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.actions { display: flex; gap: 8px; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; background: var(--bg-surface); }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.metric { background: var(--bg-surface-2); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; }
	.metric-label, .metric-sub { font-size: 11px; color: var(--color-text-secondary); }
	.metric-value { margin-top: 5px; font-size: 20px; font-weight: 600; color: var(--color-text-primary); }
	.metric-value.danger, .error-note { color: var(--color-text-danger); }
	.metric-sub { color: var(--color-text-tertiary); margin-top: 3px; }
	.job-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; }
	.progress { height: 3px; background: var(--bg-surface-2); }
	.progress div { height: 100%; background: var(--accent); }
	.job-header { display: flex; align-items: center; gap: 12px; padding: 14px 16px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.job-icon { width: 34px; height: 34px; border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; font-size: 16px; background: var(--bg-surface-2); }
	.job-icon.info { color: var(--accent); }
	.job-icon.ok { color: var(--success); }
	.job-icon.err { color: var(--danger); }
	.job-icon.warn { color: var(--warning); }
	.job-title { flex: 1; min-width: 0; }
	.job-name { font-size: 13px; font-weight: 600; color: var(--color-text-primary); }
	.job-cmd, .job-footer { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-tertiary); }
	.job-actions { display: flex; gap: 6px; }
	.job-body { display: grid; grid-template-columns: repeat(4, 1fr); }
	.job-body div { padding: 10px 16px; border-right: 0.5px solid var(--color-border-tertiary); }
	.job-body div:last-child { border-right: none; }
	.job-body span { display: block; font-size: 10px; color: var(--color-text-tertiary); margin-bottom: 3px; }
	.job-body strong { font-size: 12px; color: var(--color-text-primary); font-weight: 600; }
	.job-footer { display: flex; justify-content: space-between; gap: 12px; padding: 8px 16px; background: var(--bg-sidebar); border-top: 0.5px solid var(--color-border-tertiary); }
</style>
