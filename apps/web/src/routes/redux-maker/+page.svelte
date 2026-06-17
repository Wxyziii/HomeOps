<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
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

	const STANDALONE_PATH = 'C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo\\apps\\redux-maker-ui';
	const DEV_COMMAND = `cd ${STANDALONE_PATH}; npm run tauri dev`;

	let status = $state<ReduxCorpusStatus | null>(null);
	let dataset = $state<ReduxCorpusDatasetSummary | null>(null);
	let report = $state<ReduxCorpusLatestReport | null>(null);
	let quarantineTotal = $state<number | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	const workflowCards = [
		{ icon: 'ti-wand', title: 'Create Module Plan', desc: 'Prompt → deterministic PatchPlan. Runs in the local Redux Maker app.' },
		{ icon: 'ti-report-search', title: 'Review Report', desc: 'Inspect generated stage/bundle + diff before any apply.' },
		{ icon: 'ti-checks', title: 'Apply-ready Proof', desc: 'SHA + confirm gated copied-RPF apply, with rollback. Local only.' },
		{ icon: 'ti-database', title: 'Corpus Context', desc: 'Server-built Redux corpus dataset feeds planning context.' },
		{ icon: 'ti-settings', title: 'Settings', desc: 'Local AI provider + safety toggles live in the Redux Maker app.' }
	];

	onMount(() => {
		serverConnection.load();
		void refresh();
	});

	async function refresh() {
		loading = true;
		error = null;
		try {
			const statusResponse = await getReduxCorpusStatus(serverConnection.serverUrl);
			status = statusResponse.status;
			// Dataset/report only meaningful once a scan has produced a report.
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
			error = caught instanceof Error ? caught.message : 'Could not load corpus context.';
		} finally {
			loading = false;
		}
	}

	async function copy(text: string, label: string) {
		try {
			await navigator.clipboard.writeText(text);
			actionMessage = `${label} copied to clipboard.`;
		} catch {
			actionMessage = `${label}: ${text}`;
		}
	}
</script>

<svelte:head><title>Redux Maker · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Redux Maker Studio" flush>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-package" label="Open Redux Corpus" onclick={() => goto('/redux-corpus')} />
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if actionMessage}<div class="notice success">{actionMessage}</div>{/if}

		<!-- 1. Header -->
		<section class="panel">
			<div class="panel-head">
				<div>Redux Maker Studio</div>
				<span class="health degraded">local bridge not connected</span>
			</div>
			<div class="header-body">
				<p>
					Brings the Redux Maker workspace into HomeOps as a first-class module. AI planning,
					CodeWalker, copied-RPF apply, and rollback still run in the <strong>local desktop Redux
					Maker app</strong> until the safe local bridge lands (H2.1).
				</p>
				<div class="header-meta">
					<span>Corpus server: <strong>{status ? (status.scannerConfigured ? 'scanner ready' : 'scanner missing') : '…'}</strong></span>
					<span>Latest scan: <strong>{report ? (report.finishedAt ?? 'done') : 'none yet'}</strong></span>
				</div>
			</div>
		</section>

		<!-- 2. Workflow cards -->
		<section class="panel">
			<div class="panel-head"><div>Workflow</div></div>
			<div class="cards">
				{#each workflowCards as card}
					<div class="card">
						<i class="ti {card.icon}" aria-hidden="true"></i>
						<div><strong>{card.title}</strong><span>{card.desc}</span></div>
					</div>
				{/each}
			</div>
		</section>

		<!-- 3. Corpus context -->
		<section class="panel">
			<div class="panel-head">
				<div>Corpus context</div>
				<a class="link" href="/redux-corpus">open corpus →</a>
			</div>
			{#if dataset || report}
				<div class="status-grid">
					<div class="stat"><span>Packages scanned</span><strong>{dataset?.packagesScanned ?? report?.packagesScanned ?? 0}</strong></div>
					<div class="stat"><span>Dataset records</span><strong>{dataset?.datasetRecords ?? report?.datasetRecords ?? 0}</strong></div>
					<div class="stat"><span>Feature records</span><strong>{dataset?.featureRecords ?? 0}</strong></div>
					<div class="stat"><span>Target patterns</span><strong>{dataset?.targetPatterns ?? 0}</strong></div>
					<div class="stat"><span>Quarantined</span><strong>{quarantineTotal ?? dataset?.packagesQuarantined ?? 0}</strong></div>
					<div class="stat"><span>Latest scan</span><strong>{report?.finishedAt ?? 'done'}</strong></div>
				</div>
			{:else}
				<p class="note">No corpus dataset yet. Build one on the <a class="link" href="/redux-corpus">Redux Corpus</a> page, then refresh.</p>
			{/if}
		</section>

		<!-- 4. Local maker panel -->
		<section class="panel">
			<div class="panel-head"><div>Local Redux Maker</div><span>desktop app</span></div>
			<div class="local">
				<div class="kv"><span>Standalone app path</span><code>{STANDALONE_PATH}</code></div>
				<p class="note">
					AI generation, CodeWalker, copied-RPF apply, and rollback still run in the local Redux
					Maker app until the safe local bridge is added (H2.1).
				</p>
				<p class="note safe">🛡 The HomeOps server does not edit RPF files. No server-side apply, no
					server CodeWalker, no RPF write endpoints.</p>
			</div>
		</section>

		<!-- 5. Action buttons -->
		<section class="panel">
			<div class="panel-head"><div>Actions</div></div>
			<div class="actions">
				<SmallButton icon="ti-package" label="Open Redux Corpus" onclick={() => goto('/redux-corpus')} />
				<SmallButton icon="ti-copy" label="Copy Redux Maker path" onclick={() => copy(STANDALONE_PATH, 'Redux Maker path')} />
				<SmallButton icon="ti-terminal-2" label="Copy local dev command" onclick={() => copy(DEV_COMMAND, 'Dev command')} />
				<SmallButton icon="ti-refresh" label="Refresh corpus status" onclick={refresh} />
			</div>
			<p class="note">No Generate/Apply here — those remain in the local Redux Maker app for safety.</p>
		</section>
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { color: var(--color-text-success); background: var(--color-background-success); }
	.panel { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 13px; font-weight: 600; }
	.panel-head span { color: var(--color-text-secondary); font-size: 11px; }
	.link { color: var(--accent); font-size: 11px; }
	.health { text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; padding: 2px 8px; border-radius: 999px; border: 0.5px solid var(--color-border-tertiary); }
	.health.degraded { color: var(--color-text-warning); }
	.header-body { padding: 12px 14px; color: var(--color-text-secondary); font-size: 12px; }
	.header-meta { margin-top: 10px; display: flex; flex-wrap: wrap; gap: 16px; color: var(--color-text-tertiary); font-size: 11px; }
	.header-meta strong { color: var(--color-text-primary); }
	.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 10px; padding: 12px; }
	.card { display: flex; gap: 10px; align-items: flex-start; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-sidebar); padding: 10px; }
	.card i { color: var(--accent); font-size: 16px; margin-top: 1px; }
	.card strong { display: block; color: var(--color-text-primary); font-size: 12px; }
	.card span { color: var(--color-text-tertiary); font-size: 11px; }
	.status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; padding: 14px; }
	.stat span { display: block; color: var(--color-text-tertiary); font-size: 11px; }
	.stat strong { display: block; margin-top: 4px; color: var(--color-text-primary); font-size: 16px; font-weight: 600; }
	.local { padding: 12px 14px; }
	.kv { display: flex; flex-direction: column; gap: 4px; }
	.kv span { color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
	.note { color: var(--color-text-tertiary); font-size: 11px; margin: 8px 0 0; }
	.note.safe { color: var(--color-text-success); }
	.actions { display: flex; flex-wrap: wrap; gap: 8px; padding: 12px 14px; }
	code { font-family: var(--font-mono); color: var(--accent); font-size: 11px; overflow-wrap: anywhere; }
	@media (max-width: 700px) { .content { padding: 12px; } }
</style>
