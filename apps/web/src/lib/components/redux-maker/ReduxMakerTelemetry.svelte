<script lang="ts">
	import ReduxMakerCorpusContext from './ReduxMakerCorpusContext.svelte';
	import type {
		ReduxCorpusStatus,
		ReduxCorpusDatasetSummary,
		ReduxCorpusLatestReport,
		ReduxCorpusDatasetRecord
	} from '$lib/api/client';
	import type { RunStatus, BridgeStatus, ApplyOutput } from '$lib/redux-maker/bridge';
	import type { ContextPack } from '$lib/redux-maker/contextPack';
	import { APPLY_ENDPOINT } from '$lib/redux-maker/bridge';

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
		applyResult = null,
		attachedContext = null,
		serverUrl = '',
		onAttach = (_records: ReduxCorpusDatasetRecord[]) => {}
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
		attachedContext?: ContextPack | null;
		serverUrl?: string;
		onAttach?: (records: ReduxCorpusDatasetRecord[]) => void;
	} = $props();

	const rep = $derived((runStatus?.report ?? null) as Record<string, unknown> | null);
	const safety = $derived((rep?.safetyFacts ?? {}) as Record<string, unknown>);
	const sb = (k: string, d = false) => (typeof safety[k] === 'boolean' ? (safety[k] as boolean) : d);

	const liveLabel = $derived(
		applyResult ? (applyResult.applied ? 'applied' : 'apply failed')
			: runStatus?.phase === 'running' ? 'running'
			: runStatus?.phase === 'finished' ? 'run finished'
			: runStatus?.phase === 'failed' ? 'run failed'
			: rep ? 'loaded' : 'no run loaded'
	);
	const forbidden = $derived(runStatus?.forbiddenEndpointCallCount ?? 0);
	const replaceCount = $derived(applyResult?.replaceRpfEntryCallCount ?? (sb('replaceRpfEntryCalled') ? 1 : 0));
	const localModelCalled = $derived(runStatus?.localModelCalled ?? false);
	const fallbackUsed = $derived(runStatus?.fallbackUsed ?? false);
	const writerAllowed = $derived(sb('writerAllowed', false));
	const nativeRpf = $derived(sb('nativeRpfParserImplemented', false));
	const cloudAi = $derived(runStatus?.cloudAiCalled ?? false);
	const publicNet = $derived(runStatus?.publicNetworkCall ?? false);
	const copiedModified = $derived(sb('copiedRpfModified', false));
	const shaBefore = $derived(applyResult?.shaBefore ?? (bridge?.copiedRpfSha ?? null));
	const shaAfter = $derived(applyResult?.shaAfter ?? null);

	const safeScore = $derived(
		[writerAllowed === false, nativeRpf === false, cloudAi === false, publicNet === false, forbidden === 0, copiedModified === false].filter(Boolean).length
	);
	const pct = $derived(Math.round((safeScore / 6) * 100));
	const blockReason = $derived(
		(rep?.blockedReasons as string[])?.[0] ?? (rep?.errors as string[])?.[0] ?? (rep?.warnings as string[])?.[0] ??
			(runStatus?.readyToApply === false ? 'readyToApply is false' : 'none')
	);
</script>

<div class="telemetry">
	<div class="tele-row">
		<span class="tele-title">process telemetry</span>
		<span class="tele-live"><span class="guard-dot"></span>{liveLabel}</span>
	</div>

	<div class="prog-wrap">
		<div class="prog-label"><span>Safety Readiness</span><span class="c-amber">{pct}%</span></div>
		<div class="prog-bar"><div class="prog-fill" style={`width:${pct}%`}></div></div>
		<div class="prog-sub">⟳ <span>{rep ? 'real run report loaded' : 'no report loaded · activeRun: none'}</span></div>
	</div>

	<div class="stat-grid">
		<div class="stat-card"><div class="stat-label">writerAllowed</div><div class="stat-val c-green">{String(writerAllowed)}</div><div class="stat-delta c-green">must remain false</div></div>
		<div class="stat-card"><div class="stat-label">Native RPF</div><div class="stat-val c-green">{String(nativeRpf)}</div><div class="stat-delta">no writer</div></div>
		<div class="stat-card"><div class="stat-label">Cloud AI</div><div class="stat-val c-green">{String(cloudAi)}</div><div class="stat-delta">no call</div></div>
		<div class="stat-card"><div class="stat-label">Local Model</div><div class="stat-val {localModelCalled ? 'c-amber' : 'c-text1'}">{String(localModelCalled)}</div><div class="stat-delta">loopback only</div></div>
		<div class="stat-card"><div class="stat-label">Fallback Used</div><div class="stat-val {fallbackUsed ? 'c-amber' : 'c-text1'}">{String(fallbackUsed)}</div><div class="stat-delta">rule_based fallback</div></div>
		<div class="stat-card"><div class="stat-label">Public Net</div><div class="stat-val c-green">{String(publicNet)}</div><div class="stat-delta">blocked</div></div>
		<div class="stat-card"><div class="stat-label">Replace Calls</div><div class="stat-val c-amber">{replaceCount}</div><div class="stat-delta">/api/replace-rpf-entry</div></div>
		<div class="stat-card"><div class="stat-label">Forbidden</div><div class="stat-val {forbidden === 0 ? 'c-green' : 'c-red'}">{forbidden}</div><div class="stat-delta {forbidden === 0 ? 'c-green' : 'c-red'}">must be 0</div></div>
	</div>

	<div class="badge-row">
		<span class="badge {nativeRpf === false ? 'badge-new' : 'badge-err'}">nativeRpfWrite={String(nativeRpf)}</span>
		<span class="badge {copiedModified === false ? 'badge-new' : 'badge-err'}">copiedRpfModified={String(copiedModified)}</span>
		<span class="badge {forbidden === 0 ? 'badge-new' : 'badge-err'}">forbidden={forbidden}</span>
	</div>

	<div class="apply-facts">
		<div class="prog-label"><span>apply facts</span><span class="c-amber">{APPLY_ENDPOINT}</span></div>
		<div class="apply-grid">
			<span>provider</span><span class="c-text1">{String(rep?.provider ?? '—')}</span>
			<span>readyToApply</span><span class={runStatus?.readyToApply ? 'c-amber' : 'c-text1'}>{String(runStatus?.readyToApply ?? false)}</span>
			<span>blocked reason</span><span class="c-text1">{blockReason}</span>
			<span>applied</span><span class={runStatus?.applied ? 'c-green' : 'c-text1'}>{String(runStatus?.applied ?? false)}</span>
			<span>replaceRpfEntry</span><span class="c-amber">{replaceCount}</span>
			<span>forbiddenEndpoints</span><span class={forbidden === 0 ? 'c-green' : 'c-red'}>{forbidden}</span>
			<span>CodeWalker</span><span class="c-text1 sha">{bridge?.codewalkerUrl ?? '—'}</span>
			<span>SHA before</span><span class="c-text1 sha">{shaBefore ?? '—'}</span>
			<span>SHA after</span><span class="c-text1 sha">{shaAfter ?? '—'}</span>
			<span>corpus ctx</span><span class={attachedContext ? 'c-green' : 'c-text1'}>{attachedContext ? `${attachedContext.recordCount} rec` : 'none'}</span>
		</div>
	</div>
</div>

<ReduxMakerCorpusContext {status} {dataset} {report} {quarantineTotal} loading={corpusLoading} error={corpusError} {corpusConnected} {serverUrl} {onAttach} />

<div class="log-pane">
	<div class="log-header"><div class="log-title"><span style="color:var(--orange)">$</span>system log</div></div>
	<div class="log-scroll">
		<div class="log-line"><span class="log-ts">00:00:00.000</span><span class="log-lvl lvl-info">[INFO]</span><span class="log-msg">Redux Maker bridge initialized</span></div>
		<div class="log-line"><span class="log-ts">00:00:00.080</span><span class="log-lvl lvl-bake">[SAFE]</span><span class="log-msg">HomeOps server does <span class="gn">not</span> edit RPF · apply is local desktop only</span></div>
		{#if bridge}
			<div class="log-line"><span class="log-ts">00:00:00.100</span><span class="log-lvl {bridge.available ? 'lvl-info' : 'lvl-warn'}">{bridge.available ? '[INFO]' : '[WARN]'}</span><span class="log-msg">bridge {bridge.available ? 'ready' : bridge.reason ?? 'unavailable'} · scanner <span class="gn">{bridge.scannerBinaryExists ? 'found' : 'missing'}</span> · copied RPF <span class={bridge.copiedRpfClean ? 'gn' : 'rd'}>{bridge.copiedRpfClean ? 'clean' : 'dirty'}</span></span></div>
		{:else}
			<div class="log-line"><span class="log-ts">00:00:00.100</span><span class="log-lvl lvl-warn">[WARN]</span><span class="log-msg">local bridge: browser/server mode (view-only)</span></div>
		{/if}
		{#if runStatus}
			<div class="log-line"><span class="log-ts">00:00:01.000</span><span class="log-lvl {runStatus.phase === 'failed' ? 'lvl-err' : 'lvl-info'}">{runStatus.phase === 'failed' ? '[ERR]' : '[INFO]'}</span><span class="log-msg">run <span class="hl">{runStatus.runId}</span>: {runStatus.phase}</span></div>
		{/if}
		{#if applyResult}
			<div class="log-line"><span class="log-ts">00:00:02.000</span><span class="log-lvl {applyResult.applied ? 'lvl-bake' : 'lvl-err'}">{applyResult.applied ? '[SAFE]' : '[ERR]'}</span><span class="log-msg">apply {applyResult.status} · replace-rpf-entry {applyResult.replaceRpfEntryCallCount} · forbidden {applyResult.forbiddenEndpointCallCount}</span></div>
		{/if}
		{#if corpusError}
			<div class="log-line"><span class="log-ts">00:00:03.000</span><span class="log-lvl lvl-err">[ERR]</span><span class="log-msg"><span class="rd">corpus: {corpusError}</span></span></div>
		{:else if corpusConnected}
			<div class="log-line"><span class="log-ts">00:00:03.000</span><span class="log-lvl lvl-info">[INFO]</span><span class="log-msg">corpus context: scanner ready{report ? ` · latest ${report.finishedAt}` : ''}</span></div>
		{/if}
	</div>
</div>
