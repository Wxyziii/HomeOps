<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		getMinecraftServerConfig,
		updateMinecraftServerConfig,
		type MinecraftConfigEntry
	} from '$lib/api/minecraft';

	let entries = $state<MinecraftConfigEntry[]>([]);
	let original = $state<Record<string, string>>({});
	let edited = $state<Record<string, string>>({});
	let pageError = $state<string | null>(null);
	let saveError = $state<string | null>(null);
	let savedAt = $state<string | null>(null);
	let busy = $state(false);
	let filter = $state('');

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
		pageError = null;
		try {
			const response = await getMinecraftServerConfig(serverConnection.serverUrl);
			entries = response.entries;
			original = Object.fromEntries(response.entries.map((entry) => [entry.key, entry.value]));
			edited = { ...original };
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load server.properties.';
		}
	}

	const changedKeys = $derived(
		Object.keys(edited).filter((key) => edited[key] !== original[key])
	);

	const visibleEntries = $derived(
		entries.filter((entry) => !filter || entry.key.includes(filter.toLowerCase()))
	);

	async function save() {
		if (changedKeys.length === 0 || busy) return;
		busy = true;
		saveError = null;
		savedAt = null;
		try {
			const properties = Object.fromEntries(changedKeys.map((key) => [key, edited[key]]));
			await updateMinecraftServerConfig(serverConnection.serverUrl, properties);
			savedAt = new Date().toLocaleTimeString();
			await load();
		} catch (error) {
			saveError = error instanceof Error ? error.message : 'Save failed.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Minecraft config · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Server configuration">
		<div class="actions">
			{#if savedAt}<span class="saved">Saved {savedAt}</span>{/if}
			<SmallButton icon="ti-refresh" label="Reload" onclick={() => void load()} />
			<SmallButton icon="ti-device-floppy" label={busy ? 'Saving…' : `Save ${changedKeys.length ? `(${changedKeys.length})` : ''}`} disabled={busy || changedKeys.length === 0} onclick={save} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if saveError}<div class="notice error">{saveError}</div>{/if}

	<Panel title="server.properties" icon="ti-adjustments">
		<div class="config-toolbar">
			<input type="text" placeholder="Filter properties…" bind:value={filter} />
			<span class="hint">Changes take effect after a server restart. Secret values (passwords) are hidden and cannot be edited here.</span>
		</div>
		<div class="config-list">
			{#each visibleEntries as entry (entry.key)}
				<div class="config-row" class:changed={edited[entry.key] !== original[entry.key]}>
					<label class="key mono" for={`prop-${entry.key}`}>{entry.key}</label>
					{#if entry.redacted}
						<input id={`prop-${entry.key}`} type="text" value="••••••••" disabled />
					{:else}
						<input id={`prop-${entry.key}`} type="text" bind:value={edited[entry.key]} />
					{/if}
				</div>
			{:else}
				<div class="empty-state">No properties match the filter.</div>
			{/each}
		</div>
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions { display: flex; align-items: center; gap: 8px; }
	.saved { font-size: 11px; color: var(--color-text-success); }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.config-toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
	.config-toolbar input { background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 6px 10px; color: var(--color-text-primary); font-size: 12px; width: 220px; }
	.hint { font-size: 11px; color: var(--color-text-tertiary); }
	.config-list { display: flex; flex-direction: column; }
	.config-row { display: grid; grid-template-columns: 240px 1fr; align-items: center; gap: 12px; padding: 5px 0; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.config-row.changed .key { color: var(--color-text-warning); }
	.key { font-size: 12px; color: var(--color-text-secondary); }
	.mono { font-family: var(--font-mono); }
	.config-row input { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 5px 9px; color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; }
	.config-row input:disabled { opacity: 0.5; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 760px) { .config-row { grid-template-columns: 1fr; gap: 4px; } }
</style>
