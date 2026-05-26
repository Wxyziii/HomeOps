<script lang="ts">
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import {
		API_TOKEN_KEY,
		DEFAULT_SERVER_URL,
		clearStoredApiToken,
		createHomeOpsStateBackup,
		downloadFile,
		getSettings,
		getWorkspaceStatus,
		hasStoredApiToken,
		listHomeOpsStateBackups,
		normalizeServerUrl,
		saveStoredApiToken,
		updateSettings,
		type BackendSettingsResponse,
		type HomeOpsStateBackup,
		type WorkspaceStatusResponse
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let serverUrlInput = $state(DEFAULT_SERVER_URL);
	let saveMessage = $state<string | null>(null);
	let validationError = $state<string | null>(null);
	let backendSettings = $state<BackendSettingsResponse | null>(null);
	let workspaceStatus = $state<WorkspaceStatusResponse | null>(null);
	let backendLoading = $state(false);
	let backendError = $state<string | null>(null);
	let appNameInput = $state('');
	let settingsSaveMessage = $state<string | null>(null);
	let apiTokenInput = $state('');
	let apiTokenStored = $state(false);
	let apiTokenMessage = $state<string | null>(null);
	let apiTokenError = $state<string | null>(null);
	let backups = $state<HomeOpsStateBackup[]>([]);
	let backupsLoading = $state(false);
	let backupMessage = $state<string | null>(null);
	let backupError = $state<string | null>(null);
	let backupJobId = $state<string | null>(null);

	onMount(() => {
		serverConnection.load();
		serverUrlInput = serverConnection.serverUrl;
		apiTokenStored = hasStoredApiToken();
		void refreshBackendDetails();
		void refreshBackups();
	});

	function saveServerUrl() {
		try {
			const normalized = serverConnection.saveServerUrl(serverUrlInput);
			serverUrlInput = normalized;
			validationError = null;
			saveMessage = 'Server URL saved.';
		} catch (error) {
			saveMessage = null;
			validationError = error instanceof Error ? error.message : 'Invalid server URL.';
		}
	}

	async function testConnection() {
		saveMessage = null;
		validationError = null;
		try {
			serverUrlInput = serverConnection.saveServerUrl(serverUrlInput);
			await serverConnection.testConnection();
			await refreshBackendDetails();
			if (!backendError) {
				saveMessage = 'Backend reachable and API access verified.';
			}
		} catch (error) {
			validationError = explainConnectionError(error);
		}
	}

	function resetToDefault() {
		serverUrlInput = serverConnection.resetToDefault();
		validationError = null;
		saveMessage = 'Server URL reset to default.';
	}

	async function refreshBackendDetails() {
		backendLoading = true;
		backendError = null;
		settingsSaveMessage = null;

		try {
			const [settings, workspace] = await Promise.all([
				getSettings(serverConnection.serverUrl),
				getWorkspaceStatus(serverConnection.serverUrl)
			]);
			backendSettings = settings;
			workspaceStatus = workspace;
			appNameInput = settings.settings.app_name?.value ?? settings.config.app_name;
		} catch (error) {
			backendError = explainConnectionError(error);
		} finally {
			backendLoading = false;
		}
	}

	async function refreshBackups() {
		backupsLoading = true;
		backupError = null;
		try {
			const response = await listHomeOpsStateBackups(serverConnection.serverUrl);
			backups = response.backups;
		} catch (error) {
			backupError = explainConnectionError(error);
		} finally {
			backupsLoading = false;
		}
	}

	async function startBackup() {
		backupMessage = null;
		backupError = null;
		backupJobId = null;
		try {
			const response = await createHomeOpsStateBackup(serverConnection.serverUrl);
			backupJobId = response.job.id;
			backupMessage = `Backup job started: ${response.job.id}`;
		} catch (error) {
			backupError = explainConnectionError(error);
		}
	}

	async function downloadBackup(backup: HomeOpsStateBackup) {
		backupMessage = null;
		backupError = null;
		try {
			const filename = await downloadFile(serverConnection.serverUrl, backup.relativePath);
			backupMessage = `Downloaded ${filename} to your default downloads folder.`;
		} catch (error) {
			backupError = explainConnectionError(error);
		}
	}

	async function saveBackendSettings() {
		backendError = null;
		settingsSaveMessage = null;

		try {
			await updateSettings(serverConnection.serverUrl, {
				app_name: appNameInput
			});
			settingsSaveMessage = 'Backend settings saved.';
			await refreshBackendDetails();
		} catch (error) {
			backendError = explainConnectionError(error);
		}
	}

	function saveApiToken() {
		apiTokenMessage = null;
		apiTokenError = null;
		try {
			saveStoredApiToken(apiTokenInput);
			apiTokenInput = '';
			apiTokenStored = true;
			apiTokenMessage = 'API token saved locally.';
		} catch (error) {
			apiTokenError = error instanceof Error ? error.message : 'Could not save API token.';
		}
	}

	function clearApiToken() {
		clearStoredApiToken();
		apiTokenInput = '';
		apiTokenStored = false;
		apiTokenError = null;
		apiTokenMessage = 'API token cleared.';
	}

	function explainConnectionError(error: unknown) {
		const message = error instanceof Error ? error.message : 'Connection test failed.';
		if (message.includes('AUTH_REQUIRED')) {
			return 'Server requires an API token. Add it in Settings.';
		}
		if (message.includes('AUTH_INVALID')) {
			return 'Saved API token is invalid.';
		}
		return message;
	}

	function formatBytes(value: number | null) {
		if (value === null) return 'Unknown';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = value;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	$effect(() => {
		try {
			normalizeServerUrl(serverUrlInput);
			if (validationError?.startsWith('Server URL')) {
				validationError = null;
			}
		} catch {
			// Keep validation quiet while typing; Save/Test shows the clear message.
		}
	});
</script>

<svelte:head><title>Settings · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Settings" />
	<Panel title="Server connection" icon="ti-plug-connected">
		<div class="settings-grid">
			<label for="server-url">Server URL</label>
			<div class="input-row">
				<input id="server-url" bind:value={serverUrlInput} placeholder={DEFAULT_SERVER_URL} />
				<SmallButton icon="ti-device-floppy" label="Save" onclick={saveServerUrl} />
				<SmallButton icon="ti-refresh" label="Test Connection" onclick={testConnection} />
				<SmallButton icon="ti-restore" label="Reset to Default" onclick={resetToDefault} />
			</div>
			<p class="hint">Default: {DEFAULT_SERVER_URL}</p>
		</div>
		<div class="status-panel">
			<div class="status-line">
				<span>Status</span>
				<StatusBadge status={serverConnection.connectionStatus} />
			</div>
			<div class="status-line"><span>Current URL</span><strong>{serverConnection.serverUrl}</strong></div>
			<div class="status-line"><span>Last checked</span><strong>{serverConnection.lastCheckedAt ? new Date(serverConnection.lastCheckedAt).toLocaleString() : 'Never'}</strong></div>
			{#if serverConnection.lastHealth}
				<pre>{JSON.stringify(serverConnection.lastHealth, null, 2)}</pre>
			{/if}
			{#if saveMessage}<div class="notice ok">{saveMessage}</div>{/if}
			{#if validationError || serverConnection.lastError}
				<div class="notice error">{validationError ?? serverConnection.lastError}</div>
			{/if}
		</div>
	</Panel>
	<Panel title="API token" icon="ti-key">
		<div class="settings-grid">
			<label for="api-token">API token</label>
			<div class="input-row token-row">
				<input
					id="api-token"
					type="password"
					bind:value={apiTokenInput}
					placeholder={apiTokenStored ? 'Token saved locally' : 'Paste server API token'}
					autocomplete="off"
				/>
				<SmallButton icon="ti-device-floppy" label="Save Token" onclick={saveApiToken} />
				<SmallButton icon="ti-trash" label="Clear Token" onclick={clearApiToken} />
			</div>
			<p class="hint">Required when server-agent is configured with <code>api_token</code>. Stored locally as <code>{API_TOKEN_KEY}</code>; the saved value is not displayed here.</p>
		</div>
		<div class="status-panel">
			<div class="status-line"><span>Local token</span><strong>{apiTokenStored ? 'Stored' : 'Not stored'}</strong></div>
			{#if backendSettings}
				<div class="status-line"><span>Server requires token</span><strong>{backendSettings.config.api_token_configured ? 'Yes' : 'No'}</strong></div>
			{/if}
			{#if apiTokenMessage}<div class="notice ok">{apiTokenMessage}</div>{/if}
			{#if apiTokenError}<div class="notice error">{apiTokenError}</div>{/if}
		</div>
	</Panel>
	<Panel title="Backend foundation" icon="ti-database">
		<div class="panel-actions">
			<SmallButton icon="ti-refresh" label={backendLoading ? 'Loading' : 'Refresh'} onclick={refreshBackendDetails} />
		</div>

		{#if backendError}
			<div class="notice error">{backendError}</div>
		{/if}
		{#if settingsSaveMessage}
			<div class="notice ok">{settingsSaveMessage}</div>
		{/if}

		{#if backendSettings}
			<div class="settings-grid backend-grid">
				<label for="app-name">App name</label>
				<input id="app-name" bind:value={appNameInput} />

				<span class="setting-label">Max parallel jobs</span>
				<div class="readonly-setting">
					<strong>{backendSettings.config.max_parallel_jobs}</strong>
					<span>Config/restart controlled</span>
				</div>

				<span class="setting-label">Archive extraction</span>
				<div class="readonly-setting">
					<strong>{backendSettings.config.allow_archive_extract ? 'Enabled' : 'Disabled'}</strong>
					<span>Config/restart controlled</span>
				</div>

				<span class="setting-label">Direct Tailscale</span>
				<div class="readonly-setting">
					<strong>{backendSettings.config.direct_tailscale_enabled ? 'Enabled' : 'Disabled'}</strong>
					<span>Config/restart controlled</span>
				</div>
			</div>
			<div class="button-row">
				<SmallButton icon="ti-device-floppy" label="Save Backend Settings" onclick={saveBackendSettings} />
			</div>

			<div class="status-panel">
				<div class="status-line"><span>Bind</span><strong>{backendSettings.config.bind_host}:{backendSettings.config.bind_port}</strong></div>
				<div class="status-line"><span>Data</span><strong>{backendSettings.config.data_dir}</strong></div>
				<div class="status-line"><span>Logs</span><strong>{backendSettings.config.logs_dir}</strong></div>
				<div class="status-line"><span>Delete enabled</span><strong>{backendSettings.config.allow_delete ? 'Yes' : 'No'}</strong></div>
				<div class="status-line"><span>API token configured</span><strong>{backendSettings.config.api_token_configured ? 'Yes' : 'No'}</strong></div>
			</div>

			<div class="module-grid">
				{#each backendSettings.modules as module}
					<div class="module-row">
						<strong>{module.name}</strong>
						<span>{module.enabled ? 'enabled' : 'disabled'}</span>
					</div>
				{/each}
			</div>
		{:else if backendLoading}
			<p class="hint">Loading backend settings...</p>
		{:else}
			<p class="hint">Backend settings have not been loaded yet.</p>
		{/if}
	</Panel>

	<Panel title="Workspace safety" icon="ti-shield-check">
		{#if workspaceStatus}
			<div class="status-panel no-top">
				<div class="status-line"><span>Root</span><strong>{workspaceStatus.workspace_root}</strong></div>
				<div class="status-line"><span>Exists</span><strong>{workspaceStatus.exists ? 'Yes' : 'No'}</strong></div>
				<div class="status-line"><span>Writable</span><strong>{workspaceStatus.writable ? 'Yes' : 'No'}</strong></div>
				<div class="status-line"><span>Free space</span><strong>{formatBytes(workspaceStatus.free_bytes)}</strong></div>
				<div class="status-line"><span>Safety</span><strong>{workspaceStatus.safety.ok ? 'OK' : 'Warning'}</strong></div>
				{#if workspaceStatus.writable_reason}
					<div class="notice error">{workspaceStatus.writable_reason}</div>
				{/if}
				<div class:ok={workspaceStatus.safety.ok} class:error={!workspaceStatus.safety.ok} class="notice">
					{workspaceStatus.safety.message}
				</div>
			</div>
		{:else if backendLoading}
			<p class="hint">Loading workspace status...</p>
		{:else}
			<p class="hint">Workspace status has not been loaded yet.</p>
		{/if}
	</Panel>
	<Panel title="HomeOps State Backup" icon="ti-archive">
		<div class="panel-actions">
			<SmallButton icon="ti-refresh" label={backupsLoading ? 'Loading' : 'Refresh'} onclick={refreshBackups} />
		</div>
		<div class="notice warning">This backup contains sensitive config/token data. Keep it private.</div>
		<p class="hint">Includes HomeOps SQLite state, server config, API token file when present, and a redacted manifest. Workspace uploads/extractions are not included.</p>
		<div class="button-row">
			<SmallButton icon="ti-database-export" label="Create Backup" onclick={startBackup} />
			{#if backupJobId}
				<a class="job-link" href="/jobs">Open Jobs</a>
			{/if}
		</div>
		{#if backupMessage}<div class="notice ok">{backupMessage}</div>{/if}
		{#if backupError}<div class="notice error">{backupError}</div>{/if}
		<div class="backup-list">
			{#if backups.length === 0}
				<p class="hint">No HomeOps state backups found yet.</p>
			{:else}
				{#each backups as backup}
					<div class="backup-row">
						<div>
							<strong>{backup.name}</strong>
							<span>{formatBytes(backup.sizeBytes)} · {backup.createdAt ? new Date(backup.createdAt).toLocaleString() : 'Unknown time'}</span>
						</div>
						<SmallButton icon="ti-download" label="Download" onclick={() => downloadBackup(backup)} />
					</div>
				{/each}
			{/if}
		</div>
	</Panel>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.panel-actions { display: flex; justify-content: flex-end; margin-bottom: 12px; }
	.settings-grid { display: flex; flex-direction: column; gap: 7px; }
	.backend-grid { display: grid; grid-template-columns: 150px minmax(240px, 1fr); align-items: center; }
	label, .setting-label { color: var(--color-text-secondary); font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; }
	.input-row { display: grid; grid-template-columns: minmax(260px, 1fr) auto auto auto; gap: 8px; position: relative; }
	.token-row { grid-template-columns: minmax(260px, 1fr) auto auto; }
	input { width: 100%; padding: 8px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-primary); outline: none; }
	input:focus { border-color: var(--accent); }
	.hint { margin: 0; color: var(--color-text-tertiary); font-size: 11px; }
	.readonly-setting { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface); display: flex; justify-content: space-between; gap: 12px; font-size: 12px; }
	.readonly-setting strong { color: var(--color-text-primary); }
	.readonly-setting span { color: var(--color-text-tertiary); }
	.button-row { margin-top: 12px; display: flex; justify-content: flex-end; }
	.status-panel { margin-top: 14px; border-top: 0.5px solid var(--color-border-tertiary); padding-top: 12px; display: flex; flex-direction: column; gap: 8px; }
	.status-panel.no-top { margin-top: 0; border-top: 0; padding-top: 0; }
	.status-line { display: flex; justify-content: space-between; align-items: center; gap: 12px; font-size: 12px; }
	.status-line span { color: var(--color-text-secondary); }
	.status-line strong { color: var(--color-text-primary); font-weight: 600; overflow-wrap: anywhere; }
	.module-grid { margin-top: 12px; display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 8px; }
	.module-row { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface); display: flex; justify-content: space-between; gap: 10px; font-size: 12px; }
	.module-row strong { color: var(--color-text-primary); }
	.module-row span { color: var(--color-text-secondary); }
	pre { margin: 4px 0 0; padding: 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-secondary); font-family: var(--font-mono); font-size: 11px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.ok { color: var(--color-text-success); background: var(--color-background-success); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.warning { color: var(--color-text-warning); background: var(--color-background-warning); }
	.job-link { color: var(--accent); font-size: 12px; text-decoration: none; align-self: center; }
	.backup-list { margin-top: 12px; display: flex; flex-direction: column; gap: 8px; }
	.backup-row { display: flex; justify-content: space-between; gap: 12px; align-items: center; padding: 9px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface); }
	.backup-row div { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
	.backup-row strong { color: var(--color-text-primary); font-size: 12px; overflow-wrap: anywhere; }
	.backup-row span { color: var(--color-text-secondary); font-size: 11px; }
	@media (max-width: 920px) { .input-row, .backend-grid { grid-template-columns: 1fr; } }
</style>
