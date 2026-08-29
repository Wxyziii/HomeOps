<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import FileToolbar from '$lib/components/FileToolbar.svelte';
	import FileTable from '$lib/components/FileTable.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import DownloadProgress from '$lib/components/DownloadProgress.svelte';
	import {
		createFolder,
		deleteFile,
		downloadFile,
		extractArchive,
		getSettings,
		getWorkspaceStatus,
		listFiles,
		moveFile,
		renameFile,
		uploadFilesWithProgress,
		getServerStoragePool,
		resolvePlacement,
		type FileEntry,
		type StorageRootStatus,
		type SmartStoragePool
	} from '$lib/api/client';
	import IconButton from '$lib/components/IconButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import { ALL_STORAGE_ROOT_ID, storageRoots } from '$lib/stores/storageRoots.svelte';
	import { uploadState } from '$lib/stores/uploads.svelte';

	type SortKey = 'type' | 'name' | 'size' | 'modified';
	type SortDirection = 'asc' | 'desc';
	type ViewMode = 'list' | 'grid';
	type FileFilter = 'all' | 'archives' | 'folders' | 'large' | 'recent';

	let currentPath = $state('');
	let files = $state<FileEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let allowDelete = $state(false);
	let lastLoadedAt = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let searchQuery = $state('');
	let storageRootOptions = $state<StorageRootStatus[]>([]);
	let viewMode = $state<ViewMode>('list');
	let sortKey = $state<SortKey>('type');
	let sortDirection = $state<SortDirection>('asc');
	let fileFilter = $state<FileFilter>('all');
	let uploadInput: HTMLInputElement;
	let interval: ReturnType<typeof setInterval> | null = null;

	let smartPool = $state<SmartStoragePool | null>(null);
	const isAllStorage = $derived(storageRoots.selectedRootId === ALL_STORAGE_ROOT_ID);
	const activeRootId = $derived(storageRoots.selectedRootId);

	const breadcrumbParts = $derived([
		'Home',
		...currentPath.split('/').filter(Boolean)
	]);
	const visibleFiles = $derived(
		sortFiles(
			filterFiles(
				searchQuery.trim()
					? files.filter((file) =>
							[file.name, file.relativePath, file.rootLabel, file.rootId]
								.some((value) => value.toLowerCase().includes(searchQuery.trim().toLowerCase()))
						)
					: files
			)
		)
	);
	const selectedRoot = $derived(storageRootOptions.find((root) => root.id === storageRoots.selectedRootId) ?? storageRootOptions[0] ?? null);

	onMount(() => {
		serverConnection.load();
		storageRoots.load();
		loadViewPreferences();
		loadUrlFilter();
		loadPendingProjectTarget();
		void refresh();
		void loadDeleteCapability();
		void loadStorageRoots();
		void loadStoragePool();
		interval = setInterval(() => {
			if (document.visibilityState === 'visible') void refresh(false);
		}, 4000);
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});

	async function refresh(showLoading = true) {
		if (showLoading) loading = true;
		error = null;

		try {
			const response = await listFiles(serverConnection.serverUrl, currentPath, activeRootId);
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

	async function loadStorageRoots() {
		try {
			const workspace = await getWorkspaceStatus(serverConnection.serverUrl);
			storageRootOptions = workspace.storage_roots;
			if (
				storageRoots.selectedRootId !== ALL_STORAGE_ROOT_ID &&
				!storageRootOptions.some((root) => root.id === storageRoots.selectedRootId)
			) {
				storageRoots.select(ALL_STORAGE_ROOT_ID);
			}
		} catch {
			storageRootOptions = [];
		}
	}

	async function loadStoragePool() {
		try {
			const response = await getServerStoragePool(serverConnection.serverUrl);
			smartPool = response.pool;
		} catch {
			smartPool = null;
		}
	}

	function selectRoot(event: Event) {
		const id = (event.currentTarget as HTMLSelectElement).value;
		storageRoots.select(id);
		currentPath = '';
		void refresh();
	}

	function openDirectory(file: FileEntry) {
		if (file.kind !== 'directory') return;
		if (uploadState.isUploadingPath(file.relativePath)) return;
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
			const rootId = isAllStorage ? await resolveFolderRoot(path) : activeRootId;
			await createFolder(serverConnection.serverUrl, path, rootId);
			actionMessage = `Created ${cleanName}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not create folder.';
		}
	}

	async function renameEntry(file: FileEntry) {
		if (guardUploadingPath(file)) return;
		const rootId = concreteRootId(file);
		if (!rootId) return;
		const name = window.prompt('New name', file.name);
		if (!name) return;
		const cleanName = name.trim();
		if (!cleanName || cleanName === file.name) return;

		const parent = file.relativePath.split('/').slice(0, -1).join('/');
		const target = parent ? `${parent}/${cleanName}` : cleanName;
		try {
			await renameFile(serverConnection.serverUrl, file.relativePath, target, rootId);
			actionMessage = `Renamed ${file.name}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not rename item.';
		}
	}

	async function moveEntry(file: FileEntry) {
		if (guardUploadingPath(file)) return;
		const rootId = concreteRootId(file);
		if (!rootId) return;
		const destination = window.prompt('Move to relative destination path or existing folder', file.relativePath);
		if (destination === null) return;
		const cleanDestination = destination.trim();
		if (!cleanDestination) {
			error = 'Move destination is required.';
			return;
		}
		if (cleanDestination === file.relativePath) return;
		const previewDestination = movePreview(file, cleanDestination);
		const confirmed = window.confirm(`Moving: ${file.relativePath}\nTo: ${previewDestination}`);
		if (!confirmed) return;

		try {
			await moveFile(serverConnection.serverUrl, file.relativePath, cleanDestination, rootId);
			actionMessage = `Moved ${file.name} to ${previewDestination}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not move item.';
		}
	}

	async function downloadEntry(file: FileEntry) {
		if (guardUploadingPath(file)) return;
		const rootId = concreteRootId(file);
		if (!rootId) return;
		error = null;
		try {
			const filename = await downloadFile(serverConnection.serverUrl, file.relativePath, rootId);
			actionMessage = `Downloaded ${filename} to your default downloads folder.`;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not download file.';
		}
	}

	function chooseUploadFiles() {
		error = null;
		uploadInput.value = '';
		uploadInput.click();
	}

	async function handleUpload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		if (!input.files || input.files.length === 0) return;
		const filesToUpload = Array.from(input.files);
		const blocked = filesToUpload.find((file) =>
			uploadState.isUploadingPath(currentPath ? `${currentPath}/${file.name}` : file.name)
		);
		if (blocked) {
			error = `${blocked.name} is already uploading to this folder.`;
			input.value = '';
			return;
		}

		error = null;
		actionMessage = null;
		const uploadItems = uploadState.start(filesToUpload, currentPath);
		let completed = 0;
		let failed = 0;
		try {
			for (let index = 0; index < filesToUpload.length; index += 1) {
				const file = filesToUpload[index];
				const upload = uploadItems[index];
				try {
					let targetRootId = activeRootId;
					if (isAllStorage) {
						const relativePath = currentPath ? `${currentPath}/${file.name}` : file.name;
						const extension = file.name.includes('.') ? file.name.split('.').pop() : undefined;
						const { decision } = await resolvePlacement(serverConnection.serverUrl, {
							intent: 'upload',
							relativePath,
							fileName: file.name,
							sizeBytes: file.size,
							extension
						});
						if (!decision.allowed || !decision.selectedRootId) {
							throw new Error(`Placement blocked: ${decision.reason}`);
						}
						targetRootId = decision.selectedRootId;
						uploadState.setStatus([upload.id], 'uploading', `Routed to '${targetRootId}' — ${decision.reason}`);
					}
					await uploadFilesWithProgress(
						serverConnection.serverUrl,
						currentPath,
						[file],
						targetRootId,
						(uploaded, total) => {
							uploadState.updateProgress([upload.id], uploaded, total);
							if (total > 0 && uploaded >= total) {
								uploadState.setStatus([upload.id], 'finalizing');
							}
						}
					);
					uploadState.setStatus([upload.id], 'completed');
					completed += 1;
				} catch (caught) {
					const message = caught instanceof Error ? caught.message : `Could not upload ${file.name}.`;
					uploadState.setStatus([upload.id], 'failed', message);
					failed += 1;
				}
			}
			if (failed > 0) {
				error = `${failed} upload${failed === 1 ? '' : 's'} failed. See upload panel for details.`;
			}
			if (completed > 0) {
				actionMessage = `Uploaded ${completed} file${completed === 1 ? '' : 's'}.`;
			}
			await refresh(false);
		} catch (caught) {
			const message = caught instanceof Error ? caught.message : 'Could not upload files.';
			error = message;
			await refresh(false);
		} finally {
			input.value = '';
		}
	}

	async function extractEntry(file: FileEntry) {
		if (guardUploadingPath(file)) return;
		if (file.extension !== 'zip') return;
		const rootId = concreteRootId(file);
		if (!rootId) return;
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
				cleanDestination,
				rootId
			);
			actionMessage = `Extraction job ${response.job.id} queued. Open Operations to follow progress.`;
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

	function movePreview(file: FileEntry, destination: string) {
		const matchingDirectory = files.find(
			(item) =>
				item.kind === 'directory' &&
				(item.relativePath === destination ||
					(!destination.includes('/') && item.name === destination && item.relativePath === (currentPath ? `${currentPath}/${destination}` : destination)))
		);
		if (matchingDirectory) return `${matchingDirectory.relativePath}/${file.name}`;
		return destination;
	}

	async function deleteEntry(file: FileEntry) {
		if (guardUploadingPath(file)) return;
		const rootId = concreteRootId(file);
		if (!rootId) return;
		if (!allowDelete) {
			error = 'Delete is disabled by server config.';
			return;
		}
		const folderWarning = file.kind === 'directory' ? '\nFolder contents will be moved to trash too.' : '';
		const confirmed = window.confirm(`Delete: ${file.relativePath}${folderWarning}\n\nThis moves the item into the HomeOps trash for this storage root.`);
		if (!confirmed) return;
		error = null;
		try {
			const response = await deleteFile(serverConnection.serverUrl, file.relativePath, rootId);
			actionMessage = `Moved ${file.name} to trash: ${response.trashedPath}.`;
			await refresh(false);
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not delete item.';
		}
	}

	function sortFiles(items: FileEntry[]) {
		return [...items].sort((a, b) => {
			const direction = sortDirection === 'asc' ? 1 : -1;
			if (sortKey === 'type') {
				const rank = (item: FileEntry) => item.kind === 'directory' ? 0 : 1;
				return (rank(a) - rank(b) || a.name.localeCompare(b.name)) * direction;
			}
			if (sortKey === 'size') return ((a.sizeBytes ?? 0) - (b.sizeBytes ?? 0) || a.name.localeCompare(b.name)) * direction;
			if (sortKey === 'modified') return ((Date.parse(a.modifiedAt ?? '') || 0) - (Date.parse(b.modifiedAt ?? '') || 0) || a.name.localeCompare(b.name)) * direction;
			return a.name.localeCompare(b.name) * direction;
		});
	}

	function filterFiles(items: FileEntry[]) {
		if (fileFilter === 'archives') return items.filter((file) => file.kind === 'file' && isArchive(file));
		if (fileFilter === 'folders') return items.filter((file) => file.kind === 'directory');
		if (fileFilter === 'large') return items.filter((file) => file.kind === 'file' && file.sizeBytes >= 1024 * 1024 * 1024);
		if (fileFilter === 'recent') {
			const cutoff = Date.now() - 7 * 24 * 60 * 60 * 1000;
			return items.filter((file) => (Date.parse(file.modifiedAt ?? '') || 0) >= cutoff);
		}
		return items;
	}

	function isArchive(file: FileEntry) {
		return ['zip', '7z', 'rar', 'oiv', 'rpf'].includes(file.extension ?? '');
	}

	function setFileFilter(filter: FileFilter) {
		fileFilter = filter;
		const url = new URL(window.location.href);
		if (filter === 'all') url.searchParams.delete('filter');
		else url.searchParams.set('filter', filter);
		window.history.replaceState({}, '', url);
	}

	function loadUrlFilter() {
		const filter = new URLSearchParams(window.location.search).get('filter');
		if (filter === 'archives' || filter === 'folders' || filter === 'large' || filter === 'recent') {
			fileFilter = filter;
		}
	}

	async function resolveFolderRoot(path: string) {
		const { decision } = await resolvePlacement(serverConnection.serverUrl, {
			intent: 'generic',
			relativePath: path
		});
		if (!decision.allowed || !decision.selectedRootId) {
			throw new Error(`Placement blocked: ${decision.reason}`);
		}
		return decision.selectedRootId;
	}

	function concreteRootId(file: FileEntry) {
		if (file.rootId && file.rootId !== ALL_STORAGE_ROOT_ID) return file.rootId;
		if (file.sourceRootIds.length === 1) return file.sourceRootIds[0];
		error = `${file.name} is merged across storage roots. Open a specific root before renaming, moving, deleting, or downloading it.`;
		return '';
	}

	async function copyDetails(file: FileEntry) {
		const details = [
			`name: ${file.name}`,
			`path: ${file.relativePath}`,
			`root: ${file.rootLabel} (${file.rootId})`,
			`size: ${formatBytes(file.sizeBytes)}`,
			`modified: ${file.modifiedAt ?? 'unknown'}`
		].join('\n');
		await navigator.clipboard.writeText(details);
		actionMessage = `Copied details for ${file.name}.`;
	}

	function setViewMode(mode: ViewMode) {
		viewMode = mode;
		localStorage.setItem('homeops.files.viewMode', mode);
	}

	function setSort(key: SortKey) {
		if (sortKey === key) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDirection = key === 'modified' || key === 'size' ? 'desc' : 'asc';
		}
		localStorage.setItem('homeops.files.sortKey', sortKey);
		localStorage.setItem('homeops.files.sortDirection', sortDirection);
	}

	function loadViewPreferences() {
		const savedView = localStorage.getItem('homeops.files.viewMode');
		if (savedView === 'list' || savedView === 'grid') viewMode = savedView;
		const savedSort = localStorage.getItem('homeops.files.sortKey');
		if (savedSort === 'type' || savedSort === 'name' || savedSort === 'size' || savedSort === 'modified') sortKey = savedSort;
		const savedDirection = localStorage.getItem('homeops.files.sortDirection');
		if (savedDirection === 'asc' || savedDirection === 'desc') sortDirection = savedDirection;
	}

	function loadPendingProjectTarget() {
		const raw = localStorage.getItem('homeops.files.openTarget');
		if (!raw) return;
		localStorage.removeItem('homeops.files.openTarget');
		try {
			const target = JSON.parse(raw) as { rootId?: string; path?: string };
			if (target.rootId) storageRoots.select(target.rootId);
			currentPath = target.path ?? '';
		} catch {
			currentPath = '';
		}
	}

	function guardUploadingPath(file: FileEntry) {
		if (!uploadState.isUploadingPath(file.relativePath)) return false;
		error = `${file.name} is still uploading. Actions are disabled until it completes.`;
		return true;
	}

	function formatBytes(bytes: number) {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	function statusText(status: string) {
		if (status === 'preparing') return 'Preparing';
		if (status === 'uploading') return 'Uploading';
		if (status === 'finalizing') return 'Finalizing';
		if (status === 'completed') return 'Completed';
		if (status === 'failed') return 'Failed';
		return status;
	}
</script>

<svelte:head><title>Files · HomeOps Panel</title></svelte:head>
<h2 class="sr-only">Files page showing directory listing with file names, sizes, dates, and actions</h2>
<div class="files-page">
	<Topbar title="Files" flush>
		<SearchInput placeholder="Filter current folder…" bind:value={searchQuery} />
		<select class="root-select" value={storageRoots.selectedRootId} onchange={selectRoot} title="Active storage root">
			<option value={ALL_STORAGE_ROOT_ID}>All storage</option>
			{#each storageRootOptions as root}
				<option value={root.id}>{root.label}</option>
			{/each}
		</select>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-upload" label="Upload" onclick={chooseUploadFiles} />
		<SmallButton icon="ti-folder-plus" label="New folder" onclick={createNewFolder} />
	</Topbar>
	{#if isAllStorage}
		<div class="root-bar smart">
			<span class="smart-badge">All storage</span>
			<strong>Virtual merged explorer</strong>
			<span>{smartPool?.displayName ?? 'Smart Storage Pool'}</span>
			<span>Large files & redux-corpus → bulk · small files → main</span>
			<a href="/resources#storage">Pool details</a>
		</div>
	{:else if selectedRoot}
		<div class="root-bar">
			<strong>{selectedRoot.label}</strong>
			<span>{selectedRoot.path}</span>
			<span>{formatBytes(selectedRoot.freeBytes ?? 0)} free</span>
			<span>{selectedRoot.usagePercent?.toFixed(1) ?? '0.0'}% used</span>
		</div>
	{/if}
	<div class="viewbar">
		<button class:active={fileFilter === 'all'} type="button" onclick={() => setFileFilter('all')}>All</button>
		<button class:active={fileFilter === 'archives'} type="button" onclick={() => setFileFilter('archives')}>Archives</button>
		<button class:active={fileFilter === 'folders'} type="button" onclick={() => setFileFilter('folders')}>Folders</button>
		<button class:active={fileFilter === 'large'} type="button" onclick={() => setFileFilter('large')}>Large files</button>
		<button class:active={fileFilter === 'recent'} type="button" onclick={() => setFileFilter('recent')}>Recent</button>
		<span class="divider"></span>
		<button class:active={viewMode === 'list'} type="button" onclick={() => setViewMode('list')}><i class="ti ti-list"></i> List</button>
		<button class:active={viewMode === 'grid'} type="button" onclick={() => setViewMode('grid')}><i class="ti ti-layout-grid"></i> Grid</button>
		<button class:active={sortKey === 'type'} type="button" onclick={() => setSort('type')}>Type {sortKey === 'type' ? sortDirection : ''}</button>
		<button class:active={sortKey === 'name'} type="button" onclick={() => setSort('name')}>Name {sortKey === 'name' ? sortDirection : ''}</button>
		<button class:active={sortKey === 'size'} type="button" onclick={() => setSort('size')}>Size {sortKey === 'size' ? sortDirection : ''}</button>
		<button class:active={sortKey === 'modified'} type="button" onclick={() => setSort('modified')}>Modified {sortKey === 'modified' ? sortDirection : ''}</button>
	</div>
	<input bind:this={uploadInput} class="upload-input" type="file" multiple onchange={handleUpload} />
	<FileToolbar parts={breadcrumbParts} onnavigate={navigateBreadcrumb} onrefresh={refresh} />
	<DownloadProgress />
	{#if error}<div class="notice error">{error}</div>{/if}
	{#if actionMessage}
		<div class="notice success">
			<span>{actionMessage}</span>
			<a href="/operations">Operations</a>
		</div>
	{/if}
	{#if viewMode === 'list'}
		<FileTable
			files={visibleFiles}
			ondirectoryopen={openDirectory}
			ondownload={downloadEntry}
			onextract={extractEntry}
			ondetails={copyDetails}
			onrename={renameEntry}
			onmove={moveEntry}
			ondelete={deleteEntry}
			{allowDelete}
			isUploading={(path) => uploadState.isUploadingPath(path)}
			uploadForPath={(path) => uploadState.activeForPath(path)}
		/>
	{:else}
		<div class="file-grid">
			{#each visibleFiles as file}
				<div class:uploading={uploadState.isUploadingPath(file.relativePath)} class="grid-card">
					<button class="grid-main" type="button" onclick={() => openDirectory(file)} disabled={file.kind !== 'directory' || uploadState.isUploadingPath(file.relativePath)}>
						<i class={`ti ${file.kind === 'directory' ? 'ti-folder' : 'ti-file'} ${file.extension === 'zip' ? 'zip' : ''}`}></i>
						<strong>{file.name}</strong>
						<span>{file.kind === 'directory' ? 'Folder' : formatBytes(file.sizeBytes)} · {file.modifiedAt ? new Date(file.modifiedAt).toLocaleDateString() : 'Unknown'}</span>
						<span class:merged={file.sourceRootIds.length > 1} class="grid-root">{file.sourceRootIds.length > 1 ? file.sourceRootIds.join('+') : file.rootLabel}</span>
						{#if file.conflict}<span class="conflict">{file.conflict === 'merged-directory' ? 'Merged folder' : 'Duplicate name'}</span>{/if}
						{#if uploadState.activeForPath(file.relativePath)}<em>Uploading {uploadState.activeForPath(file.relativePath)?.percent}%</em>{/if}
					</button>
					<div class="grid-actions">
						{#if file.kind === 'file'}<IconButton icon="ti-download" label="Download" onclick={() => downloadEntry(file)} disabled={uploadState.isUploadingPath(file.relativePath)} />{/if}
						{#if file.kind === 'file' && file.extension === 'zip'}<IconButton icon="ti-archive" label="Extract" onclick={() => extractEntry(file)} disabled={uploadState.isUploadingPath(file.relativePath)} />{/if}
						{#if file.kind === 'file' && file.extension === 'zip'}<IconButton icon="ti-copy" label="Copy path/details" onclick={() => copyDetails(file)} disabled={uploadState.isUploadingPath(file.relativePath)} />{/if}
						<IconButton icon="ti-pencil" label="Rename" onclick={() => renameEntry(file)} disabled={uploadState.isUploadingPath(file.relativePath)} />
						<IconButton icon="ti-arrows-move" label="Move to..." onclick={() => moveEntry(file)} disabled={uploadState.isUploadingPath(file.relativePath)} />
						<IconButton icon="ti-trash" label={allowDelete ? 'Delete' : 'Delete is disabled by server config'} onclick={() => deleteEntry(file)} disabled={!allowDelete || uploadState.isUploadingPath(file.relativePath)} />
					</div>
				</div>
			{/each}
		</div>
	{/if}
	{#if uploadState.uploads.length}
		<div class="upload-panel" aria-live="polite">
			<div class="upload-head">
				<strong>Uploads</strong>
				<button type="button" onclick={() => uploadState.clearFinished()}>Clear finished</button>
			</div>
			{#each uploadState.uploads as upload}
				<div class="upload-item">
					<div class="upload-meta">
						<strong>{upload.filename}</strong>
						<span>{upload.destinationPath}</span>
					</div>
					<div class="upload-status">
						<span>{statusText(upload.status)}</span>
						<span>{formatBytes(upload.uploadedBytes)} / {formatBytes(upload.totalBytes)}</span>
						<span>{upload.percent}%</span>
					</div>
					<div class="upload-bar"><div style={`width:${upload.percent}%`}></div></div>
					{#if upload.error}<div class="upload-error">{upload.error}</div>{/if}
				</div>
			{/each}
		</div>
	{/if}
	<StatusBar items={[`${visibleFiles.length} shown / ${files.length} items`, fileFilter === 'all' ? 'all file types' : `${fileFilter} filter`, currentPath || 'workspace root', lastLoadedAt ? `updated ${lastLoadedAt}` : 'not loaded']} />
</div>

<style>
	.files-page { position: relative; height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.upload-input { display: none; }
	.root-select { min-width: 150px; height: 36px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-size: 12px; padding: 0 8px; }
	.root-bar, .viewbar { display: flex; align-items: center; gap: 12px; padding: 7px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-sidebar); font-size: 11px; color: var(--color-text-secondary); }
	.root-bar strong { color: var(--color-text-primary); }
	.root-bar.smart { background: color-mix(in srgb, var(--accent) 8%, var(--bg-sidebar)); }
	.root-bar.smart a { margin-left: auto; color: var(--accent); text-decoration: none; }
	.root-bar.smart a:hover { text-decoration: underline; }
	.smart-badge { border: 0.5px solid var(--accent); color: var(--accent); border-radius: 999px; font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; padding: 2px 8px; }
	.root-bar span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.divider { width: 1px; height: 18px; background: var(--color-border-tertiary); }
	.viewbar button { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; background: transparent; color: var(--color-text-secondary); font-size: 11px; padding: 3px 9px; cursor: pointer; }
	.viewbar button.active { background: var(--bg-app); color: var(--color-text-primary); }
	.file-grid { flex: 1; overflow: auto; display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 10px; padding: 14px; background: var(--bg-surface); align-content: start; }
	.grid-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); overflow: hidden; }
	.grid-card.uploading { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 8%, var(--bg-app)); }
	.grid-main { width: 100%; min-height: 112px; display: flex; flex-direction: column; align-items: flex-start; gap: 7px; padding: 13px; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
	.grid-main:disabled { cursor: default; }
	.grid-main i { font-size: 24px; color: #ba7517; }
	.grid-main i.zip { color: #d85a30; }
	.grid-main strong { max-width: 100%; color: var(--color-text-primary); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.grid-main span, .grid-main em { color: var(--color-text-tertiary); font-size: 11px; font-style: normal; }
	.grid-main em { color: var(--color-text-info); }
	.grid-root, .conflict { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; padding: 2px 7px; color: var(--accent) !important; }
	.grid-root.merged, .conflict { color: var(--color-text-warning) !important; }
	.grid-actions { display: flex; justify-content: flex-end; gap: 4px; padding: 7px; border-top: 0.5px solid var(--color-border-tertiary); }
	.notice { padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { display: flex; gap: 10px; align-items: center; color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.notice a { color: var(--accent); text-decoration: none; }
	.notice a:hover { text-decoration: underline; }
	.upload-panel { position: absolute; right: 18px; bottom: 36px; z-index: 4; width: min(420px, calc(100% - 36px)); max-height: 52vh; overflow: auto; background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); box-shadow: 0 18px 45px rgba(0, 0, 0, 0.32); }
	.upload-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-primary); }
	.upload-head button { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface-2); color: var(--color-text-secondary); font-size: 11px; padding: 4px 8px; cursor: pointer; }
	.upload-item { padding: 10px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.upload-item:last-child { border-bottom: 0; }
	.upload-meta { display: flex; justify-content: space-between; gap: 10px; font-size: 12px; }
	.upload-meta strong { color: var(--color-text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.upload-meta span, .upload-status { color: var(--color-text-tertiary); font-size: 11px; }
	.upload-status { margin-top: 5px; display: flex; justify-content: space-between; gap: 8px; }
	.upload-bar { margin-top: 7px; height: 5px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; }
	.upload-bar div { height: 100%; background: var(--accent); }
	.upload-error { margin-top: 6px; color: var(--color-text-danger); font-size: 11px; }
</style>
