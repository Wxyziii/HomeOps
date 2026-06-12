<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		getMinecraftPlayers,
		minecraftPlayerAction,
		type MinecraftPlayer,
		type MinecraftPlayersResponse,
		type PlayerAction
	} from '$lib/api/minecraft';

	let data = $state<MinecraftPlayersResponse | null>(null);
	let pageError = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let busyPlayer = $state<string | null>(null);
	let addName = $state('');
	let addBusy = $state(false);

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

	const needsConfirm: PlayerAction[] = ['ban', 'kick'];
	const actionLabels: Record<PlayerAction, string> = {
		op: 'Make operator',
		deop: 'Remove operator',
		kick: 'Kick',
		ban: 'Ban',
		pardon: 'Unban',
		whitelist_add: 'Whitelist',
		whitelist_remove: 'Un-whitelist'
	};

	async function run(action: PlayerAction, player: string) {
		let reason: string | undefined;
		if (needsConfirm.includes(action)) {
			const input = prompt(`${actionLabels[action]} ${player} — reason (optional):`, '');
			if (input === null) return;
			reason = input.trim() || undefined;
		}
		busyPlayer = player;
		pageError = null;
		actionMessage = null;
		try {
			const result = await minecraftPlayerAction(serverConnection.serverUrl, action, player, reason);
			actionMessage = result.response;
			await load();
		} catch (error) {
			pageError = error instanceof Error ? error.message : `${actionLabels[action]} failed.`;
		} finally {
			busyPlayer = null;
		}
	}

	async function addToWhitelist(event: SubmitEvent) {
		event.preventDefault();
		const name = addName.trim();
		if (!name || addBusy) return;
		addBusy = true;
		pageError = null;
		actionMessage = null;
		try {
			const result = await minecraftPlayerAction(serverConnection.serverUrl, 'whitelist_add', name);
			actionMessage = result.response;
			addName = '';
			await load();
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Whitelist add failed.';
		} finally {
			addBusy = false;
		}
	}

	function rowBusy(player: MinecraftPlayer): boolean {
		return busyPlayer !== null;
	}
</script>

<svelte:head><title>Minecraft players · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Players">
		<div class="actions-bar">
			{#if data?.onlineSource === 'rcon'}
				<span class="meta">{data.onlinePlayers?.length ?? 0} online (live)</span>
			{/if}
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if actionMessage}<div class="notice ok">{actionMessage}</div>{/if}

	{#if data && !data.actionsAvailable}
		<div class="notice">Player actions (op, kick, ban, whitelist) require RCON. Live online status is also unavailable. Configure RCON for the server and the agent to enable management.</div>
	{/if}

	{#if data?.actionsAvailable}
		<Panel title="Add to whitelist" icon="ti-user-plus">
			<form class="add-bar" onsubmit={addToWhitelist}>
				<input type="text" placeholder="Player name" bind:value={addName} maxlength="16" disabled={addBusy} />
				<button type="submit" class="primary-btn" disabled={addBusy || !addName.trim()}>
					<i class="ti ti-user-plus" aria-hidden="true"></i> {addBusy ? 'Adding…' : 'Whitelist player'}
				</button>
			</form>
		</Panel>
	{/if}

	<Panel title="Known players" icon="ti-users">
		<div class="player-row header">
			<span>Player</span><span>Status</span><span>Roles</span><span>Actions</span>
		</div>
		{#each data?.players ?? [] as player (player.name)}
			<div class="player-row" class:banned-row={player.banned}>
				<div class="player-id">
					<strong>{player.name}</strong>
					<span class="mono uuid">{player.uuid ?? ''}</span>
				</div>
				<div class="status-cell">
					{#if data?.onlineSource === 'rcon'}
						<StatusBadge status={isOnline(player.name) ? 'active online' : 'offline'} />
					{:else}
						<span class="muted">unknown</span>
					{/if}
					{#if player.banned}
						<span class="ban-tag" title={player.banReason ?? ''}>banned</span>
					{/if}
				</div>
				<div class="roles-cell">
					{#if player.op}<span class="role op">OP{player.opLevel ? ` ${player.opLevel}` : ''}</span>{/if}
					{#if player.whitelisted}<span class="role wl">whitelisted</span>{/if}
					{#if !player.op && !player.whitelisted}<span class="muted">—</span>{/if}
				</div>
				<div class="row-actions">
					{#if data?.actionsAvailable}
						{#if player.op}
							<SmallButton icon="ti-shield-off" label="Deop" disabled={rowBusy(player)} onclick={() => void run('deop', player.name)} />
						{:else}
							<SmallButton icon="ti-shield" label="Op" disabled={rowBusy(player)} onclick={() => void run('op', player.name)} />
						{/if}
						{#if player.whitelisted}
							<SmallButton icon="ti-user-minus" label="Un-whitelist" disabled={rowBusy(player)} onclick={() => void run('whitelist_remove', player.name)} />
						{:else}
							<SmallButton icon="ti-user-plus" label="Whitelist" disabled={rowBusy(player)} onclick={() => void run('whitelist_add', player.name)} />
						{/if}
						{#if isOnline(player.name)}
							<SmallButton icon="ti-door-exit" label="Kick" disabled={rowBusy(player)} onclick={() => void run('kick', player.name)} />
						{/if}
						{#if player.banned}
							<SmallButton icon="ti-rotate" label="Unban" disabled={rowBusy(player)} onclick={() => void run('pardon', player.name)} />
						{:else}
							<button class="ban-btn" disabled={rowBusy(player)} onclick={() => void run('ban', player.name)}>
								<i class="ti ti-ban" aria-hidden="true"></i> Ban
							</button>
						{/if}
					{:else}
						<span class="muted">RCON required</span>
					{/if}
				</div>
			</div>
		{:else}
			<div class="empty-state">No players found in whitelist, ops, bans, or user cache.</div>
		{/each}
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions-bar { display: flex; align-items: center; gap: 10px; }
	.meta { font-size: 12px; color: var(--color-text-secondary); }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.add-bar { display: flex; gap: 8px; }
	.add-bar input { width: 220px; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.primary-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--accent); color: #e6f1fb; cursor: pointer; white-space: nowrap; }
	.primary-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.player-row { display: grid; grid-template-columns: minmax(0, 1fr) 140px 170px minmax(0, 2fr); align-items: center; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.player-row.header { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-text-tertiary); }
	.player-row.banned-row { opacity: 0.75; }
	.player-id { min-width: 0; }
	.player-id strong { display: block; color: var(--color-text-primary); }
	.player-id .uuid { display: block; font-size: 9px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mono { font-family: var(--font-mono); }
	.status-cell { display: flex; align-items: center; gap: 6px; }
	.ban-tag { font-size: 9px; padding: 1px 6px; border-radius: 8px; background: var(--color-background-danger); color: var(--color-text-danger); }
	.roles-cell { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; }
	.role { font-size: 9px; padding: 1px 6px; border-radius: 8px; }
	.role.op { background: var(--color-background-warning); color: var(--color-text-warning); }
	.role.wl { background: var(--color-background-success); color: var(--color-text-success); }
	.row-actions { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; justify-content: flex-end; }
	.ban-btn { display: inline-flex; align-items: center; gap: 4px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 12px; border: 0.5px solid var(--color-border-danger); border-radius: var(--border-radius-md); background: transparent; color: var(--color-text-danger); cursor: pointer; }
	.ban-btn:hover:not(:disabled) { background: var(--color-background-danger); }
	.ban-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.muted { color: var(--color-text-tertiary); font-size: 11px; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 980px) { .player-row { grid-template-columns: 1fr 1fr; } .player-row.header { display: none; } .row-actions { justify-content: flex-start; } }
</style>
