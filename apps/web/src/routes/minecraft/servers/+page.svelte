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
		getMinecraftStatus,
		minecraftServiceAction,
		type MinecraftStatus
	} from '$lib/api/minecraft';

	let status = $state<MinecraftStatus | null>(null);
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
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load server status.';
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
</script>

<svelte:head><title>Minecraft servers · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Servers" />

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if actionError}<div class="notice error">{actionError}</div>{/if}

	{#if status && !status.enabled}
		<div class="notice">The Minecraft module is disabled in the server-agent config.</div>
	{:else if status}
		<Panel title="Configured servers" icon="ti-server">
			<div class="server-card">
				<div class="server-head">
					<div class="server-icon"><i class="ti ti-cube" aria-hidden="true"></i></div>
					<div class="server-meta">
						<strong>{status.motd ?? status.serviceName}</strong>
						<span>{status.serverVersion ? `Minecraft ${status.serverVersion}` : 'Version unknown'}{status.loader ? ` · ${status.loader}` : ''} · port {status.serverPort ?? '?'}</span>
					</div>
					<StatusBadge status={status.running ? 'active online' : 'offline stopped'} />
				</div>
				<div class="server-stats">
					<div><span>Uptime</span><strong>{formatUptime(status.uptimeSeconds)}</strong></div>
					<div><span>Memory</span><strong>{status.memoryBytes !== null ? formatBytes(status.memoryBytes) : 'Unavailable'}</strong></div>
					<div><span>Players</span><strong>{status.onlinePlayers !== null ? `${status.onlinePlayers} / ${status.maxPlayers ?? '?'}` : 'Unavailable'}</strong></div>
					<div><span>World</span><strong>{status.worldName ?? 'Unknown'}</strong></div>
				</div>
				<div class="server-actions">
					<SmallButton icon="ti-player-play" label="Start" disabled={actionBusy !== null || status.running} onclick={() => runAction('start')} />
					<SmallButton icon="ti-refresh" label="Restart" disabled={actionBusy !== null || !status.running} onclick={() => runAction('restart')} />
					<SmallButton icon="ti-player-stop" label="Stop" disabled={actionBusy !== null || !status.running} onclick={() => runAction('stop')} />
					<a class="link" href="/minecraft/console">Console</a>
					<a class="link" href="/minecraft/backups">Backups</a>
				</div>
				{#if actionBusy}<div class="notice">Running systemctl {actionBusy}…</div>{/if}
			</div>
			<div class="single-note">This deployment manages a single Minecraft server ({status.serviceName}). Multi-server and multi-node management is not configured.</div>
		</Panel>
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.server-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
	.server-head { display: flex; align-items: center; gap: 12px; }
	.server-icon { width: 36px; height: 36px; background: var(--accent); border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; color: #e6f1fb; font-size: 18px; }
	.server-meta { flex: 1; min-width: 0; }
	.server-meta strong { display: block; color: var(--color-text-primary); font-size: 14px; }
	.server-meta span { color: var(--color-text-secondary); font-size: 11px; }
	.server-stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
	.server-stats div { background: var(--bg-surface-2); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 8px; }
	.server-stats span { display: block; color: var(--color-text-secondary); font-size: 10px; }
	.server-stats strong { color: var(--color-text-primary); font-size: 13px; }
	.server-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
	.link { font-size: 12px; color: var(--accent); }
	.single-note { margin-top: 10px; color: var(--color-text-tertiary); font-size: 11px; }
	@media (max-width: 980px) { .server-stats { grid-template-columns: 1fr 1fr; } }
</style>
