<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';

	const moduleMetrics = [
		{ label: 'Imported Reduxes', icon: 'ti-database-import', value: '12', unit: '', sub: 'mock library entries', percent: 60, color: '#378ADD' },
		{ label: 'Pending Jobs', icon: 'ti-clock-cog', value: '3', unit: '', sub: 'queued placeholders', percent: 30, color: '#EF9F27' },
		{ label: 'Profiles Built', icon: 'ti-id', value: '8', unit: '', sub: 'latest profile set', percent: 72, color: '#1D9E75' },
		{ label: 'Master Knowledge', icon: 'ti-brain', value: '4.6', unit: ' MB', sub: 'mock index size', percent: 45, color: '#7F77DD' }
	];
</script>

<svelte:head><title>AI Redux Maker · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="AI Redux Maker"><div class="chip"><div class="dot"></div> Module placeholder</div></Topbar>
	<div class="metrics">{#each moduleMetrics as metric}<MetricCard metric={metric} />{/each}</div>
	<div class="row2">
		<Panel title="Latest Report" icon="ti-report-analytics">
			<div class="report-line"><span>Last build</span><strong>redux-profile-summary.md</strong></div>
			<div class="report-line"><span>Generated</span><strong>Today 09:12</strong></div>
			<div class="report-line"><span>Status</span><StatusBadge status="ready" /></div>
		</Panel>
		<Panel title="AI Sandbox Status" icon="ti-flask">
			<div class="sandbox"><div class="sandbox-dot"></div><div><strong>Idle</strong><span>No model calls are wired in Phase 2.</span></div></div>
			<p>Imports, jobs, reports, and knowledge building are visual placeholders only.</p>
		</Panel>
	</div>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.row2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
	.chip { display: flex; align-items: center; gap: 5px; font-size: 12px; padding: 5px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); background: var(--color-background-secondary); }
	.dot, .sandbox-dot { width: 6px; height: 6px; border-radius: 50%; background: #1d9e75; }
	.report-line { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.report-line:last-child { border-bottom: none; }
	.report-line span, p { color: var(--color-text-secondary); }
	.sandbox { display: flex; align-items: flex-start; gap: 8px; font-size: 12px; }
	.sandbox span { display: block; color: var(--color-text-secondary); margin-top: 3px; }
	p { margin: 12px 0 0; font-size: 12px; }
</style>
