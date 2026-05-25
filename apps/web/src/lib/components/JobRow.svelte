<script lang="ts">
	import StatusBadge from './StatusBadge.svelte';
	let { job }: { job: { name: string; time: string; status: string; icon: string } } = $props();
	const tone: Record<string, string> = { running: 'info', done: 'success', failed: 'danger', queued: 'warning' };
</script>

<div class="job-row">
	<div class="job-icon {tone[job.status] ?? 'info'}"><i class="ti {job.icon}" aria-hidden="true"></i></div>
	<div class="job-name">{job.name}</div>
	<div class="job-time">{job.time}</div>
	<StatusBadge status={job.status} />
</div>

<style>
	.job-row { display: flex; align-items: center; gap: 8px; padding: 7px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.job-row:last-child { border-bottom: none; }
	.job-icon { width: 26px; height: 26px; border-radius: 6px; display: flex; align-items: center; justify-content: center; font-size: 13px; flex-shrink: 0; }
	.job-icon.info { background: var(--color-background-info); color: var(--color-text-info); }
	.job-icon.success { background: var(--color-background-success); color: var(--color-text-success); }
	.job-icon.danger { background: var(--color-background-danger); color: var(--color-text-danger); }
	.job-icon.warning { background: var(--color-background-warning); color: var(--color-text-warning); }
	.job-name { flex: 1; color: var(--color-text-primary); font-weight: 500; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.job-time { color: var(--color-text-tertiary); font-size: 11px; white-space: nowrap; }
</style>
