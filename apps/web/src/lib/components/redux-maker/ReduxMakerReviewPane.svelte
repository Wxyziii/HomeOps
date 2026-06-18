<script lang="ts">
	import type { RunStatus } from '$lib/redux-maker/bridge';

	let {
		corpusConnected = false,
		runStatus = null,
		packagesScanned = null,
		datasetRecords = null,
		latestScan = null,
		corpusError = null
	}: {
		corpusConnected?: boolean;
		runStatus?: RunStatus | null;
		packagesScanned?: number | null;
		datasetRecords?: number | null;
		latestScan?: string | null;
		corpusError?: string | null;
	} = $props();

	const reportText = $derived(
		runStatus?.report ? JSON.stringify(runStatus.report, null, 2) : ''
	);
	const logText = $derived(
		runStatus ? [runStatus.stdoutTail, runStatus.stderrTail].filter(Boolean).join('\n') : ''
	);
</script>

<div class="review">
	<div class="ctx-ribbon">
		<span class="ctx-item">workspace: <b>redux-maker</b></span>
		<span class="ctx-item">corpus: <b class:ok={corpusConnected} class:err={!!corpusError}>{corpusError ? 'unavailable' : corpusConnected ? 'connected' : 'idle'}</b></span>
		{#if packagesScanned !== null}<span class="ctx-item">scanned: <b>{packagesScanned}</b></span>{/if}
		{#if datasetRecords !== null}<span class="ctx-item">records: <b>{datasetRecords}</b></span>{/if}
		{#if latestScan}<span class="ctx-item">scan: <b>{latestScan}</b></span>{/if}
		<span class="ctx-item read-only">{runStatus ? 'local run loaded' : 'read-only · no POST'}</span>
	</div>

	<div class="panes">
		<div class="pane">
			<div class="pane-head">
				<span class="dot red"></span>
				<span class="ph-label input">RUN — {runStatus ? runStatus.phase : 'no run loaded'}</span>
				<span class="sha">{runStatus ? runStatus.runId : '— · —'}</span>
			</div>
			{#if runStatus}
				<div class="pane-scroll">
					<pre class="log">{logText || '(no log output yet)'}</pre>
				</div>
			{:else}
				<div class="pane-empty">
					<div class="pe-title">No run loaded</div>
					<div class="pe-sub">Enter a prompt or pick a preset, then Generate Module Plan in the desktop app. Run logs and the report appear here.</div>
				</div>
			{/if}
		</div>
		<div class="pane">
			<div class="pane-head">
				<span class="dot green"></span>
				<span class="ph-label review">REVIEW — {runStatus?.readyToApply ? 'apply-ready' : 'plan preview'}</span>
				<span class="sha">read-only · no POST</span>
			</div>
			{#if runStatus?.report}
				<div class="pane-scroll">
					<div class="flags">
						<span class="flag" class:ok={runStatus.readyToApply}>readyToApply <b>{String(runStatus.readyToApply ?? false)}</b></span>
						<span class="flag" class:ok={runStatus.moduleSafe}>moduleSafe <b>{String(runStatus.moduleSafe ?? '—')}</b></span>
						<span class="flag" class:bad={runStatus.applied}>applied <b>{String(runStatus.applied)}</b></span>
						<span class="flag" class:bad={runStatus.forbiddenEndpointCallCount > 0}>forbidden <b>{runStatus.forbiddenEndpointCallCount}</b></span>
					</div>
					<pre class="report">{reportText}</pre>
				</div>
			{:else}
				<div class="pane-empty">
					<div class="pe-title">No report files</div>
					<div class="pe-sub">No <code>mvp_report.json</code> loaded yet. HomeOps never generates or applies on the server — review appears after a local desktop run finishes.</div>
				</div>
			{/if}
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
	.pane-scroll { flex: 1; min-height: 0; overflow: auto; padding: 8px 12px; }
	.log, .report { margin: 0; font-family: var(--font-mono); font-size: 10.5px; color: var(--color-text-tertiary); white-space: pre-wrap; overflow-wrap: anywhere; }
	.flags { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
	.flag { font-size: 10px; font-family: var(--font-mono); color: var(--color-text-tertiary); border: 0.5px solid var(--color-border-tertiary); border-radius: 3px; padding: 2px 7px; }
	.flag b { color: var(--color-text-secondary); }
	.flag.ok b { color: var(--color-text-success); }
	.flag.bad b { color: var(--color-text-danger); }
	code { font-family: var(--font-mono); color: var(--accent); }
	@media (max-width: 720px) { .panes { grid-template-columns: 1fr; grid-template-rows: 1fr 1fr; } }
</style>
