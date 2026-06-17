<script lang="ts">
	let {
		corpusConnected = false,
		packagesScanned = null,
		datasetRecords = null,
		latestScan = null,
		corpusError = null
	}: {
		corpusConnected?: boolean;
		packagesScanned?: number | null;
		datasetRecords?: number | null;
		latestScan?: string | null;
		corpusError?: string | null;
	} = $props();
</script>

<div class="review">
	<!-- Context ribbon -->
	<div class="ctx-ribbon">
		<span class="ctx-item">workspace: <b>redux-maker</b></span>
		<span class="ctx-item">corpus: <b class:ok={corpusConnected} class:err={!!corpusError}>{corpusError ? 'unavailable' : corpusConnected ? 'connected' : 'idle'}</b></span>
		{#if packagesScanned !== null}<span class="ctx-item">scanned: <b>{packagesScanned}</b></span>{/if}
		{#if datasetRecords !== null}<span class="ctx-item">records: <b>{datasetRecords}</b></span>{/if}
		{#if latestScan}<span class="ctx-item">scan: <b>{latestScan}</b></span>{/if}
		<span class="ctx-item read-only">read-only · no POST</span>
	</div>

	<!-- Dual review panes (honest empty state) -->
	<div class="panes">
		<div class="pane">
			<div class="pane-head"><span class="dot red"></span><span class="ph-label input">INPUT — no run loaded</span><span class="sha">— · —</span></div>
			<div class="pane-empty">
				<div class="pe-title">No run loaded</div>
				<div class="pe-sub">Generate a module plan in the local Redux Maker app. Inputs and prompts appear here once the H2.1 local bridge loads a real run.</div>
			</div>
		</div>
		<div class="pane">
			<div class="pane-head"><span class="dot green"></span><span class="ph-label review">REVIEW — safe plan preview</span><span class="sha">read-only · no POST</span></div>
			<div class="pane-empty">
				<div class="pe-title">No report files</div>
				<div class="pe-sub">No <code>mvp_report.json</code> / <code>apply_plan.json</code> loaded. HomeOps does not generate or apply — review appears after a local run is bridged in.</div>
				<div class="pe-flags">
					<span class="flag">readyToApply <b>false</b></span>
					<span class="flag">applied <b>false</b></span>
					<span class="flag">rollbackReady <b>false</b></span>
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.review { flex: 1; min-height: 0; display: flex; flex-direction: column; background: var(--bg-app); }
	.ctx-ribbon { flex: none; display: flex; flex-wrap: wrap; gap: 12px; padding: 7px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); font-size: 11px; color: var(--color-text-tertiary); }
	.ctx-item b { color: var(--color-text-secondary); font-weight: 600; }
	.ctx-item b.ok { color: var(--color-text-success); }
	.ctx-item b.err { color: var(--color-text-danger); }
	.ctx-item.read-only { margin-left: auto; color: var(--color-text-success); }
	.panes { flex: 1; min-height: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 1px; background: var(--color-border-tertiary); }
	.pane { background: var(--bg-app); min-width: 0; display: flex; flex-direction: column; }
	.pane-head { flex: none; display: flex; align-items: center; gap: 8px; padding: 7px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); }
	.dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
	.dot.red { background: var(--red); }
	.dot.green { background: var(--green); }
	.ph-label { font-size: 10px; font-weight: 600; letter-spacing: 0.04em; }
	.ph-label.input { color: var(--red); }
	.ph-label.review { color: var(--green); }
	.sha { margin-left: auto; color: var(--text-faint); font-size: 10px; font-family: var(--font-mono); }
	.pane-empty { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; justify-content: center; gap: 8px; padding: 20px; }
	.pe-title { color: var(--color-text-secondary); font-size: 13px; font-weight: 600; }
	.pe-sub { color: var(--text-faint); font-size: 11px; line-height: 1.6; }
	.pe-flags { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 4px; }
	.flag { font-size: 10px; font-family: var(--font-mono); color: var(--color-text-tertiary); border: 0.5px solid var(--color-border-tertiary); border-radius: 3px; padding: 2px 7px; }
	.flag b { color: var(--color-text-danger); }
	code { font-family: var(--font-mono); color: var(--accent); }
	@media (max-width: 720px) { .panes { grid-template-columns: 1fr; grid-template-rows: 1fr 1fr; } }
</style>
