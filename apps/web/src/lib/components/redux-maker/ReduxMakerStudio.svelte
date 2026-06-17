<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import ReduxMakerLocalBridgeStatus from './ReduxMakerLocalBridgeStatus.svelte';
	import ReduxMakerBlueprint from './ReduxMakerBlueprint.svelte';
	import ReduxMakerReviewPane from './ReduxMakerReviewPane.svelte';
	import ReduxMakerPromptDock from './ReduxMakerPromptDock.svelte';
	import ReduxMakerTelemetry from './ReduxMakerTelemetry.svelte';
	import ReduxMakerActionDock from './ReduxMakerActionDock.svelte';
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

	const STANDALONE_PATH =
		'C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo\\apps\\redux-maker-ui';
	const DEV_COMMAND = `cd ${STANDALONE_PATH}; npm run tauri dev`;

	// Honest state: the local bridge is not implemented yet (H2.1), so there is
	// no run, no report, no module plan, no generated assets here.
	let status = $state<ReduxCorpusStatus | null>(null);
	let dataset = $state<ReduxCorpusDatasetSummary | null>(null);
	let report = $state<ReduxCorpusLatestReport | null>(null);
	let quarantineTotal = $state<number | null>(null);
	let corpusLoading = $state(false);
	let corpusError = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	const corpusConnected = $derived(!!status && status.scannerConfigured);

	onMount(() => {
		serverConnection.load();
		void refreshCorpus();
	});

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
		bridgeConnected={false}
		{corpusConnected}
		{corpusLoading}
		corpusUnavailable={!!corpusError}
		latestScan={report?.finishedAt ?? null}
	/>

	<div class="studio-body">
		<div class="cell blueprint"><ReduxMakerBlueprint /></div>

		<div class="cell center">
			<ReduxMakerReviewPane
				{corpusConnected}
				packagesScanned={dataset?.packagesScanned ?? report?.packagesScanned ?? null}
				datasetRecords={dataset?.datasetRecords ?? report?.datasetRecords ?? null}
				latestScan={report?.finishedAt ?? null}
				{corpusError}
			/>
			<ReduxMakerPromptDock
				bridgeConnected={false}
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
			/>
			<ReduxMakerActionDock
				bridgeConnected={false}
				{actionMessage}
				standalonePath={STANDALONE_PATH}
				onOpenCorpus={() => goto('/redux-corpus')}
				onRefresh={refreshCorpus}
				onCopyPath={() => copy(STANDALONE_PATH, 'Redux Maker path')}
				onCopyDevCommand={() => copy(DEV_COMMAND, 'Local dev command')}
			/>
		</div>
	</div>
</div>

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

	/* Medium windows: telemetry/action dock drops below the center; blueprint
	   keeps its column and spans both rows. */
	@media (max-width: 1200px) {
		.studio-body {
			grid-template-columns: 220px minmax(0, 1fr);
			grid-template-rows: minmax(0, 1.5fr) minmax(140px, 0.5fr);
			grid-template-areas: 'left center' 'left right';
		}
		.dock { flex-direction: row; gap: 1px; background: var(--color-border-tertiary); }
	}

	/* Small desktop windows: stack everything; blueprint + dock become short,
	   scrollable sections, the center (with the prompt) stays primary. */
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
