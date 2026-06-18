<script lang="ts">
	import type { RunStatus } from '$lib/redux-maker/bridge';

	let { runStatus = null }: { runStatus?: RunStatus | null } = $props();

	// When a real local run is loaded, mirror its facts; otherwise show truthful
	// EMPTY rows. HomeOps never fabricates a plan.
	const sections = $derived.by(() => {
		if (!runStatus || !runStatus.report) {
			return [
				{ label: 'Run', rows: [{ text: 'no report files', badge: 'EMPTY' }] },
				{ label: 'Module', rows: [{ text: 'no module plan', badge: 'EMPTY' }] },
				{ label: 'Blocked', rows: [{ text: 'no blocked edits', badge: 'EMPTY' }] },
				{ label: 'Assets', rows: [{ text: 'no generated assets', badge: 'EMPTY' }] }
			];
		}
		const r = runStatus;
		return [
			{
				label: 'Run',
				rows: [
					{ text: r.runId, badge: r.phase.toUpperCase() },
					{ text: `readyToApply ${r.readyToApply ?? false}`, badge: r.readyToApply ? 'READY' : 'NO' },
					{ text: `applied ${r.applied}`, badge: r.applied ? 'YES' : 'NO' }
				]
			},
			{
				label: 'Module',
				rows: [
					{ text: `moduleSafe ${r.moduleSafe ?? '—'}`, badge: r.moduleSafe ? 'SAFE' : '—' },
					{ text: `replacement plans`, badge: String(r.replacementPlans) }
				]
			},
			{
				label: 'Assets',
				rows: [{ text: 'generated assets', badge: String(r.generatedAssets) }]
			},
			{
				label: 'Safety',
				rows: [
					{ text: 'forbidden endpoint calls', badge: String(r.forbiddenEndpointCallCount) },
					{ text: 'public network call', badge: r.publicNetworkCall ? 'YES' : 'NO' }
				]
			}
		];
	});
</script>

<aside class="blueprint">
	<div class="pane-title">Redux Blueprint <span class="hint">{runStatus ? 'local run' : 'read-only'}</span></div>
	<div class="tree">
		{#each sections as section}
			<div class="tree-section">{section.label}</div>
			{#each section.rows as row}
				<div class="tree-row" title={row.text}>
					<span class="glyph">{runStatus ? '●' : '○'}</span><span class="row-label">{row.text}</span>
					<span class="badge">{row.badge}</span>
				</div>
			{/each}
		{/each}
	</div>
	{#if !runStatus}
		<div class="tree-empty">
			<div class="te-title">No run loaded</div>
			<div class="te-sub">Enter a prompt or pick a preset, then Generate Module Plan (desktop bridge).</div>
		</div>
	{/if}
</aside>

<style>
	.blueprint { height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-sidebar); }
	.pane-title { flex: none; display: flex; align-items: center; justify-content: space-between; padding: 9px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
	.pane-title .hint { color: var(--text-faint); font-size: 10px; text-transform: none; letter-spacing: 0; }
	.tree { flex: 1; overflow: auto; padding: 6px 0; min-height: 0; }
	.tree-section { padding: 8px 12px 3px; color: var(--text-faint); font-size: 10.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; }
	.tree-row { display: flex; align-items: center; gap: 7px; padding: 4px 12px 4px 18px; color: var(--color-text-tertiary); font-size: 12px; }
	.glyph { color: var(--text-faint); }
	.row-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.badge { margin-left: auto; font-size: 9px; letter-spacing: 0.05em; color: var(--text-faint); border: 0.5px solid var(--color-border-tertiary); border-radius: 2px; padding: 1px 5px; }
	.tree-empty { flex: none; padding: 12px; border-top: 0.5px solid var(--color-border-tertiary); }
	.te-title { color: var(--color-text-secondary); font-size: 12px; font-weight: 600; }
	.te-sub { margin-top: 4px; color: var(--text-faint); font-size: 11px; line-height: 1.5; }
</style>
