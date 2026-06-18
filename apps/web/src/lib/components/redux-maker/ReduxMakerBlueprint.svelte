<script lang="ts">
	import type { RunStatus } from '$lib/redux-maker/bridge';

	let { runStatus = null }: { runStatus?: RunStatus | null } = $props();

	type Row = { text: string; badge: string; cls: string };
	const sections = $derived.by((): { label: string; rows: Row[] }[] => {
		const r = runStatus?.report;
		if (!r) {
			return [
				{ label: 'Run', rows: [{ text: 'no report files', badge: 'EMPTY', cls: 'badge-empty' }] },
				{ label: 'Module', rows: [{ text: 'no module plan', badge: 'EMPTY', cls: 'badge-empty' }] },
				{ label: 'Blocked', rows: [{ text: 'no blocked edits', badge: 'EMPTY', cls: 'badge-empty' }] },
				{ label: 'Assets', rows: [{ text: 'no generated assets', badge: 'EMPTY', cls: 'badge-empty' }] }
			];
		}
		const n = (k: string) => Number((r[k] as number) ?? 0);
		const ready = runStatus?.readyToApply ?? false;
		return [
			{
				label: 'Run',
				rows: [
					{ text: runStatus!.runId, badge: runStatus!.phase.toUpperCase(), cls: runStatus!.phase === 'finished' ? 'badge-new' : 'badge-mod' },
					{ text: `status ${r.status ?? '—'}`, badge: ready ? 'READY' : 'PLAN', cls: ready ? 'badge-new' : 'badge-empty' }
				]
			},
			{
				label: 'Module',
				rows: [
					{ text: `moduleSafe ${runStatus!.moduleSafe ?? '—'}`, badge: runStatus!.moduleSafe ? 'SAFE' : '—', cls: runStatus!.moduleSafe ? 'badge-new' : 'badge-empty' },
					{ text: 'replacement plans', badge: String(runStatus!.replacementPlans), cls: runStatus!.replacementPlans ? 'badge-new' : 'badge-empty' }
				]
			},
			{
				label: 'Blocked',
				rows: [{ text: 'blocked children', badge: String(n('blockedChildCount')), cls: n('blockedChildCount') ? 'badge-err' : 'badge-empty' }]
			},
			{
				label: 'Assets',
				rows: [{ text: 'generated assets', badge: String(runStatus!.generatedAssets), cls: runStatus!.generatedAssets ? 'badge-new' : 'badge-empty' }]
			}
		];
	});
</script>

<aside class="sidebar-l">
	<div class="pane-title">Redux Blueprint<span class="pane-title-actions"><span style="color:var(--text-3);font-weight:400;letter-spacing:0">{runStatus ? 'local run' : 'read-only'}</span></span></div>
	<div class="tree">
		{#each sections as section}
			<div class="tree-section">{section.label}</div>
			{#each section.rows as row}
				<div class="tree-item" class:empty-row={!runStatus} title={row.text}>
					<span>{runStatus ? '●' : '○'}</span>
					<span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{row.text}</span>
					<span class="badge {row.cls}">{row.badge}</span>
				</div>
			{/each}
		{/each}
	</div>
	{#if !runStatus}
		<div class="tree-empty">
			<div class="tree-empty-title">No run loaded</div>
			<div class="tree-empty-sub">Enter a prompt or pick a preset, then Generate Module Plan (desktop bridge).</div>
		</div>
	{/if}
</aside>
