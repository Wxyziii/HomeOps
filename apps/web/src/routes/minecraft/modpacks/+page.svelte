<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		formatDownloads,
		getCurseForgeStatus,
		getManagedServers,
		installModpackAsServer,
		searchModrinth,
		type CurseForgeStatus,
		type ModrinthSearchHit
	} from '$lib/api/minecraft';

	let searchInput = $state('');
	let searchBusy = $state(false);
	let searchError = $state<string | null>(null);
	let hits = $state<ModrinthSearchHit[]>([]);
	let totalHits = $state(0);
	let offset = $state(0);
	let searched = $state(false);
	let instanceSupport = $state(true);
	let instanceSupportReason = $state<string | null>(null);
	let curseforge = $state<CurseForgeStatus | null>(null);

	let installTarget = $state<ModrinthSearchHit | null>(null);
	let installBusy = $state(false);
	let installError = $state<string | null>(null);
	let installMessage = $state<string | null>(null);
	let form = $state({
		name: '',
		gameVersion: '',
		port: 25570,
		memoryMb: 4096,
		maxPlayers: 10,
		acceptEula: false
	});

	onMount(async () => {
		serverConnection.load();
		try {
			const [servers, cf] = await Promise.all([
				getManagedServers(serverConnection.serverUrl),
				getCurseForgeStatus(serverConnection.serverUrl)
			]);
			instanceSupport = servers.instanceSupport;
			instanceSupportReason = servers.instanceSupportReason;
			curseforge = cf;
		} catch {
			// errors surface via search
		}
		await search(0);
	});

	async function search(newOffset: number) {
		searchBusy = true;
		searchError = null;
		try {
			const response = await searchModrinth(
				serverConnection.serverUrl,
				searchInput.trim(),
				'modpack',
				newOffset
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

	function startInstall(hit: ModrinthSearchHit) {
		installTarget = hit;
		installError = null;
		installMessage = null;
		form = {
			name: hit.slug.slice(0, 32).replace(/[^a-z0-9-]/g, '-'),
			gameVersion: hit.latestVersion ?? '',
			port: 25570,
			memoryMb: 4096,
			maxPlayers: 10,
			acceptEula: false
		};
	}

	async function submitInstall(event: SubmitEvent) {
		event.preventDefault();
		if (!installTarget || installBusy) return;
		installBusy = true;
		installError = null;
		try {
			const response = await installModpackAsServer(serverConnection.serverUrl, installTarget.slug, {
				name: form.name.trim(),
				gameVersion: form.gameVersion.trim(),
				port: form.port,
				memoryMb: form.memoryMb,
				motd: installTarget.title,
				maxPlayers: form.maxPlayers,
				acceptEula: form.acceptEula
			});
			installMessage = `Modpack install job ${response.job.id} started — track it on the Jobs page, then start the new server from the Servers page.`;
			installTarget = null;
		} catch (error) {
			installError = error instanceof Error ? error.message : 'Modpack install failed.';
		} finally {
			installBusy = false;
		}
	}

	function submitSearch(event: SubmitEvent) {
		event.preventDefault();
		void search(0);
	}
</script>

<svelte:head><title>Minecraft modpacks · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Modpacks" />

	{#if installMessage}<div class="notice ok">{installMessage}</div>{/if}
	{#if !instanceSupport && instanceSupportReason}
		<div class="notice">{instanceSupportReason} Modpack installs create a new server instance, so installs are unavailable until this is resolved.</div>
	{/if}

	{#if installTarget}
		<Panel title={`Install ${installTarget.title} as a new server`} icon="ti-packages">
			<form class="create-form" onsubmit={submitInstall}>
				<div class="form-grid">
					<label>
						<span>Server name (id)</span>
						<input type="text" bind:value={form.name} pattern={'[a-z0-9-]{2,32}'} required />
						<em>lowercase letters, digits, dashes</em>
					</label>
					<label>
						<span>Minecraft version</span>
						<input type="text" bind:value={form.gameVersion} required />
						<em>must match a version the pack supports</em>
					</label>
					<label>
						<span>Port</span>
						<input type="number" bind:value={form.port} min="1024" max="65535" required />
					</label>
					<label>
						<span>Memory (MB)</span>
						<input type="number" bind:value={form.memoryMb} min="512" max="16384" step="256" required />
						<em>modpacks usually need 4096+</em>
					</label>
					<label>
						<span>Max players</span>
						<input type="number" bind:value={form.maxPlayers} min="1" max="200" required />
					</label>
				</div>
				<label class="eula">
					<input type="checkbox" bind:checked={form.acceptEula} />
					I accept the <a href="https://aka.ms/MinecraftEULA" target="_blank" rel="noreferrer">Minecraft EULA</a> for this server.
				</label>
				{#if installError}<div class="notice error">{installError}</div>{/if}
				<div class="form-actions">
					<button type="submit" class="primary-btn" disabled={installBusy || !form.acceptEula || !form.name.trim() || !form.gameVersion.trim()}>
						<i class="ti ti-download" aria-hidden="true"></i> {installBusy ? 'Starting…' : 'Install modpack'}
					</button>
					<SmallButton icon="ti-x" label="Cancel" onclick={() => { installTarget = null; }} />
				</div>
			</form>
		</Panel>
	{/if}

	<Panel title="Browse Modrinth modpacks" icon="ti-packages">
		<form class="toolbar" onsubmit={submitSearch}>
			<input type="text" placeholder="Search Fabric modpacks on Modrinth…" bind:value={searchInput} disabled={searchBusy} />
			<button type="submit" class="primary-btn" disabled={searchBusy}>
				<i class="ti ti-search" aria-hidden="true"></i> {searchBusy ? 'Searching…' : 'Search'}
			</button>
		</form>
		{#if searchError}<div class="notice error">{searchError}</div>{/if}

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
								<div class="icon-fallback"><i class="ti ti-packages" aria-hidden="true"></i></div>
							{/if}
							<div class="hit-meta">
								<strong>{hit.title}</strong>
								<span>by {hit.author} · <i class="ti ti-download" aria-hidden="true"></i> {formatDownloads(hit.downloads)}</span>
							</div>
						</div>
						<p class="hit-desc">{hit.description}</p>
						<div class="hit-foot">
							<span class="categories">{hit.latestVersion ? `up to MC ${hit.latestVersion}` : ''}</span>
							<SmallButton icon="ti-server-2" label="Install as server" disabled={!instanceSupport || installBusy} onclick={() => startInstall(hit)} />
						</div>
					</div>
				{:else}
					{#if searched}<div class="empty-state">No Fabric modpacks found for this search.</div>{/if}
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

	<Panel title="CurseForge" icon="ti-flame">
		{#if curseforge?.configured}
			<div class="notice ok">CurseForge API key is configured.</div>
			<div class="empty-state">CurseForge modpack search is not implemented yet in this phase — Modrinth packs above are fully supported.</div>
		{:else}
			<div class="empty-state">
				{curseforge?.reason ?? 'CurseForge requires an API key (free at console.curseforge.com). Set it as an environment variable for the agent service and restart it.'}
				No fake listings are shown without a working API connection.
			</div>
		{/if}
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); margin-bottom: 8px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.toolbar { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
	.toolbar input { flex: 1; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.primary-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--accent); color: #e6f1fb; cursor: pointer; white-space: nowrap; }
	.primary-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.hit-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 10px; }
	.hit-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px; display: flex; flex-direction: column; gap: 8px; }
	.hit-head { display: flex; align-items: center; gap: 10px; }
	.hit-head img, .icon-fallback { width: 40px; height: 40px; border-radius: var(--border-radius-md); background: var(--bg-surface-2); object-fit: cover; flex-shrink: 0; }
	.icon-fallback { display: flex; align-items: center; justify-content: center; color: var(--color-text-tertiary); font-size: 20px; }
	.hit-meta { min-width: 0; }
	.hit-meta strong { display: block; color: var(--color-text-primary); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.hit-meta span { font-size: 11px; color: var(--color-text-secondary); }
	.hit-meta span i { font-size: 11px; }
	.hit-desc { margin: 0; font-size: 11px; color: var(--color-text-secondary); line-height: 1.45; display: -webkit-box; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; min-height: 32px; }
	.hit-foot { display: flex; align-items: center; gap: 8px; margin-top: auto; }
	.categories { flex: 1; font-size: 10px; color: var(--color-text-tertiary); }
	.pager { display: flex; align-items: center; justify-content: center; gap: 12px; margin-top: 12px; }
	.meta { font-size: 12px; color: var(--color-text-secondary); }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; line-height: 1.5; }
	.create-form { display: flex; flex-direction: column; gap: 12px; }
	.form-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
	.form-grid label { display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
	.form-grid label span { color: var(--color-text-secondary); font-weight: 600; }
	.form-grid label em { color: var(--color-text-tertiary); font-style: normal; font-size: 10px; }
	.form-grid input { background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.eula { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--color-text-secondary); }
	.eula a { color: var(--accent); }
	.form-actions { display: flex; align-items: center; gap: 8px; }
	@media (max-width: 860px) { .form-grid { grid-template-columns: 1fr; } }
</style>
