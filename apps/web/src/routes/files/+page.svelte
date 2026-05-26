<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import FileToolbar from '$lib/components/FileToolbar.svelte';
	import FileTable from '$lib/components/FileTable.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import {
		createFolder,
		downloadFile,
		extractArchive,
		getSettings,
		listFiles,
		moveFile,
		renameFile,
		uploadFiles,
		type FileEntry
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let currentPath = $state('');
	let files = $state<FileEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let allowDelete = $state(false);
	let lastLoadedAt = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let searchQuery = $state('');
	let uploadInput: HTMLInputElement;

	const breadcrumbParts = $derived([
		'Home',
		...currentPath.split('/').filter(Boolean)
	]);
	const visibleFiles = $derived(
		searchQuery.trim()
			? files.filter((file) => file.name.toLowerCase().includes(searchQuery.trim().toLowerCase()))
			: files
	);

	onMount(() => {
		serverConnection.load();
		void refresh();
		void loadDeleteCapability();
	});

	async function refresh() {
		loading = true;
		error = null;
		actionMessage = null;

		try {
			const response = await listFiles(serverConnection.serverUrl, currentPath);
			files = response.items;
			currentPath = response.path;
			lastLoadedAt = new Date().toLocaleTimeString();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load files.';
		} finally {
			loading = false;
		}
	}

	async function loadDeleteCapability() {
		try {
			const settings = await getSettings(serverConnection.serverUrl);
			allowDelete = settings.config.allow_delete;
		} catch {
			allowDelete = false;
		}
	}

	function openDirectory(file: FileEntry) {
		if (file.kind !== 'directory') return;
		currentPath = file.relativePath;
		void refresh();
	}

	function navigateBreadcrumb(index: number) {
		if (index === 0) {
			currentPath = '';
		} else {
			currentPath = breadcrumbParts.slice(1, index + 1).join('/');
		}
		void refresh();
	}

	async function createNewFolder() {
		const name = window.prompt('Folder name');
		if (!name) return;
		const cleanName = name.trim();
		if (!cleanName) return;

		const path = currentPath ? `${currentPath}/${cleanName}` : cleanName;
		try {
			await createFolder(serverConnection.serverUrl, path);
			actionMessage = `Created ${cleanName}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not create folder.';
		}
	}

	async function renameEntry(file: FileEntry) {
		const name = window.prompt('New name', file.name);
		if (!name) return;
		const cleanName = name.trim();
		if (!cleanName || cleanName === file.name) return;

		const parent = file.relativePath.split('/').slice(0, -1).join('/');
		const target = parent ? `${parent}/${cleanName}` : cleanName;
		try {
			await renameFile(serverConnection.serverUrl, file.relativePath, target);
			actionMessage = `Renamed ${file.name}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not rename item.';
		}
	}

	async function moveEntry(file: FileEntry) {
		const destination = window.prompt('Move to relative destination path', file.relativePath);
		if (destination === null) return;
		const cleanDestination = destination.trim();
		if (!cleanDestination) {
			error = 'Move destination is required.';
			return;
		}
		if (cleanDestination === file.relativePath) return;

		try {
			await moveFile(serverConnection.serverUrl, file.relativePath, cleanDestination);
			actionMessage = `Moved ${file.name}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not move item.';
		}
	}

	async function downloadEntry(file: FileEntry) {
		error = null;
		try {
			await downloadFile(serverConnection.serverUrl, file.relativePath);
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not download file.';
		}
	}

	function chooseUploadFiles() {
		uploadInput.value = '';
		uploadInput.click();
	}

	async function handleUpload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		if (!input.files || input.files.length === 0) return;

		loading = true;
		error = null;
		try {
			await uploadFiles(serverConnection.serverUrl, currentPath, input.files);
			actionMessage = 'Upload complete.';
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not upload files.';
		} finally {
			loading = false;
			input.value = '';
		}
	}

	async function extractEntry(file: FileEntry) {
		if (file.extension !== 'zip') return;
		const destination = defaultExtractionDestination(file);
		const chosen = window.prompt('Extract to folder', destination);
		if (!chosen) return;
		const cleanDestination = chosen.trim();
		if (!cleanDestination) return;

		loading = true;
		error = null;
		actionMessage = null;
		try {
			const response = await extractArchive(
				serverConnection.serverUrl,
				file.relativePath,
				cleanDestination
			);
			actionMessage = `Extraction job ${response.job.id} queued. Open Jobs to follow progress.`;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not start extraction job.';
		} finally {
			loading = false;
		}
	}

	function defaultExtractionDestination(file: FileEntry) {
		const archiveName = file.name.replace(/\.zip$/i, '');
		return `extracted/${archiveName}`;
	}
</script>

<svelte:head><title>Files · HomeOps Panel</title></svelte:head>
<h2 class="sr-only">Files page showing directory listing with file names, sizes, dates, and actions</h2>
<div class="files-page">
	<Topbar title="Files" flush>
		<SearchInput placeholder="Filter current folder…" bind:value={searchQuery} />
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-upload" label={loading ? 'Uploading' : 'Upload'} onclick={chooseUploadFiles} />
		<SmallButton icon="ti-folder-plus" label="New folder" onclick={createNewFolder} />
	</Topbar>
	<input bind:this={uploadInput} class="upload-input" type="file" multiple onchange={handleUpload} />
	<FileToolbar parts={breadcrumbParts} onnavigate={navigateBreadcrumb} onrefresh={refresh} />
	{#if error}<div class="notice error">{error}</div>{/if}
	{#if actionMessage}
		<div class="notice success">
			<span>{actionMessage}</span>
			<a href="/jobs">Jobs</a>
		</div>
	{/if}
	<FileTable
		files={visibleFiles}
		ondirectoryopen={openDirectory}
		ondownload={downloadEntry}
		onextract={extractEntry}
		onrename={renameEntry}
		onmove={moveEntry}
		{allowDelete}
	/>
	<StatusBar items={[`${visibleFiles.length} shown / ${files.length} items`, currentPath || 'workspace root', lastLoadedAt ? `updated ${lastLoadedAt}` : 'not loaded']} />
</div>

<style>
	.files-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.upload-input { display: none; }
	.notice { padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { display: flex; gap: 10px; align-items: center; color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.notice a { color: var(--accent); text-decoration: none; }
	.notice a:hover { text-decoration: underline; }
</style>
