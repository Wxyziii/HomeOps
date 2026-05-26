<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';

	const moduleMetrics = [
		{ label: 'Imported Reduxes', icon: 'ti-database-import', value: '0', unit: '', sub: 'placeholder: importer later', percent: 0, color: '#8db9e4' },
		{ label: 'Pending Jobs', icon: 'ti-clock-cog', value: '0', unit: '', sub: 'placeholder: module jobs later', percent: 0, color: '#b9852d' },
		{ label: 'Profiles Built', icon: 'ti-id', value: '0', unit: '', sub: 'placeholder: profile builder later', percent: 0, color: '#2f8f1f' },
		{ label: 'Master Knowledge', icon: 'ti-brain', value: '0', unit: ' MB', sub: 'placeholder: knowledge store later', percent: 0, color: '#7F77DD' }
	];
</script>

<svelte:head><title>AI Redux Maker · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="AI Redux Maker"><div class="chip"><div class="dot"></div> Planned module</div></Topbar>
	<div class="metrics">{#each moduleMetrics as metric}<MetricCard metric={metric} />{/each}</div>
	<div class="row2">
		<Panel title="Latest Report" icon="ti-report-analytics">
			<div class="report-line"><span>Last build</span><strong>None yet</strong></div>
			<div class="report-line"><span>Generated</span><strong>Not implemented</strong></div>
			<div class="report-line"><span>Status</span><StatusBadge status="later" /></div>
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
	.dot, .sandbox-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--warning); }
	.report-line { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.report-line:last-child { border-bottom: none; }
	.report-line span, p { color: var(--color-text-secondary); }
	.sandbox { display: flex; align-items: flex-start; gap: 8px; font-size: 12px; }
	.sandbox span { display: block; color: var(--color-text-secondary); margin-top: 3px; }
	p { margin: 12px 0 0; font-size: 12px; }
</style>
