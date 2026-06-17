<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import {
		getReduxCorpusStatus,
		bootstrapReduxCorpus,
		startReduxCorpusScan,
		getReduxCorpusLatestReport,
		getReduxCorpusQuarantine,
		type ReduxCorpusStatus,
		type ReduxCorpusLatestReport,
		type ReduxCorpusQuarantineSummary
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let status = $state<ReduxCorpusStatus | null>(null);
	let report = $state<ReduxCorpusLatestReport | null>(null);
	let quarantine = $state<ReduxCorpusQuarantineSummary | null>(null);
	let loading = $state(false);
	let busy = $state(false);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	const canScan = $derived(
		!!status && status.enabled && status.scannerConfigured && !status.activeJob
	);

	onMount(() => {
		serverConnection.load();
		void refresh();
	});

	async function refresh() {
		loading = true;
		error = null;
		try {
			const response = await getReduxCorpusStatus(serverConnection.serverUrl);
			status = response.status;
			report = status.latestBatchReport ?? null;
			// Best-effort: load quarantine only when a report exists.
			if (status.latestBatchReport) {
				await loadQuarantine();
			}
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load corpus status.';
		} finally {
			loading = false;
		}
	}

	async function bootstrap() {
		busy = true;
		actionMessage = null;
		error = null;
		try {
			const response = await bootstrapReduxCorpus(serverConnection.serverUrl);
			actionMessage = `Corpus folders ready on '${response.rootId}': ${response.created.length} created, ${response.existing.length} existed.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not bootstrap folders.';
		} finally {
			busy = false;
		}
	}

	async function scan() {
		busy = true;
		actionMessage = null;
		error = null;
		try {
			const response = await startReduxCorpusScan(serverConnection.serverUrl);
			actionMessage = `Corpus scan started (job ${response.job.id}). Follow progress on the Jobs page.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start corpus scan.';
		} finally {
			busy = false;
		}
	}

	async function loadReport() {
		error = null;
		try {
			const response = await getReduxCorpusLatestReport(serverConnection.serverUrl);
			report = response.report;
			await loadQuarantine();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'No report available yet.';
		}
	}

	async function loadQuarantine() {
		try {
			const response = await getReduxCorpusQuarantine(serverConnection.serverUrl);
			quarantine = response.quarantine;
		} catch {
			quarantine = null;
		}
	}

	function openInboxInFiles() {
		// Reuse the existing Files route-state handoff (same mechanism Projects uses).
		try {
			localStorage.setItem(
				'homeops.files.openTarget',
				JSON.stringify({ rootId: status?.smartPoolRootId ?? 'bulk', path: 'redux-corpus/inbox' })
			);
		} catch {
			// Non-fatal: the path is still shown below for manual navigation.
		}
		void goto('/files');
	}

	async function copyInbox() {
		if (!status) return;
		try {
			await navigator.clipboard.writeText(status.inboxRoot);
			actionMessage = 'Inbox path copied to clipboard.';
		} catch {
			actionMessage = `Inbox path: ${status.inboxRoot}`;
		}
	}

	function formatBytes(bytes: number | null) {
		if (bytes === null) return '—';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

</script>

<svelte:head><title>Redux Corpus · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Redux Corpus" flush>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-folder-plus" label="Bootstrap folders" onclick={bootstrap} />
		<SmallButton
			icon="ti-player-play"
			label={busy ? 'Working…' : 'Scan corpus'}
			onclick={scan}
			disabled={!canScan}
		/>
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if actionMessage}<div class="notice success">{actionMessage}</div>{/if}

		{#if status}
			<!-- 1. Corpus status -->
			<section class="panel">
				<div class="panel-head">
					<div>Corpus status</div>
					<span class="health {status.enabled && status.scannerConfigured ? 'healthy' : 'degraded'}">
						{status.enabled ? (status.scannerConfigured ? 'ready' : 'scanner missing') : 'disabled'}
					</span>
				</div>
				<div class="status-grid">
					<div class="stat"><span>Enabled</span><strong>{status.enabled ? 'yes' : 'no'}</strong></div>
					<div class="stat"><span>Scanner configured</span><strong>{status.scannerConfigured ? 'yes' : 'no'}</strong></div>
					<div class="stat"><span>Smart pool root</span><strong>{status.smartPoolRootId ?? '—'}</strong></div>
					<div class="stat"><span>Directories OK</span><strong>{status.directoriesOk ? 'yes' : 'no'}</strong></div>
					<div class="stat"><span>Disk free (bulk)</span><strong>{formatBytes(status.diskFree)}</strong></div>
					<div class="stat"><span>Scanner version</span><strong>{status.scannerVersion ?? '—'}</strong></div>
				</div>
				<div class="paths">
					<div><span>scanner</span><code>{status.scannerPath}</code></div>
					<div><span>corpus root</span><code>{status.corpusRoot}</code></div>
				</div>
				{#if !status.scannerConfigured}
					<p class="note">
						The scanner binary is not installed at the configured path. Deploy it to
						<code>{status.scannerPath}</code> on the server, then refresh. Scanning stays disabled until then.
					</p>
				{/if}
			</section>

			<!-- Active job -->
			{#if status.activeJob}
				<section class="panel">
					<div class="panel-head"><div>Active scan</div><span>{status.activeJob.status}</span></div>
					<div class="active">
						<div class="active-title">{status.activeJob.title}</div>
						<div class="active-id">job {status.activeJob.id}</div>
						<div class="bar wide"><div style={`width:${status.activeJob.progress}%`}></div></div>
						<p class="note">A corpus scan is running. Only one runs at a time. Follow logs on the Jobs page.</p>
					</div>
				</section>
			{/if}

			<!-- 2. Inbox / drop-zone -->
			<section class="panel">
				<div class="panel-head"><div>Inbox / drop-zone</div><span>bulk root</span></div>
				<div class="inbox">
					<p>Drop Redux / mod packages (folders, <code>.zip</code>, <code>.oiv</code>) into the inbox, then run a scan.</p>
					<code class="inbox-path">{status.inboxRoot}</code>
					<div class="inbox-actions">
						<SmallButton icon="ti-folder-open" label="Open inbox in Files" onclick={openInboxInFiles} />
						<SmallButton icon="ti-copy" label="Copy inbox path" onclick={copyInbox} />
					</div>
					<p class="note">Uploads go through the Files page. Sources are never modified — scanning is read-only.</p>
				</div>
			</section>

			<!-- 3. Actions -->
			<section class="panel">
				<div class="panel-head"><div>Actions</div></div>
				<div class="actions">
					<SmallButton icon="ti-folder-plus" label="Bootstrap folders" onclick={bootstrap} disabled={busy} />
					<SmallButton icon="ti-player-play" label="Scan corpus / build dataset" onclick={scan} disabled={!canScan} />
					<SmallButton icon="ti-refresh" label="Refresh" onclick={refresh} />
					<SmallButton icon="ti-report" label="View latest report" onclick={loadReport} />
					<SmallButton icon="ti-alert-triangle" label="View quarantine" onclick={loadQuarantine} />
				</div>
			</section>

			<!-- 4. Latest batch report -->
			<section class="panel">
				<div class="panel-head"><div>Latest batch report</div>{#if report?.finishedAt}<span>{report.finishedAt}</span>{/if}</div>
				{#if report}
					<div class="status-grid">
						<div class="stat"><span>Packages total</span><strong>{report.packagesTotal}</strong></div>
						<div class="stat"><span>Scanned</span><strong>{report.packagesScanned}</strong></div>
						<div class="stat"><span>Quarantined</span><strong>{report.packagesQuarantined}</strong></div>
						<div class="stat"><span>Dataset records</span><strong>{report.datasetRecords}</strong></div>
					</div>
				{:else}
					<p class="note">No report yet. Run a scan to build the dataset.</p>
				{/if}
			</section>

			<!-- 5. Quarantine -->
			<section class="panel">
				<div class="panel-head"><div>Quarantine</div><span>{quarantine?.total ?? 0} records</span></div>
				{#if quarantine && quarantine.entries.length}
					<div class="q-list">
						{#each quarantine.entries as entry}
							<div class="q-row">
								<div class="q-name">{entry.packageName || entry.packageId}</div>
								<div class="q-reason">{entry.reason}</div>
								<div class="q-meta"><span>{entry.detectedKind}</span><span>{formatBytes(entry.sizeBytes)}</span></div>
							</div>
						{/each}
					</div>
					<p class="note">Quarantine is metadata-only. Sources are left untouched — there is no delete action.</p>
				{:else}
					<p class="note">No quarantined packages.</p>
				{/if}
			</section>

			<!-- 6. Output files -->
			<section class="panel">
				<div class="panel-head"><div>Output files</div><span>bulk/redux-corpus</span></div>
				<div class="outputs">
					<div><span>reports</span><code>{status.reportRoot}/batch_report.json</code></div>
					<div><span>reports</span><code>{status.reportRoot}/batch_report.md</code></div>
					<div><span>datasets</span><code>{status.datasetRoot}/corpus_dataset_records.jsonl</code></div>
					<div><span>datasets</span><code>{status.datasetRoot}/corpus_coverage_report.md</code></div>
					<div><span>reports</span><code>{status.reportRoot}/quarantine_records.jsonl</code></div>
				</div>
				<p class="note">
					Read-only metadata only — no raw asset preview, no AI generation, no CodeWalker, no RPF apply.
					Live Redux apply remains in the local desktop Redux Maker, never on this server.
				</p>
			</section>
		{:else if loading}
			<div class="notice">Loading corpus status…</div>
		{:else}
			<div class="notice">No corpus status loaded yet.</div>
		{/if}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.panel { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 13px; font-weight: 600; }
	.panel-head span { color: var(--color-text-secondary); font-size: 11px; }
	.health { text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; padding: 2px 8px; border-radius: 999px; border: 0.5px solid var(--color-border-tertiary); }
	.health.healthy { color: var(--color-text-success); }
	.health.degraded { color: var(--color-text-danger); }
	.status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; padding: 14px; }
	.stat span { display: block; color: var(--color-text-tertiary); font-size: 11px; }
	.stat strong { display: block; margin-top: 4px; color: var(--color-text-primary); font-size: 16px; font-weight: 600; }
	.paths { padding: 0 14px 12px; display: flex; flex-direction: column; gap: 6px; }
	.paths div { display: flex; gap: 8px; align-items: baseline; }
	.paths span { color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; min-width: 80px; }
	.note { padding: 0 14px 12px; color: var(--color-text-tertiary); font-size: 11px; }
	.bar.wide { display: block; width: calc(100% - 28px); height: 5px; margin: 8px 14px 10px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; }
	.bar.wide div { height: 100%; background: var(--accent); }
	.active { padding: 12px 14px; }
	.active-title { color: var(--color-text-primary); font-size: 13px; }
	.active-id { color: var(--color-text-tertiary); font-size: 11px; margin-top: 2px; }
	.inbox { padding: 12px 14px; color: var(--color-text-secondary); font-size: 12px; }
	.inbox-path { display: block; margin: 8px 0; }
	.inbox-actions { display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0; }
	.actions { display: flex; flex-wrap: wrap; gap: 8px; padding: 12px 14px; }
	.q-list { display: flex; flex-direction: column; }
	.q-row { display: grid; grid-template-columns: 1.4fr 2fr 1fr; gap: 12px; padding: 9px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); align-items: center; }
	.q-row:last-child { border-bottom: 0; }
	.q-name { color: var(--color-text-primary); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.q-reason { color: var(--color-text-danger); font-size: 12px; }
	.q-meta { display: flex; gap: 10px; justify-content: flex-end; color: var(--color-text-tertiary); font-size: 11px; }
	.outputs { padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; }
	.outputs div { display: flex; gap: 8px; align-items: baseline; }
	.outputs span { color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; min-width: 70px; }
	code { font-family: var(--font-mono); color: var(--accent); font-size: 11px; overflow-wrap: anywhere; }
</style>
