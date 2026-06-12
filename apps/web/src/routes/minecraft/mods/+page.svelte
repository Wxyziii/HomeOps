<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		deleteMinecraftMod,
		formatBytes,
		formatDownloads,
		getMinecraftMods,
		installMinecraftMod,
		searchModrinth,
		setMinecraftModEnabled,
		type MinecraftMod,
		type ModrinthSearchHit
	} from '$lib/api/minecraft';

	let tab = $state<'installed' | 'browse'>('installed');

	let mods = $state<MinecraftMod[]>([]);
	let installEnabled = $state(false);
	let gameVersion = $state<string | null>(null);
	let pageError = $state<string | null>(null);
	let busyFile = $state<string | null>(null);
	let modFilter = $state('');

	let searchInput = $state('');
	let searchBusy = $state(false);
	let searchError = $state<string | null>(null);
	let hits = $state<ModrinthSearchHit[]>([]);
	let totalHits = $state(0);
	let offset = $state(0);
	let installingProject = $state<string | null>(null);
	let installMessage = $state<string | null>(null);
	let searched = $state(false);

	onMount(async () => {
		serverConnection.load();
		await loadInstalled();
	});

	async function loadInstalled() {
		pageError = null;
		try {
			const response = await getMinecraftMods(serverConnection.serverUrl);
			mods = response.mods;
			installEnabled = response.installEnabled;
			gameVersion = response.gameVersion;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load mods.';
		}
	}

	const visibleMods = $derived(
		mods.filter((mod) => !modFilter || mod.fileName.toLowerCase().includes(modFilter.toLowerCase()))
	);
	const enabledCount = $derived(mods.filter((mod) => mod.enabled).length);

	async function toggle(mod: MinecraftMod) {
		busyFile = mod.fileName;
		pageError = null;
		try {
			await setMinecraftModEnabled(serverConnection.serverUrl, mod.fileName, !mod.enabled);
			await loadInstalled();
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not toggle mod.';
		} finally {
			busyFile = null;
		}
	}

	async function remove(mod: MinecraftMod) {
		if (!confirm(`Move ${mod.fileName} to the removed-mods folder?`)) return;
		busyFile = mod.fileName;
		pageError = null;
		try {
			await deleteMinecraftMod(serverConnection.serverUrl, mod.fileName);
			await loadInstalled();
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not remove mod.';
		} finally {
			busyFile = null;
		}
	}

	async function search(newOffset = 0) {
		searchBusy = true;
		searchError = null;
		try {
			const response = await searchModrinth(
				serverConnection.serverUrl,
				searchInput.trim(),
				'mod',
				newOffset,
				gameVersion ?? ''
			);
			hits = response.hits;
			totalHits = response.totalHits;
			offset = response.offset;
			searched = true;
		} catch (error) {
			searchError = error instanceof Error ? error.message : 'Modrinth search failed.';
		} finally {
			searchBusy = false;
		}
	}

	async function installFromSearch(hit: ModrinthSearchHit) {
		installingProject = hit.projectId;
		installMessage = null;
		searchError = null;
		try {
			const result = await installMinecraftMod(serverConnection.serverUrl, hit.slug);
			installMessage = `Installed ${result.fileName} (${result.version}). Restart the server to load it.`;
			await loadInstalled();
		} catch (error) {
			searchError = error instanceof Error ? error.message : 'Install failed.';
		} finally {
			installingProject = null;
		}
	}

	function openBrowse() {
		tab = 'browse';
		if (!searched && !searchBusy) void search(0);
	}

	function submitSearch(event: SubmitEvent) {
		event.preventDefault();
		void search(0);
	}
</script>

<svelte:head><title>Minecraft mods · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Mods">
		<div class="actions">
			<span class="meta">{enabledCount} enabled · {mods.length - enabledCount} disabled{gameVersion ? ` · Minecraft ${gameVersion}` : ''}</span>
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void loadInstalled()} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	<div class="tabs">
		<button class:active={tab === 'installed'} onclick={() => (tab = 'installed')}>
			<i class="ti ti-puzzle" aria-hidden="true"></i> Installed ({mods.length})
		</button>
		<button class:active={tab === 'browse'} onclick={openBrowse}>
			<i class="ti ti-world-search" aria-hidden="true"></i> Browse Modrinth
		</button>
	</div>

	{#if tab === 'installed'}
		<Panel title="Installed mods" icon="ti-puzzle">
			<div class="toolbar">
				<input type="text" placeholder="Filter installed mods…" bind:value={modFilter} />
				<span class="hint">Disable renames to <span class="mono">.jar.disabled</span>; changes apply on restart.</span>
			</div>
			<div class="mod-table">
				{#each visibleMods as mod (mod.fileName)}
					<div class="mod-row">
						<div class="mod-name">
							<strong>{mod.displayName}</strong>
							<span class="mono">{mod.fileName}</span>
						</div>
						<span>{formatBytes(mod.sizeBytes)}</span>
						<StatusBadge status={mod.enabled ? 'active enabled' : 'offline disabled'} />
						<div class="mod-actions">
							<SmallButton
								icon={mod.enabled ? 'ti-toggle-right' : 'ti-toggle-left'}
								label={mod.enabled ? 'Disable' : 'Enable'}
								disabled={busyFile !== null}
								onclick={() => void toggle(mod)}
							/>
							<button class="icon-btn danger" title="Remove" disabled={busyFile !== null} onclick={() => void remove(mod)}><i class="ti ti-trash" aria-hidden="true"></i></button>
						</div>
					</div>
				{:else}
					<div class="empty-state">{modFilter ? 'No mods match the filter.' : 'No mods found in the mods folder.'}</div>
				{/each}
			</div>
		</Panel>
	{:else}
		<Panel title="Browse Modrinth" icon="ti-world-search">
			{#if !installEnabled}
				<div class="notice">Mod installation is disabled in the server-agent config; browsing is read-only.</div>
			{/if}
			<form class="toolbar" onsubmit={submitSearch}>
				<input type="text" placeholder="Search Fabric mods on Modrinth…" bind:value={searchInput} disabled={searchBusy} />
				<button type="submit" class="primary-btn" disabled={searchBusy}>
					<i class="ti ti-search" aria-hidden="true"></i> {searchBusy ? 'Searching…' : 'Search'}
				</button>
				<span class="hint">Filtered to Fabric{gameVersion ? ` · Minecraft ${gameVersion}` : ''}</span>
			</form>
			{#if searchError}<div class="notice error">{searchError}</div>{/if}
			{#if installMessage}<div class="notice ok">{installMessage}</div>{/if}

			{#if searchBusy && hits.length === 0}
				<div class="empty-state">Searching Modrinth…</div>
			{:else}
				<div class="hit-grid">
					{#each hits as hit (hit.projectId)}
						<div class="hit-card">
							<div class="hit-head">
								{#if hit.iconUrl}
									<img src={hit.iconUrl} alt="" loading="lazy" />
								{:else}
									<div class="icon-fallback"><i class="ti ti-puzzle" aria-hidden="true"></i></div>
								{/if}
								<div class="hit-meta">
									<strong>{hit.title}</strong>
									<span>by {hit.author} · <i class="ti ti-download" aria-hidden="true"></i> {formatDownloads(hit.downloads)}</span>
								</div>
							</div>
							<p class="hit-desc">{hit.description}</p>
							<div class="hit-foot">
								<span class="categories">{hit.categories.slice(0, 3).join(' · ')}</span>
								<SmallButton
									icon="ti-download"
									label={installingProject === hit.projectId ? 'Installing…' : 'Install'}
									disabled={!installEnabled || installingProject !== null}
									onclick={() => void installFromSearch(hit)}
								/>
							</div>
						</div>
					{:else}
						{#if searched}<div class="empty-state">No Fabric mods found for this search{gameVersion ? ` on Minecraft ${gameVersion}` : ''}.</div>{/if}
					{/each}
				</div>
				{#if totalHits > 20}
					<div class="pager">
						<SmallButton icon="ti-chevron-left" label="Prev" disabled={searchBusy || offset === 0} onclick={() => void search(Math.max(0, offset - 20))} />
						<span class="meta">{offset + 1}–{Math.min(offset + 20, totalHits)} of {totalHits}</span>
						<SmallButton icon="ti-chevron-right" label="Next" disabled={searchBusy || offset + 20 >= totalHits} onclick={() => void search(offset + 20)} />
					</div>
				{/if}
			{/if}
		</Panel>
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions { display: flex; align-items: center; gap: 10px; }
	.meta { font-size: 12px; color: var(--color-text-secondary); }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); margin-bottom: 8px; color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.tabs { display: flex; gap: 4px; }
	.tabs button { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; font-size: 12px; font-weight: 600; background: transparent; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); cursor: pointer; }
	.tabs button.active { background: var(--bg-surface); color: var(--color-text-primary); border-color: var(--color-border-secondary); }
	.tabs button.active i { color: var(--accent); }
	.toolbar { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; flex-wrap: wrap; }
	.toolbar input { flex: 1; min-width: 200px; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.primary-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--accent); color: #e6f1fb; cursor: pointer; white-space: nowrap; }
	.primary-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.hint { font-size: 11px; color: var(--color-text-tertiary); }
	.mono { font-family: var(--font-mono); }
	.mod-table { display: flex; flex-direction: column; }
	.mod-row { display: grid; grid-template-columns: minmax(0, 1fr) 90px 90px 190px; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.mod-name { min-width: 0; }
	.mod-name strong { display: block; color: var(--color-text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mod-name span { display: block; font-size: 10px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mod-actions { display: flex; align-items: center; gap: 6px; justify-content: flex-end; }
	.icon-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; padding: 4px; border-radius: 4px; }
	.icon-btn:hover:not(:disabled) { background: var(--bg-surface-2); color: var(--color-text-danger); }
	.icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 14px 0; }
	.hit-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 10px; }
	.hit-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px; display: flex; flex-direction: column; gap: 8px; }
	.hit-head { display: flex; align-items: center; gap: 10px; }
	.hit-head img, .icon-fallback { width: 40px; height: 40px; border-radius: var(--border-radius-md); background: var(--bg-surface-2); object-fit: cover; flex-shrink: 0; }
	.icon-fallback { display: flex; align-items: center; justify-content: center; color: var(--color-text-tertiary); font-size: 20px; }
	.hit-meta { min-width: 0; }
	.hit-meta strong { display: block; color: var(--color-text-primary); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.hit-meta span { font-size: 11px; color: var(--color-text-secondary); }
	.hit-meta span i { font-size: 11px; }
	.hit-desc { margin: 0; font-size: 11px; color: var(--color-text-secondary); line-height: 1.45; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; min-height: 32px; }
	.hit-foot { display: flex; align-items: center; gap: 8px; margin-top: auto; }
	.categories { flex: 1; font-size: 10px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.pager { display: flex; align-items: center; justify-content: center; gap: 12px; margin-top: 12px; }
	@media (max-width: 860px) { .mod-row { grid-template-columns: 1fr; } .mod-actions { justify-content: flex-start; } }
</style>
