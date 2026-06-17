<script lang="ts">
	import ReduxMakerCorpusContext from './ReduxMakerCorpusContext.svelte';
	import type {
		ReduxCorpusStatus,
		ReduxCorpusDatasetSummary,
		ReduxCorpusLatestReport
	} from '$lib/api/client';

	let {
		corpusConnected = false,
		corpusLoading = false,
		corpusError = null,
		status = null,
		dataset = null,
		report = null,
		quarantineTotal = null
	}: {
		corpusConnected?: boolean;
		corpusLoading?: boolean;
		corpusError?: string | null;
		status?: ReduxCorpusStatus | null;
		dataset?: ReduxCorpusDatasetSummary | null;
		report?: ReduxCorpusLatestReport | null;
		quarantineTotal?: number | null;
	} = $props();

	// Honest system log: no run has happened inside HomeOps, so the log only
	// reflects real workspace facts — never a fabricated run/apply.
	const logLines = $derived([
		{ level: 'safe', text: 'HomeOps server does not edit RPF · no apply · no CodeWalker' },
		{ level: 'info', text: 'local bridge: not connected (H2.1)' },
		corpusLoading
			? { level: 'info', text: 'corpus context: loading…' }
			: corpusError
				? { level: 'err', text: `corpus context: ${corpusError}` }
				: corpusConnected
					? { level: 'info', text: `corpus context: scanner ready${report ? ` · latest ${report.finishedAt}` : ''}` }
					: { level: 'warn', text: 'corpus context: scanner not configured' }
	]);
</script>

<div class="telemetry">
	<ReduxMakerCorpusContext {status} {dataset} {report} {quarantineTotal} loading={corpusLoading} error={corpusError} />

	<div class="block">
		<div class="card-title">Process telemetry</div>
		<div class="rows">
			<div class="trow"><span>run phase</span><b>idle · no run</b></div>
			<div class="trow"><span>module plan</span><b>none</b></div>
			<div class="trow"><span>generated assets</span><b>0</b></div>
			<div class="trow"><span>readyToApply</span><b class="danger">false</b></div>
			<div class="trow"><span>applied</span><b class="danger">false</b></div>
			<div class="trow"><span>forbidden endpoint calls</span><b class="ok">0</b></div>
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
