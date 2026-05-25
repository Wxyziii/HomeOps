<script lang="ts">
	import IconButton from '$lib/components/IconButton.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import { archives } from '$lib/data/mock';

	const metrics = [
		['Total archives', '24', 'across 4 jobs'],
		['Total size', '312 GB', 'of 1 TB quota'],
		['Last backup', '2h ago', 'nightly-backup'],
		['Retention', '30 days', '4 expiring soon']
	];
</script>

<svelte:head><title>Archives · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Archives" flush>
		<div class="actions">
			<SmallButton icon="ti-search" label="Search" />
			<SmallButton icon="ti-plus" label="New archive" />
		</div>
	</Topbar>
	<div class="content">
		<div class="metrics">
			{#each metrics as metric}
				<div class="metric">
					<div class="metric-label">{metric[0]}</div>
					<div class="metric-value">{metric[1]}</div>
					<div class="metric-sub">{metric[2]}</div>
				</div>
			{/each}
		</div>
		<div class="section-head">
			<div class="section-title">All archives</div>
			<div class="filters"><span class="active">All</span><span>Full</span><span>Incremental</span><span>Snapshot</span></div>
		</div>
		<div class="archive-list">
			{#each archives as archive}
				<div class="archive-card">
					<div class="archive-icon {archive.tone}"><i class="ti {archive.icon}" aria-hidden="true"></i></div>
					<div class="archive-info">
						<div class="archive-name">{archive.name}</div>
						<div class="archive-meta">{archive.meta}</div>
					</div>
					<div class="archive-size">{archive.size}</div>
					<div class="archive-date">{archive.date}</div>
					<StatusBadge status={archive.status} />
					<div class="archive-actions">
						<IconButton icon={archive.status === 'failed' ? 'ti-refresh' : 'ti-download'} label="Archive action placeholder" />
						<IconButton icon="ti-dots-vertical" label="More actions" />
					</div>
				</div>
			{/each}
		</div>
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
	.metric-sub { color: var(--color-text-tertiary); margin-top: 3px; }
	.section-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
	.section-title { font-size: 13px; font-weight: 600; color: var(--color-text-primary); }
	.filters { display: flex; gap: 6px; }
	.filters span { font-size: 11px; padding: 4px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); }
	.filters .active { background: var(--bg-sidebar); color: var(--color-text-primary); }
	.archive-list { display: flex; flex-direction: column; gap: 8px; }
	.archive-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 14px 16px; display: flex; align-items: center; gap: 14px; }
	.archive-icon { width: 36px; height: 36px; border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; font-size: 18px; flex-shrink: 0; background: var(--bg-surface-2); }
	.archive-icon.archive { color: var(--warning); }
	.archive-icon.db { color: var(--accent); }
	.archive-icon.media { color: var(--success); }
	.archive-info { flex: 1; min-width: 0; }
	.archive-name { font-size: 13px; font-weight: 600; color: var(--color-text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.archive-meta, .archive-date { font-size: 11px; color: var(--color-text-secondary); }
	.archive-size { font-size: 13px; font-weight: 600; color: var(--color-text-primary); min-width: 70px; text-align: right; }
	.archive-date { min-width: 96px; text-align: right; color: var(--color-text-tertiary); }
	.archive-actions { display: flex; gap: 4px; }
</style>
