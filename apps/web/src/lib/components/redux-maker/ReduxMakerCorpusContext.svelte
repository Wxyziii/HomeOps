<script lang="ts">
	import {
		getReduxCorpusDatasetRecords,
		type ReduxCorpusStatus,
		type ReduxCorpusDatasetSummary,
		type ReduxCorpusLatestReport,
		type ReduxCorpusDatasetRecord
	} from '$lib/api/client';

	let {
		status = null,
		dataset = null,
		report = null,
		quarantineTotal = null,
		loading = false,
		error = null,
		serverUrl = '',
		onAttach = (_records: ReduxCorpusDatasetRecord[]) => {}
	}: {
		status?: ReduxCorpusStatus | null;
		dataset?: ReduxCorpusDatasetSummary | null;
		report?: ReduxCorpusLatestReport | null;
		quarantineTotal?: number | null;
		loading?: boolean;
		error?: string | null;
		serverUrl?: string;
		onAttach?: (records: ReduxCorpusDatasetRecord[]) => void;
	} = $props();

	const stats = $derived([
		{ label: 'Packages scanned', value: dataset?.packagesScanned ?? report?.packagesScanned ?? null },
		{ label: 'Dataset records', value: dataset?.datasetRecords ?? report?.datasetRecords ?? null },
		{ label: 'Target patterns', value: dataset?.targetPatterns ?? null },
		{ label: 'Quarantined', value: quarantineTotal ?? dataset?.packagesQuarantined ?? null }
	]);

	const categoryOptions = $derived(
		dataset?.categoryCoverage ? Object.keys(dataset.categoryCoverage).sort() : []
	);
	const hasDataset = $derived(!!(report || dataset) && (dataset?.datasetRecords ?? report?.datasetRecords ?? 0) > 0);

	let category = $state('');
	let targetPattern = $state('');
	let records = $state<ReduxCorpusDatasetRecord[]>([]);
	let selected = $state<Set<string>>(new Set());
	let recordsLoading = $state(false);
	let recordsError = $state<string | null>(null);
	let totalMatched = $state<number | null>(null);
	let truncated = $state(false);

	async function fetchRecords() {
		recordsLoading = true;
		recordsError = null;
		try {
			const res = await getReduxCorpusDatasetRecords(serverUrl, {
				category: category || undefined,
				targetPattern: targetPattern || undefined,
				limit: 50
			});
			records = res.result.records;
			totalMatched = res.result.totalMatched;
			truncated = res.result.truncated;
			selected = new Set();
		} catch (e) {
			recordsError = e instanceof Error ? e.message : 'records unavailable';
			records = [];
			totalMatched = null;
		} finally {
			recordsLoading = false;
		}
	}

	function toggle(id: string) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	function attach() {
		const chosen = records.filter((r) => selected.has(r.id));
		if (chosen.length) onAttach(chosen);
	}
</script>

<div class="corpus">
	<div class="card-title">Corpus context <a class="link" href="/redux-corpus">open →</a></div>
	{#if loading}
		<div class="msg">Loading corpus context…</div>
	{:else if error}
		<div class="msg err">Corpus context unavailable: {error}</div>
	{:else if !status}
		<div class="msg">Corpus status unavailable.</div>
	{:else if !hasDataset}
		<div class="msg">No corpus dataset yet. Build one on the Redux Corpus page.</div>
	{:else}
		<div class="grid">
			{#each stats as stat}
				<div class="stat"><span>{stat.label}</span><strong>{stat.value ?? '—'}</strong></div>
			{/each}
		</div>

		<div class="retrieval">
			<div class="filters">
				<select bind:value={category} aria-label="category">
					<option value="">all categories</option>
					{#each categoryOptions as c}<option value={c}>{c}</option>{/each}
				</select>
				<input
					type="text"
					bind:value={targetPattern}
					placeholder="target pattern (e.g. frontend.ytd)"
					aria-label="target pattern"
				/>
				<button class="btn" type="button" onclick={fetchRecords} disabled={recordsLoading}>
					{recordsLoading ? '…' : 'Find'}
				</button>
			</div>

			{#if recordsError}
				<div class="msg err">{recordsError}</div>
			{:else if totalMatched !== null}
				<div class="result-head">
					{totalMatched} match{totalMatched === 1 ? '' : 'es'}{truncated ? ' (showing first 50)' : ''} ·
					{selected.size} selected
				</div>
				<div class="cards">
					{#each records as r (r.id)}
						<button class="card" class:sel={selected.has(r.id)} type="button" onclick={() => toggle(r.id)}>
							<div class="card-head">
								<span class="cat">{r.category}</span>
								<span class="conf">{r.confidence.toFixed(2)}</span>
							</div>
							<div class="intent">{r.intent}</div>
							{#if r.targetPatterns.length}
								<div class="targets">{r.targetPatterns.slice(0, 3).join(' · ')}{r.targetPatterns.length > 3 ? ' …' : ''}</div>
							{/if}
							{#if r.blockedReasons.length}
								<div class="blocked">⚠ {r.blockedReasons[0]}</div>
							{/if}
						</button>
					{:else}
						<div class="msg">No records for this filter.</div>
					{/each}
				</div>
				<button class="btn attach" type="button" disabled={selected.size === 0} onclick={attach}>
					Attach Context to Prompt ({selected.size})
				</button>
			{:else}
				<div class="msg">Filter and click Find to retrieve safe context records.</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.corpus { padding: 10px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.card-title { display: flex; align-items: center; justify-content: space-between; color: var(--color-text-primary); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.link { color: var(--accent); font-size: 10px; text-transform: none; letter-spacing: 0; }
	.msg { margin-top: 8px; color: var(--color-text-tertiary); font-size: 11px; }
	.msg.err { color: var(--color-text-danger); }
	.grid { margin-top: 8px; display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
	.stat { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface); padding: 6px 8px; }
	.stat span { display: block; color: var(--text-faint); font-size: 9.5px; text-transform: uppercase; letter-spacing: 0.04em; }
	.stat strong { display: block; margin-top: 2px; color: var(--color-text-primary); font-size: 14px; }
	.retrieval { margin-top: 10px; border-top: 0.5px solid var(--color-border-tertiary); padding-top: 8px; }
	.filters { display: flex; gap: 5px; flex-wrap: wrap; }
	.filters select, .filters input { flex: 1; min-width: 90px; padding: 5px 7px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-size: 11px; }
	.btn { padding: 5px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-secondary); font-size: 11px; cursor: pointer; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn.attach { width: 100%; margin-top: 8px; background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.result-head { margin-top: 8px; color: var(--text-faint); font-size: 10px; }
	.cards { margin-top: 6px; display: flex; flex-direction: column; gap: 5px; max-height: 240px; overflow: auto; }
	.card { text-align: left; border: 0.5px solid var(--color-border-tertiary); border-radius: 5px; background: var(--bg-surface); padding: 6px 8px; cursor: pointer; }
	.card.sel { border-color: var(--accent); background: var(--orange-bg); }
	.card-head { display: flex; justify-content: space-between; }
	.cat { color: var(--color-text-primary); font-size: 11px; font-weight: 600; }
	.conf { color: var(--text-faint); font-size: 10px; font-family: var(--font-mono); }
	.intent { color: var(--color-text-tertiary); font-size: 10.5px; margin-top: 2px; }
	.targets { color: var(--text-faint); font-family: var(--font-mono); font-size: 9.5px; margin-top: 3px; overflow-wrap: anywhere; }
	.blocked { color: var(--color-text-warning); font-size: 9.5px; margin-top: 3px; }
</style>
