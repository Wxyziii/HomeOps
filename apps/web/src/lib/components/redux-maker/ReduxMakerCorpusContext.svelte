<script lang="ts">
	import type {
		ReduxCorpusStatus,
		ReduxCorpusDatasetSummary,
		ReduxCorpusLatestReport
	} from '$lib/api/client';

	let {
		status = null,
		dataset = null,
		report = null,
		quarantineTotal = null,
		loading = false,
		error = null
	}: {
		status?: ReduxCorpusStatus | null;
		dataset?: ReduxCorpusDatasetSummary | null;
		report?: ReduxCorpusLatestReport | null;
		quarantineTotal?: number | null;
		loading?: boolean;
		error?: string | null;
	} = $props();

	const stats = $derived([
		{ label: 'Packages scanned', value: dataset?.packagesScanned ?? report?.packagesScanned ?? null },
		{ label: 'Dataset records', value: dataset?.datasetRecords ?? report?.datasetRecords ?? null },
		{ label: 'Feature records', value: dataset?.featureRecords ?? null },
		{ label: 'Target patterns', value: dataset?.targetPatterns ?? null },
		{ label: 'Quarantined', value: quarantineTotal ?? dataset?.packagesQuarantined ?? null }
	]);
</script>

<div class="corpus">
	<div class="card-title">Corpus context <a class="link" href="/redux-corpus">open →</a></div>
	{#if loading}
		<div class="msg">Loading corpus context…</div>
	{:else if error}
		<div class="msg err">Corpus context unavailable: {error}</div>
	{:else if !status}
		<div class="msg">Corpus status unavailable.</div>
	{:else if !report && !dataset}
		<div class="msg">No corpus dataset yet. Build one on the Redux Corpus page.</div>
	{:else}
		<div class="grid">
			{#each stats as stat}
				<div class="stat"><span>{stat.label}</span><strong>{stat.value ?? '—'}</strong></div>
			{/each}
			<div class="stat"><span>Latest scan</span><strong>{report?.finishedAt ?? 'done'}</strong></div>
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
</style>
