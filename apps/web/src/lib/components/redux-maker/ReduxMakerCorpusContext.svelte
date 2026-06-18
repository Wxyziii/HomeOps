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
		corpusConnected = false,
		serverUrl = '',
		onAttach = (_records: ReduxCorpusDatasetRecord[]) => {}
	}: {
		status?: ReduxCorpusStatus | null;
		dataset?: ReduxCorpusDatasetSummary | null;
		report?: ReduxCorpusLatestReport | null;
		quarantineTotal?: number | null;
		loading?: boolean;
		error?: string | null;
		corpusConnected?: boolean;
		serverUrl?: string;
		onAttach?: (records: ReduxCorpusDatasetRecord[]) => void;
	} = $props();

	const stats = $derived([
		{ label: 'Dataset records', value: dataset?.datasetRecords ?? report?.datasetRecords ?? null },
		{ label: 'Target patterns', value: dataset?.targetPatterns ?? null }
	]);
	const categoryOptions = $derived(dataset?.categoryCoverage ? Object.keys(dataset.categoryCoverage).sort() : []);
	const hasDataset = $derived(!!(report || dataset) && (dataset?.datasetRecords ?? report?.datasetRecords ?? 0) > 0);

	let category = $state('');
	let targetPattern = $state('');
	let records = $state<ReduxCorpusDatasetRecord[]>([]);
	let selected = $state<Set<string>>(new Set());
	let recordsLoading = $state(false);
	let recordsError = $state<string | null>(null);
	let totalMatched = $state<number | null>(null);

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
	<div class="tele-row" style="margin-bottom:0">
		<span class="tele-title">corpus context</span>
		<a class="link-btn" href="/redux-corpus">open →</a>
	</div>
	{#if loading}
		<div class="prog-sub" style="margin-top:8px">Loading corpus context…</div>
	{:else if error}
		<div class="prog-sub c-red" style="margin-top:8px">Corpus unavailable: {error}</div>
	{:else if !status}
		<div class="prog-sub" style="margin-top:8px">Corpus status unavailable.</div>
	{:else if !hasDataset}
		<div class="prog-sub" style="margin-top:8px">No corpus dataset yet. Build one on the Redux Corpus page.</div>
	{:else}
		<div class="corpus-grid">
			{#each stats as stat}
				<div class="stat-card"><div class="stat-label">{stat.label}</div><div class="stat-val c-text1">{stat.value ?? '—'}</div></div>
			{/each}
		</div>
		<div class="filters">
			<select bind:value={category} aria-label="category">
				<option value="">all categories</option>
				{#each categoryOptions as c}<option value={c}>{c}</option>{/each}
			</select>
			<input type="text" bind:value={targetPattern} placeholder="target pattern" aria-label="target pattern" />
			<button class="btn-secondary" type="button" style="padding:5px 10px" onclick={fetchRecords} disabled={recordsLoading}>{recordsLoading ? '…' : 'Find'}</button>
		</div>

		{#if recordsError}
			<div class="prog-sub c-red" style="margin-top:6px">{recordsError}</div>
		{:else if totalMatched !== null}
			<div class="prog-sub" style="margin-top:6px">{totalMatched} match{totalMatched === 1 ? '' : 'es'} · {selected.size} selected</div>
			<div class="cards">
				{#each records as r (r.id)}
					<button class="ctx-card" class:sel={selected.has(r.id)} type="button" onclick={() => toggle(r.id)}>
						<div class="cc-head"><span class="cat">{r.category}</span><span class="conf">{r.confidence.toFixed(2)}</span></div>
						<div class="intent">{r.intent}</div>
						{#if r.targetPatterns.length}<div class="targets">{r.targetPatterns.slice(0, 3).join(' · ')}</div>{/if}
					</button>
				{:else}
					<div class="prog-sub">No records for this filter.</div>
				{/each}
			</div>
			<button class="btn-primary" type="button" style="width:100%;margin-top:8px" disabled={selected.size === 0} onclick={attach}>Attach Context to Prompt ({selected.size})</button>
		{:else}
			<div class="prog-sub" style="margin-top:6px">Filter and click Find to retrieve safe context records.</div>
		{/if}
	{/if}
</div>
