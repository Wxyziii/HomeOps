<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import { getMinecraftPlayers, type MinecraftPlayersResponse } from '$lib/api/minecraft';

	let data = $state<MinecraftPlayersResponse | null>(null);
	let pageError = $state<string | null>(null);

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
		pageError = null;
		try {
			data = await getMinecraftPlayers(serverConnection.serverUrl);
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load players.';
		}
	}

	function isOnline(name: string): boolean {
		return data?.onlinePlayers?.includes(name) ?? false;
	}
</script>

<svelte:head><title>Minecraft players · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Players">
		<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	{#if data && data.onlineSource !== 'rcon'}
		<div class="notice">Live online status is unavailable because RCON is not configured. The list below is built from whitelist.json, ops.json, and usercache.json.</div>
	{/if}

	<Panel title="Known players" icon="ti-users">
		<div class="player-row header">
			<span>Player</span><span>Online</span><span>Whitelist</span><span>Operator</span><span>UUID</span>
		</div>
		{#each data?.players ?? [] as player (player.name)}
			<div class="player-row">
				<strong>{player.name}</strong>
				<span>
					{#if data?.onlineSource === 'rcon'}
						<StatusBadge status={isOnline(player.name) ? 'active online' : 'offline'} />
					{:else}
						<span class="muted">Unavailable</span>
					{/if}
				</span>
				<span>{player.whitelisted ? 'Yes' : '—'}</span>
				<span>{player.op ? `Yes${player.opLevel ? ` (level ${player.opLevel})` : ''}` : '—'}</span>
				<span class="mono uuid">{player.uuid ?? '—'}</span>
			</div>
		{:else}
			<div class="empty-state">No players found in whitelist, ops, or user cache.</div>
		{/each}
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.player-row { display: grid; grid-template-columns: minmax(0, 1fr) 100px 80px 120px 280px; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.player-row.header { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-text-tertiary); }
	.player-row strong { color: var(--color-text-primary); }
	.mono { font-family: var(--font-mono); font-size: 11px; }
	.uuid { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.muted { color: var(--color-text-tertiary); font-size: 11px; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 860px) { .player-row { grid-template-columns: 1fr 1fr; } .player-row.header { display: none; } }
</style>
