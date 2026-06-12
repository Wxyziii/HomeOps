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
		getMinecraftMods,
		installMinecraftMod,
		setMinecraftModEnabled,
		type MinecraftMod
	} from '$lib/api/minecraft';

	let mods = $state<MinecraftMod[]>([]);
	let installEnabled = $state(false);
	let gameVersion = $state<string | null>(null);
	let pageError = $state<string | null>(null);
	let busyFile = $state<string | null>(null);

	let installInput = $state('');
	let installBusy = $state(false);
	let installError = $state<string | null>(null);
	let installSuccess = $state<string | null>(null);

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
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

	async function toggle(mod: MinecraftMod) {
		busyFile = mod.fileName;
		pageError = null;
		try {
			await setMinecraftModEnabled(serverConnection.serverUrl, mod.fileName, !mod.enabled);
			await load();
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
			await load();
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not remove mod.';
		} finally {
			busyFile = null;
		}
	}

	async function install(event: SubmitEvent) {
		event.preventDefault();
		const value = installInput.trim();
		if (!value || installBusy) return;
		installBusy = true;
		installError = null;
		installSuccess = null;
		try {
			const result = await installMinecraftMod(serverConnection.serverUrl, value);
			installSuccess = `Installed ${result.fileName} (version ${result.version}). Restart the server to load it.`;
			installInput = '';
			await load();
		} catch (error) {
			installError = error instanceof Error ? error.message : 'Install failed.';
		} finally {
			installBusy = false;
		}
	}

	const enabledCount = $derived(mods.filter((mod) => mod.enabled).length);
</script>

<svelte:head><title>Minecraft mods · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Mods">
		<div class="actions">
			<span class="meta">{enabledCount} enabled · {mods.length - enabledCount} disabled{gameVersion ? ` · Minecraft ${gameVersion}` : ''}</span>
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	<Panel title="Install from Modrinth" icon="ti-download">
		{#if installEnabled}
			<form class="install-bar" onsubmit={install}>
				<input
					type="text"
					placeholder="Modrinth project slug, id, or https://modrinth.com/mod/… URL"
					bind:value={installInput}
					disabled={installBusy}
				/>
				<button type="submit" class="install-btn" disabled={installBusy || !installInput.trim()}>
					<i class="ti ti-download" aria-hidden="true"></i> {installBusy ? 'Installing…' : 'Install'}
				</button>
			</form>
			<div class="hint">Downloads the newest Fabric build compatible with Minecraft {gameVersion ?? '(unknown version)'} from the Modrinth CDN into the mods folder. A server restart is required to load new mods.</div>
			{#if installError}<div class="notice error">{installError}</div>{/if}
			{#if installSuccess}<div class="notice ok">{installSuccess}</div>{/if}
		{:else}
			<div class="empty-state">Mod installation is disabled in the server-agent config.</div>
		{/if}
	</Panel>

	<Panel title="Installed mods" icon="ti-puzzle">
		<div class="mod-table">
			{#each mods as mod (mod.fileName)}
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
				<div class="empty-state">No mods found in the mods folder.</div>
			{/each}
		</div>
		<div class="hint">Disabling renames the file to <span class="mono">.jar.disabled</span> inside the mods folder. Changes apply on the next server restart.</div>
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions { display: flex; align-items: center; gap: 10px; }
	.meta { font-size: 12px; color: var(--color-text-secondary); }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); margin-top: 8px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.install-bar { display: flex; gap: 8px; }
	.install-bar input { flex: 1; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.install-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: transparent; color: var(--color-text-primary); cursor: pointer; white-space: nowrap; }
	.install-btn:hover:not(:disabled) { background: var(--bg-surface-2); }
	.install-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.hint { margin-top: 8px; font-size: 11px; color: var(--color-text-tertiary); }
	.mod-table { display: flex; flex-direction: column; }
	.mod-row { display: grid; grid-template-columns: minmax(0, 1fr) 90px 90px 190px; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.mod-name { min-width: 0; }
	.mod-name strong { display: block; color: var(--color-text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mod-name span { display: block; font-size: 10px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mono { font-family: var(--font-mono); }
	.mod-actions { display: flex; align-items: center; gap: 6px; justify-content: flex-end; }
	.icon-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; padding: 4px; border-radius: 4px; }
	.icon-btn:hover:not(:disabled) { background: var(--bg-surface-2); color: var(--color-text-danger); }
	.icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 860px) { .mod-row { grid-template-columns: 1fr; } .mod-actions { justify-content: flex-start; } }
</style>
