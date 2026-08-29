<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import {
		getReduxArchive,
		downloadFileUrl,
		type ReduxArchiveEntry,
		type ReduxArchiveDownload,
		type ReduxArchiveKind
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	// Standalone download library for the scraped reduxes/gunpacks.
	// Unrelated to the Redux Maker and Redux Corpus modules.
	let entries = $state<ReduxArchiveEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let search = $state('');
	let kindFilter = $state<'all' | ReduxArchiveKind>('all');
	let onlyWithVideo = $state(false);
	let onlyDownloadable = $state(false);
	let expanded = $state<Record<string, boolean>>({});

	onMount(() => {
		serverConnection.load();
		void load();
	});

	async function load() {
		loading = true;
		error = null;
		try {
			entries = await getReduxArchive();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load the archive.';
		} finally {
			loading = false;
		}
	}

	const counts = $derived.by(() => {
		const c = { all: entries.length, redux: 0, gunpack: 0, both: 0, unknown: 0 };
		for (const e of entries) c[e.kind] += 1;
		return c;
	});

	const filtered = $derived.by(() => {
		const q = search.trim().toLowerCase();
		return entries.filter((e) => {
			if (kindFilter !== 'all' && e.kind !== kindFilter) return false;
			if (onlyWithVideo && !e.youtubeId) return false;
			if (onlyDownloadable && !e.downloads.some((d) => d.downloadPath)) return false;
			if (q && !(e.name.toLowerCase().includes(q) || (e.author ?? '').toLowerCase().includes(q)))
				return false;
			return true;
		});
	});

	function dlHref(d: ReduxArchiveDownload): string | null {
		if (!d.downloadPath) return null;
		try {
			return downloadFileUrl(serverConnection.serverUrl, d.downloadPath, d.downloadRootId);
		} catch {
			return null;
		}
	}

	// First ready download for the primary button, if any.
	function primaryHref(e: ReduxArchiveEntry): string | null {
		for (const d of e.downloads) {
			const href = dlHref(d);
			if (href) return href;
		}
		return null;
	}

	function thumb(id: string): string {
		return `https://i.ytimg.com/vi/${id}/hqdefault.jpg`;
	}

	const kindTabs: Array<{ id: 'all' | ReduxArchiveKind; label: string }> = [
		{ id: 'all', label: 'All' },
		{ id: 'redux', label: 'Redux' },
		{ id: 'gunpack', label: 'Gunpacks' },
		{ id: 'unknown', label: 'Unsorted' }
	];
</script>

<svelte:head><title>Redux Archive · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Redux Archive" flush>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={load} />
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}

		<section class="toolbar">
			<SearchInput placeholder="Search reduxes by name or author…" bind:value={search} />
			<div class="tabs">
				{#each kindTabs as tab}
					<button
						class="tab {kindFilter === tab.id ? 'active' : ''}"
						onclick={() => (kindFilter = tab.id)}
					>
						{tab.label} <span>{counts[tab.id]}</span>
					</button>
				{/each}
			</div>
			<label class="chk"><input type="checkbox" bind:checked={onlyWithVideo} /> Has preview</label>
			<label class="chk"><input type="checkbox" bind:checked={onlyDownloadable} /> Downloadable</label>
		</section>

		<div class="count-line">
			Showing {filtered.length} of {entries.length} · {entries.filter((e) => e.youtubeId).length} with preview
		</div>

		{#if loading}
			<div class="notice">Loading archive…</div>
		{:else if filtered.length === 0}
			<div class="notice">No reduxes match the current filters.</div>
		{:else}
			<div class="grid">
				{#each filtered as e (e.id)}
					<article class="card">
						<div class="thumb">
							{#if e.youtubeId}
								<a
									href={`https://www.youtube.com/watch?v=${e.youtubeId}`}
									target="_blank"
									rel="noreferrer"
									title="Watch preview on YouTube"
								>
									<img src={thumb(e.youtubeId)} alt={e.name} loading="lazy" />
									<span class="play"><i class="ti ti-brand-youtube" aria-hidden="true"></i></span>
								</a>
							{:else}
								<div class="thumb-empty"><i class="ti ti-photo-off" aria-hidden="true"></i> no preview</div>
							{/if}
							<span class="kind {e.kind}">{e.kind}</span>
							{#if e.count > 1}<span class="files">{e.count} files</span>{/if}
						</div>
						<div class="body">
							<div class="name" title={e.name}>{e.name}</div>
							<div class="meta">
								{#if e.author}<span><i class="ti ti-user" aria-hidden="true"></i> {e.author}</span>{/if}
							</div>
							<div class="actions">
								{#if primaryHref(e)}
									<a class="dl" href={primaryHref(e)} download>
										<i class="ti ti-download" aria-hidden="true"></i> Download
									</a>
								{:else}
									<span class="dl disabled" title="Not yet downloaded to the server">
										<i class="ti ti-clock" aria-hidden="true"></i> Pending
									</span>
								{/if}
								{#if e.count > 1}
									<button
										class="src"
										title="Show all files"
										onclick={() => (expanded[e.id] = !expanded[e.id])}
									>
										<i class="ti {expanded[e.id] ? 'ti-chevron-up' : 'ti-dots'}" aria-hidden="true"></i>
									</button>
								{:else}
									<a class="src" href={e.downloads[0]?.url} target="_blank" rel="noreferrer" title="Original source link">
										<i class="ti ti-external-link" aria-hidden="true"></i>
									</a>
								{/if}
							</div>
							{#if expanded[e.id]}
								<div class="dl-list">
									{#each e.downloads as d, i}
										<div class="dl-row">
											<span class="dl-idx">{i + 1}</span>
											{#if dlHref(d)}
												<a href={dlHref(d)} download><i class="ti ti-download" aria-hidden="true"></i> file {i + 1}</a>
											{:else}
												<span class="pending-row"><i class="ti ti-clock" aria-hidden="true"></i> pending</span>
											{/if}
											<a class="ext" href={d.url} target="_blank" rel="noreferrer" title="Open source link" aria-label="Open source link"><i class="ti ti-external-link" aria-hidden="true"></i></a>
										</div>
									{/each}
								</div>
							{/if}
						</div>
					</article>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }

	.toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }
	.tabs { display: flex; gap: 4px; }
	.tab { display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 12px; font-size: 12px; font-weight: 600; border: 1px solid var(--color-border-secondary); border-radius: 4px; background: var(--bg-surface); color: var(--color-text-secondary); cursor: pointer; }
	.tab.active { background: var(--orange-bg); border-color: var(--orange-border); color: var(--color-text-primary); }
	.tab span { color: var(--color-text-tertiary); font-size: 11px; }
	.chk { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; color: var(--color-text-secondary); cursor: pointer; }
	.count-line { color: var(--color-text-tertiary); font-size: 11px; }

	.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 12px; }
	.card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; display: flex; flex-direction: column; }

	.thumb { position: relative; aspect-ratio: 16 / 9; background: var(--bg-surface-2); }
	.thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
	.thumb a { display: block; height: 100%; }
	.play { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: #fff; font-size: 30px; opacity: 0.85; text-shadow: 0 1px 6px rgba(0,0,0,0.6); }
	.thumb-empty { height: 100%; display: flex; align-items: center; justify-content: center; gap: 6px; color: var(--text-faint); font-size: 11px; }
	.kind { position: absolute; top: 6px; left: 6px; text-transform: uppercase; letter-spacing: 0.04em; font-size: 9px; font-weight: 700; padding: 2px 6px; border-radius: 999px; background: rgba(0,0,0,0.55); color: #fff; }
	.kind.redux { background: rgba(47, 143, 31, 0.85); }
	.kind.gunpack { background: rgba(176, 92, 20, 0.85); }
	.kind.both { background: rgba(90, 90, 160, 0.85); }
	.files { position: absolute; top: 6px; right: 6px; font-size: 9px; font-weight: 700; padding: 2px 6px; border-radius: 999px; background: rgba(0,0,0,0.55); color: #fff; }

	.dl-list { display: flex; flex-direction: column; gap: 4px; margin-top: 6px; border-top: 0.5px solid var(--color-border-tertiary); padding-top: 6px; }
	.dl-row { display: flex; align-items: center; gap: 8px; font-size: 11px; }
	.dl-idx { color: var(--text-faint); width: 16px; }
	.dl-row a { display: inline-flex; align-items: center; gap: 5px; color: var(--accent); text-decoration: none; }
	.dl-row .ext { margin-left: auto; color: var(--color-text-tertiary); }
	.pending-row { color: var(--text-faint); display: inline-flex; align-items: center; gap: 5px; }

	.body { padding: 10px; display: flex; flex-direction: column; gap: 7px; }
	.name { color: var(--color-text-primary); font-size: 13px; font-weight: 600; line-height: 1.3; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; min-height: 34px; }
	.meta { display: flex; flex-wrap: wrap; gap: 10px; color: var(--color-text-tertiary); font-size: 11px; }
	.meta i { font-size: 12px; }
	.actions { display: flex; gap: 6px; margin-top: 2px; }
	.dl { flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 30px; font-size: 12px; font-weight: 600; border-radius: 4px; text-decoration: none; border: 1px solid var(--orange-border); background: var(--orange-bg); color: var(--color-text-primary); }
	.dl.disabled { border-color: var(--color-border-secondary); background: var(--bg-surface); color: var(--text-faint); cursor: not-allowed; }
	.src { display: inline-flex; align-items: center; justify-content: center; width: 34px; height: 30px; border: 1px solid var(--color-border-secondary); border-radius: 4px; color: var(--color-text-secondary); text-decoration: none; }
	.src:hover { color: var(--color-text-primary); border-color: var(--orange-border); }
</style>
