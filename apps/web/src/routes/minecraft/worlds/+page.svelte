<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import { formatBytes, getMinecraftWorlds, type MinecraftWorld } from '$lib/api/minecraft';

	let worlds = $state<MinecraftWorld[]>([]);
	let pageError = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
		loading = true;
		pageError = null;
		try {
			const response = await getMinecraftWorlds(serverConnection.serverUrl);
			worlds = response.worlds;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load worlds.';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head><title>Minecraft worlds · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Worlds">
		<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	<Panel title="World directories" icon="ti-world">
		{#if loading}
			<div class="empty-state">Scanning world directories…</div>
		{:else}
			{#each worlds as world (world.name)}
				<div class="world-row">
					<div class="world-name">
						<i class="ti ti-world" aria-hidden="true"></i>
						<strong>{world.name}</strong>
						{#if world.active}<StatusBadge status="active" />{/if}
					</div>
					<span>{formatBytes(world.sizeBytes)}{world.sizeTruncated ? '+' : ''}</span>
					<span>{world.modifiedAt ? new Date(world.modifiedAt).toLocaleString() : '—'}</span>
				</div>
			{:else}
				<div class="empty-state">No world directories (containing level.dat) were found.</div>
			{/each}
		{/if}
		<div class="hint">Worlds are detected by scanning the server root for directories containing level.dat. Use the Backups page to snapshot or restore the active world.</div>
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.world-row { display: grid; grid-template-columns: minmax(0, 1fr) 110px 180px; align-items: center; gap: 10px; padding: 9px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.world-name { display: flex; align-items: center; gap: 8px; min-width: 0; }
	.world-name strong { color: var(--color-text-primary); }
	.world-name i { color: var(--color-text-secondary); font-size: 15px; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	.hint { margin-top: 8px; font-size: 11px; color: var(--color-text-tertiary); }
</style>
