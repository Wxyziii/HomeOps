<script lang="ts">
	// Honest empty blueprint: no run is loaded inside HomeOps (the local bridge
	// that produces runs/reports lands in H2.1). Sections mirror the standalone
	// Redux Maker tree but carry only truthful EMPTY rows.
	const sections = [
		{ label: 'Run', rows: ['no report files'] },
		{ label: 'Module', rows: ['no module plan'] },
		{ label: 'Blocked', rows: ['no blocked edits'] },
		{ label: 'Assets', rows: ['no generated assets'] }
	];
</script>

<aside class="blueprint">
	<div class="pane-title">Redux Blueprint <span class="hint">read-only</span></div>
	<div class="tree">
		{#each sections as section}
			<div class="tree-section">{section.label}</div>
			{#each section.rows as row}
				<div class="tree-row" title="Rows appear after a real local run loads (H2.1)">
					<span class="glyph">○</span><span class="row-label">{row}</span>
					<span class="badge">EMPTY</span>
				</div>
			{/each}
		{/each}
	</div>
	<div class="tree-empty">
		<div class="te-title">No run loaded</div>
		<div class="te-sub">Generate a module plan in the local Redux Maker app, then load it here once the H2.1 bridge is connected.</div>
	</div>
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
