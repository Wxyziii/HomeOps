<script lang="ts">
	import { onMount } from 'svelte';
	import IconButton from '$lib/components/IconButton.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import {
		downloadFile,
		extractArchive,
		listFiles,
		type FileEntry
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let archives = $state<FileEntry[]>([]);
	let searchQuery = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	const visibleArchives = $derived(
		searchQuery.trim()
			? archives.filter((archive) => archive.name.toLowerCase().includes(searchQuery.trim().toLowerCase()))
			: archives
	);

	onMount(() => {
		serverConnection.load();
		void refresh();
	});

	async function refresh() {
		loading = true;
		error = null;
		actionMessage = null;
		try {
			const response = await listFiles(serverConnection.serverUrl, '');
			archives = response.items.filter((item) => item.kind === 'file' && item.extension === 'zip');
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load archive files.';
		} finally {
			loading = false;
		}
	}

	async function downloadArchive(archive: FileEntry) {
		error = null;
		try {
			await downloadFile(serverConnection.serverUrl, archive.relativePath);
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not download archive.';
		}
	}

	async function extractArchiveFile(archive: FileEntry) {
		const destination = `extracted/${archive.name.replace(/\.zip$/i, '')}`;
		const chosen = window.prompt('Extract to folder', destination);
		if (chosen === null) return;
		const cleanDestination = chosen.trim();
		if (!cleanDestination) {
			error = 'Extraction destination is required.';
			return;
		}

		loading = true;
		error = null;
		try {
			const response = await extractArchive(
				serverConnection.serverUrl,
				archive.relativePath,
				cleanDestination
			);
			actionMessage = `Extraction job ${response.job.id} queued. Open Jobs to follow progress.`;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start extraction job.';
		} finally {
			loading = false;
		}
	}

	function formatSize(bytes: number) {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}
</script>

<svelte:head><title>Archives · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Archives" flush>
		<div class="actions">
			<SearchInput placeholder="Filter root ZIP files..." bind:value={searchQuery} />
			<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
			<SmallButton icon="ti-plus" label="New archive" title="Archive creation is planned for later" disabled />
		</div>
	</Topbar>
	<div class="content">
		<div class="notice info">
			Archive browser is limited to ZIP files in the workspace root. Nested archive browsing and backup creation are planned; ZIP extraction is fully available here and from Files.
		</div>
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if actionMessage}<div class="notice success"><span>{actionMessage}</span><a href="/jobs">Jobs</a></div>{/if}

		<div class="archive-list">
			{#each visibleArchives as archive}
				<div class="archive-card">
					<div class="archive-icon"><i class="ti ti-file-zip" aria-hidden="true"></i></div>
					<div class="archive-info">
						<div class="archive-name">{archive.name}</div>
						<div class="archive-meta">{archive.relativePath}</div>
					</div>
					<div class="archive-size">{formatSize(archive.sizeBytes)}</div>
					<div class="archive-date">{archive.modifiedAt ? new Date(archive.modifiedAt).toLocaleString() : 'Unknown'}</div>
					<div class="archive-actions">
						<IconButton icon="ti-archive" label="Extract" onclick={() => extractArchiveFile(archive)} />
						<IconButton icon="ti-download" label="Download" onclick={() => downloadArchive(archive)} />
						<IconButton icon="ti-dots-vertical" label="More archive actions planned for later" disabled />
					</div>
				</div>
			{:else}
				<div class="empty">No ZIP archives found in the workspace root.</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.actions { display: flex; gap: 8px; align-items: center; min-width: 520px; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); font-size: 12px; }
	.notice.info { color: var(--color-text-secondary); background: var(--bg-app); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { display: flex; gap: 10px; align-items: center; color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.notice a { color: var(--accent); text-decoration: none; }
	.archive-list { display: flex; flex-direction: column; gap: 8px; }
	.archive-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px 14px; display: flex; align-items: center; gap: 12px; }
	.archive-icon { width: 34px; height: 34px; border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; font-size: 17px; flex-shrink: 0; background: var(--bg-surface-2); color: #d85a30; }
	.archive-info { flex: 1; min-width: 0; }
	.archive-name { font-size: 13px; font-weight: 600; color: var(--color-text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.archive-meta, .archive-date { font-size: 11px; color: var(--color-text-secondary); }
	.archive-size { font-size: 13px; font-weight: 600; color: var(--color-text-primary); min-width: 70px; text-align: right; }
	.archive-date { min-width: 140px; text-align: right; color: var(--color-text-tertiary); }
	.archive-actions { display: flex; gap: 4px; }
	.empty { color: var(--color-text-tertiary); font-size: 12px; padding: 12px; border: 0.5px dashed var(--color-border-tertiary); border-radius: var(--border-radius-md); }
	@media (max-width: 900px) { .actions { min-width: 0; flex-wrap: wrap; } .archive-card { align-items: flex-start; flex-wrap: wrap; } }
</style>
