<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		createManagedServer,
		formatBytes,
		getManagedServers,
		managedServerAction,
		type ManagedServer,
		type ManagedServersResponse
	} from '$lib/api/minecraft';

	let data = $state<ManagedServersResponse | null>(null);
	let pageError = $state<string | null>(null);
	let actionError = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let busyServer = $state<string | null>(null);
	let interval: ReturnType<typeof setInterval> | null = null;

	let showCreate = $state(false);
	let createBusy = $state(false);
	let createError = $state<string | null>(null);
	let form = $state({
		name: '',
		gameVersion: '26.1.2',
		port: 25566,
		memoryMb: 2048,
		motd: '',
		maxPlayers: 10,
		acceptEula: false
	});

	onMount(async () => {
		serverConnection.load();
		await load();
		interval = setInterval(() => {
			if (document.visibilityState === 'visible' && busyServer === null) void load();
		}, 6000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function load() {
		try {
			data = await getManagedServers(serverConnection.serverUrl);
			pageError = null;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load servers.';
		}
	}

	async function run(server: ManagedServer, action: 'start' | 'stop' | 'restart') {
		busyServer = server.id;
		actionError = null;
		actionMessage = null;
		try {
			const result = await managedServerAction(serverConnection.serverUrl, server.id, action);
			actionMessage = `${server.name}: systemctl ${action} done (state: ${result.state}).`;
			await load();
		} catch (error) {
			actionError = error instanceof Error ? error.message : `Could not ${action} ${server.name}.`;
		} finally {
			busyServer = null;
		}
	}

	async function submitCreate(event: SubmitEvent) {
		event.preventDefault();
		if (createBusy) return;
		createBusy = true;
		createError = null;
		try {
			const response = await createManagedServer(serverConnection.serverUrl, {
				...form,
				name: form.name.trim(),
				motd: form.motd.trim() || form.name.trim()
			});
			actionMessage = `Server creation job ${response.job.id} started — track it in Operations. Start the server here once the job finishes.`;
			showCreate = false;
			form = { name: '', gameVersion: '26.1.2', port: form.port + 1, memoryMb: 2048, motd: '', maxPlayers: 10, acceptEula: false };
			await load();
		} catch (error) {
			createError = error instanceof Error ? error.message : 'Server creation failed.';
		} finally {
			createBusy = false;
		}
	}
</script>

<svelte:head><title>Minecraft servers · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Servers">
		<div class="actions">
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
			{#if data?.instanceSupport}
				<button class="primary-btn" onclick={() => (showCreate = !showCreate)}>
					<i class="ti ti-plus" aria-hidden="true"></i> Add server
				</button>
			{/if}
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if actionError}<div class="notice error">{actionError}</div>{/if}
	{#if actionMessage}<div class="notice ok">{actionMessage}</div>{/if}
	{#if data && !data.instanceSupport && data.instanceSupportReason}
		<div class="notice">{data.instanceSupportReason}</div>
	{/if}

	{#if showCreate}
		<Panel title="Create a new Fabric server" icon="ti-plus">
			<form class="create-form" onsubmit={submitCreate}>
				<div class="form-grid">
					<label>
						<span>Name (id)</span>
						<input type="text" bind:value={form.name} placeholder="creative-2" pattern={'[a-z0-9-]{2,32}'} required />
						<em>lowercase letters, digits, dashes</em>
					</label>
					<label>
						<span>Minecraft version</span>
						<input type="text" bind:value={form.gameVersion} placeholder="26.1.2" required />
						<em>Fabric loader resolved automatically</em>
					</label>
					<label>
						<span>Port</span>
						<input type="number" bind:value={form.port} min="1024" max="65535" required />
						<em>main server uses 25565</em>
					</label>
					<label>
						<span>Memory (MB)</span>
						<input type="number" bind:value={form.memoryMb} min="512" max="16384" step="256" required />
						<em>-Xmx for the JVM</em>
					</label>
					<label>
						<span>MOTD</span>
						<input type="text" bind:value={form.motd} maxlength="60" placeholder="Server name shown in the list" />
						<em>optional</em>
					</label>
					<label>
						<span>Max players</span>
						<input type="number" bind:value={form.maxPlayers} min="1" max="200" required />
						<em>server.properties max-players</em>
					</label>
				</div>
				<label class="eula">
					<input type="checkbox" bind:checked={form.acceptEula} />
					I accept the <a href="https://aka.ms/MinecraftEULA" target="_blank" rel="noreferrer">Minecraft EULA</a> for this server (writes eula=true).
				</label>
				{#if createError}<div class="notice error">{createError}</div>{/if}
				<div class="form-actions">
					<button type="submit" class="primary-btn" disabled={createBusy || !form.acceptEula || !form.name.trim()}>
						<i class="ti ti-server-2" aria-hidden="true"></i> {createBusy ? 'Creating…' : 'Create server'}
					</button>
					<SmallButton icon="ti-x" label="Cancel" onclick={() => { showCreate = false; }} />
				</div>
			</form>
		</Panel>
	{/if}

	{#each data?.servers ?? [] as server (server.id)}
		<div class="server-card">
			<div class="server-head">
				<div class="server-icon" class:running={server.running}><i class="ti ti-cube" aria-hidden="true"></i></div>
				<div class="server-meta">
					<strong>{server.name}</strong>
					<span>
						{server.gameVersion ? `Minecraft ${server.gameVersion}` : 'Version unknown'} · {server.loader}
						· port {server.port ?? '?'}
						{#if server.memoryMb}· {formatBytes(server.memoryMb * 1024 * 1024)} max heap{/if}
						{#if server.kind === 'main'}· primary{/if}
					</span>
					<span class="mono path">{server.path}</span>
				</div>
				<StatusBadge status={server.running ? 'active online' : server.state === 'failed' ? 'failed' : 'offline stopped'} />
			</div>
			<div class="server-actions">
				<SmallButton icon="ti-player-play" label="Start" disabled={busyServer !== null || server.running} onclick={() => void run(server, 'start')} />
				<SmallButton icon="ti-refresh" label="Restart" disabled={busyServer !== null || !server.running} onclick={() => void run(server, 'restart')} />
				<SmallButton icon="ti-player-stop" label="Stop" disabled={busyServer !== null || !server.running} onclick={() => void run(server, 'stop')} />
				<a class="link" href={`/minecraft/console?server=${encodeURIComponent(server.id)}`}>Console</a>
				{#if server.kind === 'main'}
					<a class="link" href="/minecraft/backups">Backups</a>
					<a class="link" href="/minecraft/mods">Mods</a>
				{/if}
				{#if busyServer === server.id}<span class="meta">working…</span>{/if}
			</div>
		</div>
	{:else}
		{#if !pageError}<div class="empty-state">Loading servers…</div>{/if}
	{/each}

	{#if data}
		<div class="hint">New servers are provisioned as systemd instances (minecraft-instance@&lt;name&gt;.service) under the configured instances root. Files, config, mods, and backups pages currently manage the primary server.</div>
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 14px; }
	.actions { display: flex; align-items: center; gap: 8px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); color: var(--color-text-secondary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.primary-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; font-size: 12px; font-weight: 600; padding: 5px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--accent); color: #e6f1fb; cursor: pointer; white-space: nowrap; }
	.primary-btn:disabled { cursor: not-allowed; opacity: 0.48; }
	.create-form { display: flex; flex-direction: column; gap: 12px; }
	.form-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
	.form-grid label { display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
	.form-grid label span { color: var(--color-text-secondary); font-weight: 600; }
	.form-grid label em { color: var(--color-text-tertiary); font-style: normal; font-size: 10px; }
	.form-grid input { background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); padding: 7px 10px; color: var(--color-text-primary); font-size: 12px; }
	.eula { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--color-text-secondary); }
	.eula a { color: var(--accent); }
	.form-actions { display: flex; align-items: center; gap: 8px; }
	.server-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
	.server-head { display: flex; align-items: center; gap: 12px; }
	.server-icon { width: 36px; height: 36px; background: var(--bg-surface-2); border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; color: var(--color-text-tertiary); font-size: 18px; }
	.server-icon.running { background: var(--accent); color: #e6f1fb; }
	.server-meta { flex: 1; min-width: 0; }
	.server-meta strong { display: block; color: var(--color-text-primary); font-size: 14px; }
	.server-meta span { color: var(--color-text-secondary); font-size: 11px; }
	.server-meta .path { display: block; font-size: 10px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.mono { font-family: var(--font-mono); }
	.server-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
	.link { font-size: 12px; color: var(--accent); }
	.meta { font-size: 11px; color: var(--color-text-tertiary); }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	.hint { font-size: 11px; color: var(--color-text-tertiary); }
	@media (max-width: 860px) { .form-grid { grid-template-columns: 1fr; } }
</style>
