<script lang="ts">
	import ReduxMakerCorpusContext from './ReduxMakerCorpusContext.svelte';
	import type {
		ReduxCorpusStatus,
		ReduxCorpusDatasetSummary,
		ReduxCorpusLatestReport
	} from '$lib/api/client';
	import type { RunStatus, BridgeStatus, ApplyOutput } from '$lib/redux-maker/bridge';

	let {
		corpusConnected = false,
		corpusLoading = false,
		corpusError = null,
		status = null,
		dataset = null,
		report = null,
		quarantineTotal = null,
		runStatus = null,
		bridge = null,
		applyResult = null
	}: {
		corpusConnected?: boolean;
		corpusLoading?: boolean;
		corpusError?: string | null;
		status?: ReduxCorpusStatus | null;
		dataset?: ReduxCorpusDatasetSummary | null;
		report?: ReduxCorpusLatestReport | null;
		quarantineTotal?: number | null;
		runStatus?: RunStatus | null;
		bridge?: BridgeStatus | null;
		applyResult?: ApplyOutput | null;
	} = $props();

	// Honest system log: reflects real bridge/run facts only — never fabricated.
	const logLines = $derived([
		{ level: 'safe', text: 'HomeOps server does not edit RPF · apply is local desktop only' },
		bridge
			? bridge.available
				? { level: 'info', text: `bridge ready · scanner ${bridge.scannerBinaryExists ? 'found' : 'missing'} · copied RPF ${bridge.copiedRpfClean ? 'clean' : 'dirty'}` }
				: { level: 'warn', text: `bridge unavailable: ${bridge.reason ?? 'unknown'}` }
			: { level: 'warn', text: 'local bridge: browser/server mode (view-only)' },
		runStatus
			? { level: runStatus.phase === 'failed' ? 'err' : 'info', text: `run ${runStatus.runId}: ${runStatus.phase}` }
			: { level: 'info', text: 'no local run loaded' },
		applyResult
			? { level: applyResult.applied ? 'safe' : 'err', text: `apply ${applyResult.status} · replace-rpf-entry ${applyResult.replaceRpfEntryCallCount} · forbidden ${applyResult.forbiddenEndpointCallCount}` }
			: { level: 'info', text: 'no apply performed' },
		corpusLoading
			? { level: 'info', text: 'corpus context: loading…' }
			: corpusError
				? { level: 'err', text: `corpus context: ${corpusError}` }
				: corpusConnected
					? { level: 'info', text: `corpus context: scanner ready${report ? ` · latest ${report.finishedAt}` : ''}` }
					: { level: 'warn', text: 'corpus context: scanner not configured' }
	]);

	const phase = $derived(runStatus?.phase ?? 'idle · no run');
	const modulePlan = $derived(
		runStatus ? `${runStatus.replacementPlans} replacement plan(s)` : 'none'
	);
	const genAssets = $derived(runStatus?.generatedAssets ?? 0);
	const ready = $derived(runStatus?.readyToApply ?? false);
	const applied = $derived(runStatus?.applied ?? false);
	const forbidden = $derived(runStatus?.forbiddenEndpointCallCount ?? 0);
</script>

<div class="telemetry">
	<ReduxMakerCorpusContext {status} {dataset} {report} {quarantineTotal} loading={corpusLoading} error={corpusError} />

	<div class="block">
		<div class="card-title">Process telemetry</div>
		<div class="rows">
			<div class="trow"><span>run phase</span><b>{phase}</b></div>
			<div class="trow"><span>module plan</span><b>{modulePlan}</b></div>
			<div class="trow"><span>generated assets</span><b>{genAssets}</b></div>
			<div class="trow"><span>readyToApply</span><b class:ok={ready} class:danger={!ready}>{ready}</b></div>
			<div class="trow"><span>applied</span><b class:ok={applied} class:danger={!applied}>{applied}</b></div>
			<div class="trow"><span>forbidden endpoint calls</span><b class:ok={forbidden === 0} class:danger={forbidden > 0}>{forbidden}</b></div>
		</div>
	</div>

	<div class="block">
		<div class="card-title">System log</div>
		<div class="log">
			{#each logLines as line}
				<div class="log-line {line.level}"><span class="lv">{line.level}</span>{line.text}</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.telemetry { min-height: 0; display: flex; flex-direction: column; background: var(--bg-sidebar); }
	.block { padding: 10px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.card-title { color: var(--color-text-primary); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.rows { margin-top: 8px; display: flex; flex-direction: column; gap: 5px; }
	.trow { display: flex; justify-content: space-between; gap: 10px; font-size: 11.5px; color: var(--color-text-tertiary); }
	.trow b { color: var(--color-text-secondary); font-weight: 600; }
	.trow b.danger { color: var(--color-text-danger); }
	.trow b.ok { color: var(--color-text-success); }
	.log { margin-top: 8px; display: flex; flex-direction: column; gap: 3px; font-family: var(--font-mono); font-size: 10.5px; }
	.log-line { color: var(--color-text-tertiary); overflow-wrap: anywhere; }
	.log-line .lv { display: inline-block; min-width: 38px; margin-right: 6px; text-transform: uppercase; font-size: 9px; }
	.log-line.safe { color: var(--color-text-success); }
	.log-line.err { color: var(--color-text-danger); }
	.log-line.warn { color: var(--color-text-warning); }
</style>
