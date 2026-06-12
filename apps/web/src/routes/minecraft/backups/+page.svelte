<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		createMinecraftBackup,
		formatBytes,
		getMinecraftBackups,
		getMinecraftStatus,
		restoreMinecraftBackup,
		type MinecraftBackup
	} from '$lib/api/minecraft';

	let backups = $state<MinecraftBackup[]>([]);
	let backupRoot = $state('');
	let serverRunning = $state<boolean | null>(null);
	let pageError = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let busy = $state(false);

	onMount(async () => {
		serverConnection.load();
		await load();
	});

	async function load() {
		pageError = null;
		try {
			const [response, status] = await Promise.all([
				getMinecraftBackups(serverConnection.serverUrl),
				getMinecraftStatus(serverConnection.serverUrl)
			]);
			backups = response.backups;
			backupRoot = response.backupRoot;
			serverRunning = status.running;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not load backups.';
		}
	}

	async function createBackup() {
		if (busy) return;
		busy = true;
		pageError = null;
		actionMessage = null;
		try {
			const response = await createMinecraftBackup(serverConnection.serverUrl);
			actionMessage = `Backup job ${response.job.id} started. Track progress on the Jobs page.`;
			await load();
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Backup failed to start.';
		} finally {
			busy = false;
		}
	}

	async function restore(backup: MinecraftBackup) {
		if (busy) return;
		if (serverRunning) {
			pageError = 'Stop the Minecraft server before restoring a backup.';
			return;
		}
		if (
			!confirm(
				`Restore ${backup.name}?\n\nThis replaces the current world. The current world is moved to the restore-trash folder first.`
			)
		)
			return;
		busy = true;
		pageError = null;
		actionMessage = null;
		try {
			const response = await restoreMinecraftBackup(serverConnection.serverUrl, backup.name);
			actionMessage = `Restore job ${response.job.id} started. Track progress on the Jobs page.`;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Restore failed to start.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Minecraft backups · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Backups">
		<div class="actions">
			<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load()} />
			<SmallButton icon="ti-database-export" label={busy ? 'Working…' : 'Create world backup'} disabled={busy} onclick={createBackup} />
		</div>
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}
	{#if actionMessage}<div class="notice ok">{actionMessage}</div>{/if}

	<Panel title="Backup archive" icon="ti-database">
		<div class="meta-line">Backup folder: <span class="mono">{backupRoot || '…'}</span></div>
		<div class="backup-row header">
			<span>Name</span><span>Type</span><span>Size</span><span>Created</span><span></span>
		</div>
		{#each backups as backup (backup.name)}
			<div class="backup-row">
				<strong class="mono">{backup.name}</strong>
				<span>{backup.kind}</span>
				<span>{formatBytes(backup.sizeBytes)}</span>
				<span>{backup.createdAt ? new Date(backup.createdAt).toLocaleString() : '—'}</span>
				<span class="row-actions">
					{#if backup.restorable}
						<SmallButton icon="ti-restore" label="Restore" disabled={busy || serverRunning === true} title={serverRunning ? 'Stop the server first' : 'Restore this world backup'} onclick={() => void restore(backup)} />
					{/if}
				</span>
			</div>
		{:else}
			<div class="empty-state">No backups found in the backup folder.</div>
		{/each}
		<div class="hint">World backups are created as world-&lt;timestamp&gt;.zip via a background job. Restore requires the server to be stopped; the current world is moved aside, never deleted.</div>
	</Panel>
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.actions { display: flex; align-items: center; gap: 8px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.meta-line { font-size: 11px; color: var(--color-text-tertiary); margin-bottom: 8px; }
	.mono { font-family: var(--font-mono); }
	.backup-row { display: grid; grid-template-columns: minmax(0, 1fr) 80px 90px 170px 110px; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.backup-row.header { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-text-tertiary); }
	.backup-row strong { color: var(--color-text-primary); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.row-actions { display: flex; justify-content: flex-end; }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 10px 0; }
	.hint { margin-top: 8px; font-size: 11px; color: var(--color-text-tertiary); }
	@media (max-width: 860px) { .backup-row { grid-template-columns: 1fr 1fr; } .backup-row.header { display: none; } }
</style>
