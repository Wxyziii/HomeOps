<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import { getMinecraftStatus, type MinecraftStatus } from '$lib/api/minecraft';

	let status = $state<MinecraftStatus | null>(null);
	let pageError = $state<string | null>(null);

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
		pageError = null;
		try {
			status = await getMinecraftStatus(serverConnection.serverUrl);
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load module status.';
		}
	}
</script>

<svelte:head><title>Minecraft settings · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Module settings">
		<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	<Panel title="Minecraft module" icon="ti-settings">
		<div class="setting-row"><span>Module enabled</span><StatusBadge status={status ? (status.enabled ? 'active enabled' : 'offline disabled') : 'offline loading'} /></div>
		<div class="setting-row"><span>systemd service</span><strong class="mono">{status?.serviceName ?? '…'}</strong></div>
		<div class="setting-row"><span>Server root</span><strong class="mono">{status?.serverRoot ?? '…'}</strong></div>
		<div class="setting-row"><span>RCON</span><strong>{status ? (status.rconConfigured ? 'Configured' : 'Not configured') : '…'}</strong></div>
		<div class="hint">
			These values come from the <span class="mono">minecraft</span> section of the server-agent config
			(<span class="mono">homeops_config.json</span>) and are read-only here. Changing them requires editing the
			config on the server and restarting <span class="mono">homeops-agent.service</span>. The agent never exposes
			the RCON password; it is read from an environment variable on the server.
		</div>
	</Panel>

	<Panel title="Safety model" icon="ti-shield-check">
		<ul class="safety-list">
			<li>All file operations are restricted to the configured server root; traversal, absolute, and symlink-escape paths are rejected.</li>
			<li>Service control is limited to the configured systemd unit via a polkit rule — no shell execution, no sudo.</li>
			<li>Console output is read from journalctl; commands are sent only through RCON when configured.</li>
			<li>Deletes move files to a trash folder inside the backup root instead of removing them.</li>
			<li>World restore refuses to run while the server is active and moves the current world aside first.</li>
			<li>Mod installs are limited to the Modrinth CDN with file-name and size validation.</li>
		</ul>
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.setting-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.setting-row span { color: var(--color-text-secondary); }
	.setting-row strong { color: var(--color-text-primary); overflow-wrap: anywhere; text-align: right; }
	.mono { font-family: var(--font-mono); font-size: 11px; }
	.hint { margin-top: 10px; font-size: 11px; color: var(--color-text-tertiary); line-height: 1.5; }
	.safety-list { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--color-text-secondary); }
</style>
