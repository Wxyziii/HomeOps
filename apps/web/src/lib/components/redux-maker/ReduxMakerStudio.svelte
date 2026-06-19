<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import '$lib/redux-maker/studio.css';
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
		readReportFile,
		runStatusFromReport,
		type BridgeStatus,
		type RunStatus,
		type ApplyOutput
	} from '$lib/redux-maker/bridge';
	import ReduxMakerSettingsModal from './ReduxMakerSettingsModal.svelte';
	import {
		loadBridgeSettings,
		saveBridgeSettings,
		type BridgeSettings
	} from '$lib/redux-maker/bridgeSettings';
	import { presetById, type RunMode } from '$lib/redux-maker/presets';
	import {
		buildContextPack,
		buildStructuredContextPack,
		composePrompt,
		serializeStructuredContextPack,
		type ContextPack
	} from '$lib/redux-maker/contextPack';
	import {
		loadRunHistory,
		upsertRun,
		patchRun,
		clearRunHistory,
		type RunHistoryEntry
	} from '$lib/redux-maker/runHistory';
	import type { ReduxCorpusDatasetRecord } from '$lib/api/client';

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

	let attachedContext = $state<ContextPack | null>(null);
	// H2.3 — raw selected records kept so a STRUCTURED context pack can be built
	// (against the live prompt) and passed to the engine via --context-pack.
	let attachedRecords = $state<ReduxCorpusDatasetRecord[]>([]);
	let presetId = $state<string | null>(null);
	let history = $state<RunHistoryEntry[]>([]);
	let showSettings = $state(false);

	function saveSettings(next: BridgeSettings) {
		saveBridgeSettings(next);
		settings = next;
		showSettings = false;
		if (desktop) void refreshBridge();
	}

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
		history = loadRunHistory();
		void refreshCorpus();
		if (desktop) void refreshBridge();
	});

	function attachContext(records: ReduxCorpusDatasetRecord[]) {
		attachedContext = buildContextPack(records);
		attachedRecords = records;
		actionMessage = `Attached ${records.length} corpus record(s) to prompt.`;
		setTimeout(() => (actionMessage = null), 3000);
	}

	function clearContext() {
		attachedContext = null;
		attachedRecords = [];
	}

	async function loadHistoryRun(entry: RunHistoryEntry) {
		runError = null;
		applyResult = null;
		presetId = entry.presetId;
		prompt = entry.prompt;
		runId = entry.runId;
		stopPolling();
		try {
			runStatus = await getRunStatus(entry.runId);
		} catch {
			// Job not in memory (e.g. after restart) — read the stored report file.
			try {
				const file = await readReportFile(entry.mvpReportPath);
				const report = JSON.parse(file.content) as Record<string, unknown>;
				runStatus = runStatusFromReport(entry.runId, entry.mvpReportPath, report);
			} catch (e) {
				runError = e instanceof Error ? e.message : 'could not load run report';
				runStatus = null;
			}
		}
	}

	function clearHistory() {
		history = clearRunHistory();
	}

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
		presetId = id;
	}

	async function generate() {
		if (!bridgeReady || !prompt.trim() || running) return;
		runError = null;
		applyResult = null;
		applyError = null;
		// The visible context block is still appended to the prompt (so the user
		// always sees it). H2.3 ALSO sends a structured context pack to the engine
		// via --context-pack, built from the selected records against this prompt.
		const composed = composePrompt(prompt, attachedContext?.text ?? null);
		const contextPackJson =
			attachedRecords.length > 0
				? serializeStructuredContextPack(buildStructuredContextPack(attachedRecords, prompt))
				: undefined;
		try {
			const out = await startRun({
				prompt: composed,
				presetId: presetId ?? undefined,
				mode,
				provider: settings.provider,
				allowLocalAi: settings.allowLocalAi,
				localAiUrl: settings.localAiUrl,
				model: settings.model || undefined,
				codewalkerUrl: settings.codewalkerUrl,
				fallbackToRuleBased: settings.provider !== 'rule_based',
				contextPackJson
			});
			runId = out.runId;
			runStatus = null;
			const now = new Date().toISOString();
			history = upsertRun({
				runId: out.runId,
				prompt,
				presetId,
				provider: settings.provider,
				mode,
				status: 'running',
				outDir: out.outDir,
				mvpReportPath: out.mvpReportPath,
				readyToApply: null,
				applied: false,
				corpusContextAttached: !!attachedContext,
				contextRecordCount: attachedContext?.recordCount ?? 0,
				contextCategories: attachedContext?.categories ?? [],
				copiedRpfShaBefore: bridge?.copiedRpfSha ?? null,
				copiedRpfShaAfter: null,
				createdAt: now,
				updatedAt: now
			});
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
				history = patchRun(runId, {
					status: runStatus.phase,
					readyToApply: runStatus.readyToApply,
					applied: runStatus.applied
				});
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
			history = patchRun(runId, {
				applied: applyResult.applied,
				status: applyResult.applied ? 'applied' : 'apply_failed',
				copiedRpfShaBefore: applyResult.shaBefore,
				copiedRpfShaAfter: applyResult.shaAfter ?? null
			});
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

<div class="rm-studio">
	<ReduxMakerLocalBridgeStatus
		{desktop}
		{bridge}
		{bridgeLoading}
		{bridgeError}
		{corpusConnected}
		{runStatus}
		onRefresh={() => {
			void refreshCorpus();
			void refreshBridge();
		}}
		onOpenSettings={() => (showSettings = true)}
		onOpenCorpus={() => goto('/redux-corpus')}
	/>

	<div class="workspace">
		<ReduxMakerBlueprint {runStatus} />

		<main class="center">
			<ReduxMakerReviewPane {runStatus} {running} />
			<ReduxMakerPromptDock
				{desktop}
				{bridgeReady}
				{running}
				bind:prompt
				bind:mode
				provider={settings.provider}
				{runStatus}
				{runError}
				{attachedContext}
				onPreset={applyPreset}
				onGenerate={generate}
				onCancel={cancel}
				onClearContext={clearContext}
			/>
		</main>

		<aside class="sidebar-r">
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
				{attachedContext}
				serverUrl={serverConnection.serverUrl}
				onAttach={attachContext}
			/>
			<ReduxMakerActionDock
				{bridgeReady}
				applyEnabled={applyState.enabled}
				applyReasons={applyState.reasons}
				{applyResult}
				{applyError}
				{actionMessage}
				{history}
				scannerPath={bridge?.scannerPath ?? ''}
				copiedRpfPath={bridge?.copiedRpfPath ?? ''}
				onApply={openApply}
				onLoadRun={loadHistoryRun}
				onClearHistory={clearHistory}
				onOpenSettings={() => (showSettings = true)}
				onOpenCorpus={() => goto('/redux-corpus')}
				onRefresh={() => {
					void refreshCorpus();
					void refreshBridge();
				}}
				onCopyScannerPath={() => copy(bridge?.scannerPath ?? '', 'Scanner path')}
				onCopyRpfPath={() => copy(bridge?.copiedRpfPath ?? '', 'Copied RPF path')}
				onCopyDevCommand={() => copy(DEV_COMMAND, 'Local dev command')}
				onCopyRollback={(cmd) => copy(cmd, 'Rollback command')}
			/>
		</aside>
	</div>
</div>

{#if showSettings}
	<ReduxMakerSettingsModal
		{settings}
		localAiReachable={bridge?.localAiReachable ?? null}
		onSave={saveSettings}
		onCancel={() => (showSettings = false)}
	/>
{/if}

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
