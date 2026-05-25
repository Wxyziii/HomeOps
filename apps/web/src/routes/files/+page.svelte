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
		downloadFileUrl,
		getSettings,
		listFiles,
		renameFile,
		type FileEntry
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let currentPath = $state('');
	let files = $state<FileEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let allowDelete = $state(false);
	let lastLoadedAt = $state<string | null>(null);

	const breadcrumbParts = $derived([
		'Home',
		...currentPath.split('/').filter(Boolean)
	]);

	onMount(() => {
		serverConnection.load();
		void refresh();
		void loadDeleteCapability();
	});

	async function refresh() {
		loading = true;
		error = null;

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
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not rename item.';
		}
	}

	function downloadEntry(file: FileEntry) {
		window.location.href = downloadFileUrl(serverConnection.serverUrl, file.relativePath);
	}
</script>

<svelte:head><title>Files · HomeOps Panel</title></svelte:head>
<h2 class="sr-only">Files page showing directory listing with file names, sizes, dates, and actions</h2>
<div class="files-page">
	<Topbar title="Files" flush>
		<SearchInput placeholder="Search files…" />
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-upload" label="Upload" />
		<SmallButton icon="ti-folder-plus" label="New folder" onclick={createNewFolder} />
	</Topbar>
	<FileToolbar parts={breadcrumbParts} onnavigate={navigateBreadcrumb} onrefresh={refresh} />
	{#if error}<div class="notice error">{error}</div>{/if}
	<FileTable
		{files}
		ondirectoryopen={openDirectory}
		ondownload={downloadEntry}
		onrename={renameEntry}
		{allowDelete}
	/>
	<StatusBar items={[`${files.length} items`, currentPath || 'workspace root', lastLoadedAt ? `updated ${lastLoadedAt}` : 'not loaded']} />
</div>

<style>
	.files-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.notice { padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
</style>
