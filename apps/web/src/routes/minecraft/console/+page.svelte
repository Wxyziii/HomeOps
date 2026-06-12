<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		getMinecraftConsole,
		getMinecraftStatus,
		sendMinecraftCommand
	} from '$lib/api/minecraft';

	let lines = $state<string[]>([]);
	let pageError = $state<string | null>(null);
	let rconConfigured = $state(false);
	let moduleEnabled = $state(true);
	let command = $state('');
	let commandBusy = $state(false);
	let commandError = $state<string | null>(null);
	let commandResponse = $state<string | null>(null);
	let autoRefresh = $state(true);
	let logBox = $state<HTMLElement | null>(null);
	let interval: ReturnType<typeof setInterval> | null = null;

	onMount(async () => {
		serverConnection.load();
		try {
			const status = await getMinecraftStatus(serverConnection.serverUrl);
			moduleEnabled = status.enabled;
			rconConfigured = status.rconConfigured;
		} catch {
			// status failure surfaces through refresh below
		}
		await refresh();
		interval = setInterval(() => {
			if (autoRefresh && document.visibilityState === 'visible') void refresh();
		}, 4000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh() {
		if (!moduleEnabled) return;
		try {
			const response = await getMinecraftConsole(serverConnection.serverUrl, 300);
			const stickToBottom =
				!logBox || logBox.scrollTop + logBox.clientHeight >= logBox.scrollHeight - 40;
			lines = response.lines;
			pageError = null;
			if (stickToBottom) {
				await tick();
				logBox?.scrollTo({ top: logBox.scrollHeight });
			}
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load console output.';
		}
	}

	async function submitCommand(event: SubmitEvent) {
		event.preventDefault();
		const value = command.trim();
		if (!value || commandBusy) return;
		commandBusy = true;
		commandError = null;
		commandResponse = null;
		try {
			const result = await sendMinecraftCommand(serverConnection.serverUrl, value);
			commandResponse = result.response || 'Command sent (no response body).';
			command = '';
			await refresh();
		} catch (error) {
			commandError = error instanceof Error ? error.message : 'Command failed.';
		} finally {
			commandBusy = false;
		}
	}
</script>

<svelte:head><title>Minecraft console · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Console">
		<div class="actions">
			<label class="auto-toggle"><input type="checkbox" bind:checked={autoRefresh} /> Auto-refresh</label>
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void refresh()} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if !moduleEnabled}
		<div class="notice">The Minecraft module is disabled in the server-agent config.</div>
	{:else}
		<div class="console" bind:this={logBox}>
			{#each lines as line}
				<div class="console-line">{line}</div>
			{:else}
				<div class="empty-state">No log output available from journalctl.</div>
			{/each}
		</div>

		<form class="command-bar" onsubmit={submitCommand}>
			<input
				type="text"
				placeholder={rconConfigured ? 'Server command, e.g. list' : 'RCON is not configured — commands are unavailable'}
				bind:value={command}
				disabled={!rconConfigured || commandBusy}
				maxlength="1000"
			/>
			<button type="submit" class="send-btn" disabled={!rconConfigured || commandBusy || !command.trim()}>
				<i class="ti ti-send" aria-hidden="true"></i> {commandBusy ? 'Sending…' : 'Send'}
			</button>
		</form>
		{#if !rconConfigured}
			<div class="notice">Console output is read from journalctl. Sending commands requires RCON: enable <code>enable-rcon</code> in server.properties, set <code>rcon.password</code>, and configure <code>minecraft.rcon_enabled</code> plus the password environment variable for the agent.</div>
		{/if}
		{#if commandResponse}<div class="notice ok">{commandResponse}</div>{/if}
		{#if commandError}<div class="notice error">{commandError}</div>{/if}
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 12px; height: 100%; }
	.actions { display: flex; align-items: center; gap: 10px; }
	.auto-toggle { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-text-secondary); }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.console { flex: 1; min-height: 320px; max-height: 60vh; overflow-y: auto; background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px; font-family: var(--font-mono); font-size: 11px; }
	.console-line { padding: 1px 0; color: var(--color-text-secondary); white-space: pre-wrap; overflow-wrap: anywhere; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	.command-bar { display: flex; gap: 8px; }
	.command-bar input { flex: 1; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; }
	.command-bar input:disabled { opacity: 0.55; cursor: not-allowed; }
	.send-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: transparent; color: var(--color-text-primary); cursor: pointer; white-space: nowrap; }
	.send-btn:hover:not(:disabled) { background: var(--bg-surface-2); }
	.send-btn:disabled { cursor: not-allowed; opacity: 0.48; }
</style>
