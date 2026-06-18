<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import ReduxMakerLocalBridgeStatus from './ReduxMakerLocalBridgeStatus.svelte';
	import ReduxMakerBlueprint from './ReduxMakerBlueprint.svelte';
	import ReduxMakerReviewPane from './ReduxMakerReviewPane.svelte';
	import ReduxMakerPromptDock from './ReduxMakerPromptDock.svelte';
	import ReduxMakerTelemetry from './ReduxMakerTelemetry.svelte';
	import ReduxMakerActionDock from './ReduxMakerActionDock.svelte';
	import ApplyConfirmModal from './ApplyConfirmModal.svelte';
	import {
		getReduxCorpusStatus,
		getReduxCorpusDatasetSummary,
		getReduxCorpusLatestReport,
		getReduxCorpusQuarantine,
		type ReduxCorpusStatus,
		type ReduxCorpusDatasetSummary,
		type ReduxCorpusLatestReport
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		isTauri,
		getBridgeStatus,
		startRun,
		getRunStatus,
		cancelRun,
		applyReviewedPlan,
		applyReadiness,
		type BridgeStatus,
		type RunStatus,
		type ApplyOutput
	} from '$lib/redux-maker/bridge';
	import { loadBridgeSettings, type BridgeSettings } from '$lib/redux-maker/bridgeSettings';
	import { presetById, type RunMode } from '$lib/redux-maker/presets';

	const STANDALONE_PATH =
		'C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo\\apps\\redux-maker-ui';
	const DEV_COMMAND = `cd ${STANDALONE_PATH}; npm run tauri dev`;

	// ── corpus context (read-only, unchanged) ────────────────────────────────
	let status = $state<ReduxCorpusStatus | null>(null);
	let dataset = $state<ReduxCorpusDatasetSummary | null>(null);
	let report = $state<ReduxCorpusLatestReport | null>(null);
	let quarantineTotal = $state<number | null>(null);
	let corpusLoading = $state(false);
	let corpusError = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	const corpusConnected = $derived(!!status && status.scannerConfigured);

	// ── local bridge state (H2.1, desktop-only) ──────────────────────────────
	const desktop = isTauri();
	let settings = $state<BridgeSettings>(loadBridgeSettings());
	let bridge = $state<BridgeStatus | null>(null);
	let bridgeLoading = $state(false);
	let bridgeError = $state<string | null>(null);

	let prompt = $state('');
	let mode = $state<RunMode>('planOnly');
	let runId = $state<string | null>(null);
	let runStatus = $state<RunStatus | null>(null);
	let runError = $state<string | null>(null);
	let pollTimer: ReturnType<typeof setInterval> | null = null;

	let showApplyModal = $state(false);
	let applyBusy = $state(false);
	let applyResult = $state<ApplyOutput | null>(null);
	let applyError = $state<string | null>(null);

	const running = $derived(
		!!runStatus && (runStatus.phase === 'running' || runStatus.phase === 'queued')
	);
	const bridgeReady = $derived(!!bridge && bridge.available && bridge.bridgeEnabled);

	const applyState = $derived(
		applyReadiness({
			bridgeAvailable: bridgeReady,
			copiedRpfClean: bridge?.copiedRpfClean ?? false,
			status: runStatus,
			running: running || applyBusy,
			codewalkerLoopback: bridge?.codewalkerLoopback ?? false
		})
	);

	onMount(() => {
		serverConnection.load();
		void refreshCorpus();
		if (desktop) void refreshBridge();
	});

	onDestroy(() => stopPolling());

	async function refreshBridge() {
		if (!desktop) return;
		bridgeLoading = true;
		bridgeError = null;
		try {
			bridge = await getBridgeStatus({
				codewalkerUrl: settings.codewalkerUrl,
				localAiUrl: settings.allowLocalAi ? settings.localAiUrl : undefined,
				checkLocalAi: settings.allowLocalAi
			});
		} catch (e) {
			bridgeError = e instanceof Error ? e.message : 'bridge status failed';
			bridge = null;
		} finally {
			bridgeLoading = false;
		}
	}

	function applyPreset(id: string) {
		const p = presetById(id);
		if (!p) return;
		prompt = p.prompt;
		mode = p.mode;
	}

	async function generate() {
		if (!bridgeReady || !prompt.trim() || running) return;
		runError = null;
		applyResult = null;
		applyError = null;
		try {
			const out = await startRun({
				prompt,
				mode,
				provider: settings.provider,
				allowLocalAi: settings.allowLocalAi,
				localAiUrl: settings.localAiUrl,
				model: settings.model || undefined,
				codewalkerUrl: settings.codewalkerUrl
			});
			runId = out.runId;
			runStatus = null;
			startPolling();
		} catch (e) {
			runError = e instanceof Error ? e.message : 'failed to start run';
		}
	}

	function startPolling() {
		stopPolling();
		void pollOnce();
		pollTimer = setInterval(() => void pollOnce(), 800);
	}

	function stopPolling() {
		if (pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
	}

	async function pollOnce() {
		if (!runId) return;
		try {
			runStatus = await getRunStatus(runId);
			if (
				runStatus.phase === 'finished' ||
				runStatus.phase === 'failed' ||
				runStatus.phase === 'cancelled' ||
				runStatus.phase === 'timed_out'
			) {
				stopPolling();
				void refreshBridge(); // re-read copied RPF SHA after a run
			}
		} catch (e) {
			runError = e instanceof Error ? e.message : 'run status failed';
			stopPolling();
		}
	}

	async function cancel() {
		if (!runId) return;
		try {
			runStatus = await cancelRun(runId);
		} catch (e) {
			runError = e instanceof Error ? e.message : 'cancel failed';
		} finally {
			stopPolling();
		}
	}

	function openApply() {
		if (!applyState.enabled) return;
		applyError = null;
		showApplyModal = true;
	}

	async function confirmApply(confirmation: string) {
		if (!runId || !bridge) return;
		applyBusy = true;
		applyError = null;
		try {
			applyResult = await applyReviewedPlan({
				runId,
				confirmation,
				expectedCopiedRpfSha: bridge.expectedCopiedRpfSha,
				codewalkerUrl: settings.codewalkerUrl
			});
			showApplyModal = false;
			await refreshBridge();
			await pollOnce();
		} catch (e) {
			applyError = e instanceof Error ? e.message : 'apply failed';
		} finally {
			applyBusy = false;
		}
	}

	async function refreshCorpus() {
		corpusLoading = true;
		corpusError = null;
		try {
			const statusResponse = await getReduxCorpusStatus(serverConnection.serverUrl);
			status = statusResponse.status;
			if (status.latestBatchReport) {
				const [ds, rep, q] = await Promise.allSettled([
					getReduxCorpusDatasetSummary(serverConnection.serverUrl),
					getReduxCorpusLatestReport(serverConnection.serverUrl),
					getReduxCorpusQuarantine(serverConnection.serverUrl)
				]);
				dataset = ds.status === 'fulfilled' ? ds.value.summary : null;
				report = rep.status === 'fulfilled' ? rep.value.report : null;
				quarantineTotal = q.status === 'fulfilled' ? q.value.quarantine.total : null;
			} else {
				dataset = null;
				report = null;
				quarantineTotal = null;
			}
		} catch (caught) {
			corpusError = caught instanceof Error ? caught.message : 'Corpus context unavailable.';
			status = null;
		} finally {
			corpusLoading = false;
		}
	}

	async function copy(text: string, label: string) {
		try {
			await navigator.clipboard.writeText(text);
			actionMessage = `${label} copied.`;
		} catch {
			actionMessage = `${label}: ${text}`;
		}
		setTimeout(() => (actionMessage = null), 4000);
	}
</script>

<div class="studio">
	<ReduxMakerLocalBridgeStatus
		{desktop}
		{bridge}
		{bridgeLoading}
		{bridgeError}
		{corpusConnected}
		{corpusLoading}
		corpusUnavailable={!!corpusError}
		latestScan={report?.finishedAt ?? null}
	/>

	<div class="studio-body">
		<div class="cell blueprint"><ReduxMakerBlueprint {runStatus} /></div>

		<div class="cell center">
			<ReduxMakerReviewPane
				{corpusConnected}
				{runStatus}
				packagesScanned={dataset?.packagesScanned ?? report?.packagesScanned ?? null}
				datasetRecords={dataset?.datasetRecords ?? report?.datasetRecords ?? null}
				latestScan={report?.finishedAt ?? null}
				{corpusError}
			/>
			<ReduxMakerPromptDock
				{desktop}
				{bridgeReady}
				{running}
				bind:prompt
				bind:mode
				provider={settings.provider}
				{runError}
				onPreset={applyPreset}
				onGenerate={generate}
				onCancel={cancel}
				onCopyDevCommand={() => copy(DEV_COMMAND, 'Local dev command')}
			/>
		</div>

		<div class="cell dock">
			<ReduxMakerTelemetry
				{corpusConnected}
				{corpusLoading}
				{corpusError}
				{status}
				{dataset}
				{report}
				{quarantineTotal}
				{runStatus}
				{bridge}
				{applyResult}
			/>
			<ReduxMakerActionDock
				{bridgeReady}
				applyEnabled={applyState.enabled}
				applyReasons={applyState.reasons}
				{applyResult}
				{applyError}
				{actionMessage}
				standalonePath={STANDALONE_PATH}
				onApply={openApply}
				onOpenCorpus={() => goto('/redux-corpus')}
				onRefresh={() => {
					void refreshCorpus();
					void refreshBridge();
				}}
				onCopyPath={() => copy(STANDALONE_PATH, 'Redux Maker path')}
				onCopyDevCommand={() => copy(DEV_COMMAND, 'Local dev command')}
				onCopyRollback={(cmd) => copy(cmd, 'Rollback command')}
			/>
		</div>
	</div>
</div>

{#if showApplyModal && bridge}
	<ApplyConfirmModal
		confirmPhrase={bridge.applyConfirmPhrase}
		targetRpf={bridge.copiedRpfPath}
		expectedSha={bridge.expectedCopiedRpfSha}
		currentSha={bridge.copiedRpfSha}
		codewalkerUrl={bridge.codewalkerUrl}
		busy={applyBusy}
		error={applyError}
		onConfirm={confirmApply}
		onCancel={() => (showApplyModal = false)}
	/>
{/if}

<style>
	.studio { height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-surface); overflow: hidden; }
	.studio-body {
		flex: 1;
		min-height: 0;
		display: grid;
		gap: 1px;
		background: var(--color-border-tertiary);
		grid-template-columns: 248px minmax(0, 1fr) 312px;
		grid-template-rows: minmax(0, 1fr);
		grid-template-areas: 'left center right';
	}
	.cell { background: var(--bg-app); min-width: 0; min-height: 0; overflow: hidden; display: flex; flex-direction: column; }
	.blueprint { grid-area: left; }
	.center { grid-area: center; }
	.dock { grid-area: right; overflow: auto; }

	@media (max-width: 1200px) {
		.studio-body {
			grid-template-columns: 220px minmax(0, 1fr);
			grid-template-rows: minmax(0, 1.5fr) minmax(140px, 0.5fr);
			grid-template-areas: 'left center' 'left right';
		}
		.dock { flex-direction: row; gap: 1px; background: var(--color-border-tertiary); }
	}

	@media (max-width: 880px) {
		.studio-body {
			grid-template-columns: 1fr;
			grid-template-rows: auto minmax(0, 1fr) auto;
			grid-template-areas: 'left' 'center' 'right';
			overflow: auto;
		}
		.blueprint { max-height: 200px; }
		.dock { max-height: 280px; flex-direction: column; }
	}
</style>
