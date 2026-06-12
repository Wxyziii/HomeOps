<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		formatBytes,
		formatUptime,
		getMinecraftConsole,
		getMinecraftStatus,
		minecraftServiceAction,
		type MinecraftStatus
	} from '$lib/api/minecraft';

	let status = $state<MinecraftStatus | null>(null);
	let logLines = $state<string[]>([]);
	let pageError = $state<string | null>(null);
	let actionError = $state<string | null>(null);
	let actionBusy = $state<string | null>(null);
	let interval: ReturnType<typeof setInterval> | null = null;

	onMount(async () => {
		serverConnection.load();
		await refresh();
		interval = setInterval(() => {
			if (document.visibilityState === 'visible') void refresh();
		}, 5000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh() {
		try {
			status = await getMinecraftStatus(serverConnection.serverUrl);
			pageError = null;
			if (status.enabled) {
				try {
					const console = await getMinecraftConsole(serverConnection.serverUrl, 12);
					logLines = console.lines;
				} catch {
					logLines = [];
				}
			}
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load Minecraft status.';
		}
	}

	async function runAction(action: 'start' | 'stop' | 'restart') {
		actionError = null;
		actionBusy = action;
		try {
			await minecraftServiceAction(serverConnection.serverUrl, action);
			await refresh();
		} catch (error) {
			actionError = error instanceof Error ? error.message : `Could not ${action} the server.`;
		} finally {
			actionBusy = null;
		}
	}

	const badgeStatus = $derived(
		!status ? 'offline loading' : status.running ? 'active online' : status.serviceState === 'failed' ? 'failed' : 'offline stopped'
	);
</script>

<svelte:head><title>Minecraft · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Minecraft overview">
		<div class="actions">
			{#if status?.enabled}
				<SmallButton icon="ti-player-play" label="Start" disabled={actionBusy !== null || status.running} onclick={() => runAction('start')} />
				<SmallButton icon="ti-refresh" label="Restart" disabled={actionBusy !== null || !status.running} onclick={() => runAction('restart')} />
				<SmallButton icon="ti-player-stop" label="Stop" disabled={actionBusy !== null || !status.running} onclick={() => runAction('stop')} />
			{/if}
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if actionError}<div class="notice error">{actionError}</div>{/if}
	{#if actionBusy}<div class="notice">Running systemctl {actionBusy}…</div>{/if}

	{#if status && !status.enabled}
		<div class="notice">The Minecraft module is disabled in the server-agent config. Enable it under <code>minecraft.enabled</code> and restart the agent.</div>
	{:else}
		<div class="metrics">
			<div class="metric">
				<span>Server</span>
				<div class="metric-row">
					<StatusBadge status={badgeStatus} />
					<strong>{status ? status.serviceState : 'Loading'}</strong>
				</div>
				<em>{status?.serviceName ?? ''}</em>
			</div>
			<div class="metric"><span>Uptime</span><strong>{status ? formatUptime(status.uptimeSeconds) : 'Loading'}</strong><em>{status?.serverVersion ? `Minecraft ${status.serverVersion}${status.loader ? ` · ${status.loader}` : ''}` : 'Version unknown'}</em></div>
			<div class="metric"><span>Memory</span><strong>{status ? (status.memoryBytes !== null ? formatBytes(status.memoryBytes) : 'Unavailable') : 'Loading'}</strong><em>systemd MemoryCurrent</em></div>
			<div class="metric"><span>Players</span><strong>{status ? (status.onlinePlayers !== null ? `${status.onlinePlayers} / ${status.maxPlayers ?? '?'}` : 'Unavailable') : 'Loading'}</strong><em>{status?.playerDataSource === 'rcon' ? 'Live via RCON' : 'RCON not configured'}</em></div>
		</div>

		<div class="row2">
			<Panel title="Server details" icon="ti-server">
				<div class="detail-grid">
					<div><span>World</span><strong>{status?.worldName ?? 'Unknown'}</strong></div>
					<div><span>Port</span><strong>{status?.serverPort ?? 'Unknown'}</strong></div>
					<div><span>MOTD</span><strong>{status?.motd ?? 'Unknown'}</strong></div>
					<div><span>Mods</span><strong>{status?.modsTotal ?? 'Unknown'}{#if status?.modsDisabled}<span class="muted"> ({status.modsDisabled} disabled)</span>{/if}</strong></div>
					<div><span>Backups</span><strong>{status?.backupsTotal ?? 'Unknown'}</strong></div>
					<div><span>Server root</span><strong class="mono">{status?.serverRoot ?? ''}</strong></div>
				</div>
			</Panel>

			<Panel title="Recent console" icon="ti-terminal-2" action="See /minecraft/console">
				{#each logLines as line}
					<div class="log-line mono">{line}</div>
				{:else}
					<div class="empty-state">No recent log lines available.</div>
				{/each}
			</Panel>
		</div>
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions { display: flex; align-items: center; gap: 8px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
	.metric { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; }
	.metric span, .metric em { color: var(--color-text-secondary); font-size: 11px; font-style: normal; display: block; }
	.metric strong { display: block; margin-top: 5px; color: var(--color-text-primary); font-size: 18px; }
	.metric-row { display: flex; align-items: center; gap: 8px; margin-top: 5px; }
	.metric-row strong { margin-top: 0; }
	.metric em { margin-top: 4px; }
	.row2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
	.detail-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.detail-grid span { display: block; color: var(--color-text-secondary); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
	.detail-grid strong { color: var(--color-text-primary); font-size: 13px; font-weight: 600; overflow-wrap: anywhere; }
	.muted { color: var(--color-text-tertiary); font-weight: 400; }
	.mono { font-family: var(--font-mono); font-size: 11px; }
	.log-line { padding: 4px 0; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	@media (max-width: 980px) { .metrics, .row2, .detail-grid { grid-template-columns: 1fr; } }
</style>
