<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import {
		createProject,
		deleteProjectMetadata,
		getWorkspaceStatus,
		listProjects,
		updateProject,
		type Project,
		type ProjectStatus,
		type StorageRootStatus
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import { storageRoots } from '$lib/stores/storageRoots.svelte';

	let projects = $state<Project[]>([]);
	let storageRootOptions = $state<StorageRootStatus[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);
	let searchQuery = $state('');
	let statusFilter = $state<'active' | 'archived' | 'all'>('active');
	let rootFilter = $state('all');
	let showCreate = $state(false);
	let formName = $state('');
	let formRootId = $state('main');
	let formPath = $state('projects/');
	let formTags = $state('');
	let formNotes = $state('');
	let formMode = $state<'create' | 'attach'>('create');

	const filteredProjects = $derived(
		projects
			.filter((project) => statusFilter === 'all' || project.status === statusFilter)
			.filter((project) => rootFilter === 'all' || project.rootId === rootFilter)
			.filter((project) => {
				const query = searchQuery.trim().toLowerCase();
				if (!query) return true;
				return [project.name, project.relativePath, project.notes ?? '', ...project.tags]
					.some((value) => value.toLowerCase().includes(query));
			})
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.updatedAt.localeCompare(a.updatedAt))
	);

	onMount(() => {
		serverConnection.load();
		storageRoots.load();
		void refresh();
	});

	async function refresh() {
		loading = true;
		error = null;
		try {
			const [projectResponse, workspace] = await Promise.all([
				listProjects(serverConnection.serverUrl),
				getWorkspaceStatus(serverConnection.serverUrl)
			]);
			projects = projectResponse.projects;
			storageRootOptions = workspace.storage_roots;
			if (!storageRootOptions.some((root) => root.id === formRootId)) {
				formRootId = storageRootOptions[0]?.id ?? 'main';
			}
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load projects.';
		} finally {
			loading = false;
		}
	}

	async function submitProject() {
		const name = formName.trim();
		const relativePath = formPath.trim();
		if (!name || !relativePath) {
			error = 'Project name and path are required.';
			return;
		}
		try {
			const response = await createProject(serverConnection.serverUrl, {
				name,
				rootId: formRootId,
				relativePath,
				notes: formNotes.trim() || undefined,
				status: 'active',
				tags: parseTags(formTags),
				pinned: false,
				createFolder: formMode === 'create',
				attachExisting: formMode === 'attach'
			});
			actionMessage = `Project created: ${response.project.name}.`;
			showCreate = false;
			formName = '';
			formPath = 'projects/';
			formTags = '';
			formNotes = '';
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not create project.';
		}
	}

	function openInFiles(project: Project) {
		storageRoots.select(project.rootId);
		localStorage.setItem(
			'homeops.files.openTarget',
			JSON.stringify({ rootId: project.rootId, path: project.relativePath })
		);
		void goto('/files');
	}

	async function editProject(project: Project) {
		const name = window.prompt('Project name', project.name);
		if (name === null) return;
		const notes = window.prompt('Notes', project.notes ?? '');
		if (notes === null) return;
		const tags = window.prompt('Tags, comma-separated', project.tags.join(', '));
		if (tags === null) return;
		try {
			await updateProject(serverConnection.serverUrl, project.id, {
				name,
				notes,
				tags: parseTags(tags)
			});
			actionMessage = `Updated ${project.name}.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not update project.';
		}
	}

	async function setStatus(project: Project, status: ProjectStatus) {
		try {
			await updateProject(serverConnection.serverUrl, project.id, { status });
			actionMessage = `${project.name} marked ${status}. Files were not changed.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not update status.';
		}
	}

	async function togglePinned(project: Project) {
		try {
			await updateProject(serverConnection.serverUrl, project.id, { pinned: !project.pinned });
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not update pin.';
		}
	}

	async function deleteMetadata(project: Project) {
		const confirmed = window.confirm(`Delete project metadata for ${project.name}?\n\nFiles and folders will remain on disk.`);
		if (!confirmed) return;
		try {
			await deleteProjectMetadata(serverConnection.serverUrl, project.id);
			actionMessage = `Deleted metadata for ${project.name}. Files remain on disk.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not delete project metadata.';
		}
	}

	async function copyPath(project: Project) {
		await navigator.clipboard.writeText(project.relativePath);
		actionMessage = `Copied ${project.relativePath}.`;
	}

	function parseTags(value: string) {
		return value
			.split(',')
			.map((tag) => tag.trim())
			.filter(Boolean);
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
</script>

<svelte:head><title>Projects · HomeOps Panel</title></svelte:head>
<div class="projects-page">
	<Topbar title="Projects" flush>
		<SearchInput placeholder="Filter projects..." bind:value={searchQuery} />
		<select bind:value={statusFilter}>
			<option value="active">Active</option>
			<option value="archived">Archived</option>
			<option value="all">All</option>
		</select>
		<select bind:value={rootFilter}>
			<option value="all">All roots</option>
			{#each storageRootOptions as root}<option value={root.id}>{root.label}</option>{/each}
		</select>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-plus" label="Create project" onclick={() => { showCreate = !showCreate; }} />
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if actionMessage}<div class="notice success">{actionMessage}</div>{/if}

		{#if showCreate}
			<section class="panel create-panel">
				<div class="panel-head">
					<div>Create / attach workspace</div>
					<span>Metadata only. Files stay inside selected storage root.</span>
				</div>
				<div class="form-grid">
					<label>Name<input bind:value={formName} placeholder="Redux Circle" /></label>
					<label>Storage root<select bind:value={formRootId}>{#each storageRootOptions as root}<option value={root.id}>{root.label}</option>{/each}</select></label>
					<label>Relative path<input bind:value={formPath} placeholder="projects/redux-circle" /></label>
					<label>Mode<select bind:value={formMode}><option value="create">Create folder</option><option value="attach">Attach existing folder</option></select></label>
					<label>Tags<input bind:value={formTags} placeholder="modding, redux" /></label>
					<label class="notes">Notes<textarea bind:value={formNotes} placeholder="Working copy notes"></textarea></label>
				</div>
				<div class="form-actions">
					<SmallButton icon="ti-check" label="Save project" onclick={submitProject} />
					<SmallButton icon="ti-x" label="Cancel" onclick={() => { showCreate = false; }} />
				</div>
			</section>
		{/if}

		{#if filteredProjects.length}
			<div class="project-grid">
				{#each filteredProjects as project}
					<article class:missing={!project.folderExists} class="project-card">
						<div class="project-head">
							<div>
								<h3>{project.name}</h3>
								<div class="path">{project.rootLabel} · {project.relativePath}</div>
							</div>
							<StatusBadge status={project.status} />
						</div>
						{#if project.notes}<p>{project.notes}</p>{/if}
						<div class="tag-row">{#each project.tags as tag}<span>{tag}</span>{/each}</div>
						<div class="stats">
							{#if project.stats}
								<span>{formatBytes(project.stats.sizeBytes)}</span>
								<span>{project.stats.fileCount} files</span>
								<span>{project.stats.folderCount} folders</span>
								{#if project.stats.truncated}<span>stats limited</span>{/if}
							{:else}
								<span>{project.folderMissingReason ?? 'Stats unavailable'}</span>
							{/if}
							<span>Updated {new Date(project.updatedAt).toLocaleString()}</span>
						</div>
						{#if !project.folderExists}<div class="warning">{project.folderMissingReason}</div>{/if}
						<div class="actions">
							<button type="button" onclick={() => openInFiles(project)} disabled={!project.folderExists}>Open in Files</button>
							<button type="button" onclick={() => editProject(project)}>Edit</button>
							<button type="button" onclick={() => togglePinned(project)}>{project.pinned ? 'Unpin' : 'Pin'}</button>
							<button type="button" onclick={() => setStatus(project, project.status === 'active' ? 'archived' : 'active')}>{project.status === 'active' ? 'Archive' : 'Unarchive'}</button>
							<button type="button" onclick={() => copyPath(project)}>Copy path</button>
							<button type="button" onclick={() => deleteMetadata(project)}>Delete metadata</button>
						</div>
					</article>
				{/each}
			</div>
		{:else if loading}
			<div class="empty">Loading projects...</div>
		{:else}
			<div class="empty">No projects yet. Create a project to group files, archives, and jobs into a workspace.</div>
		{/if}
	</div>
</div>

<style>
	.projects-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	select, input, textarea { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-size: 12px; padding: 8px; }
	.content { flex: 1; overflow: auto; padding: 18px; display: flex; flex-direction: column; gap: 12px; background: var(--bg-surface); }
	.notice, .empty { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 10px 12px; color: var(--color-text-secondary); font-size: 12px; background: var(--bg-app); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.panel, .project-card { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); }
	.panel-head { display: flex; justify-content: space-between; gap: 12px; padding: 12px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 13px; font-weight: 600; }
	.panel-head span { color: var(--color-text-tertiary); font-size: 11px; font-weight: 400; }
	.form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; padding: 14px; }
	label { display: flex; flex-direction: column; gap: 6px; color: var(--color-text-secondary); font-size: 11px; }
	.notes { grid-column: 1 / -1; }
	textarea { min-height: 76px; resize: vertical; }
	.form-actions { display: flex; gap: 8px; padding: 0 14px 14px; }
	.project-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 12px; }
	.project-card { padding: 14px; display: flex; flex-direction: column; gap: 10px; }
	.project-card.missing { border-color: var(--warning); }
	.project-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }
	h3 { margin: 0; color: var(--color-text-primary); font-size: 14px; }
	.path { margin-top: 4px; color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 10px; word-break: break-all; }
	p { margin: 0; color: var(--color-text-secondary); font-size: 12px; line-height: 1.45; }
	.tag-row, .stats, .actions { display: flex; flex-wrap: wrap; gap: 7px; }
	.tag-row span { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; padding: 2px 7px; color: var(--color-text-secondary); font-size: 10px; }
	.stats { color: var(--color-text-tertiary); font-size: 11px; }
	.warning { color: var(--warning); font-size: 11px; }
	.actions { border-top: 0.5px solid var(--color-border-tertiary); padding-top: 10px; }
	.actions button { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-surface-2); color: var(--color-text-secondary); padding: 5px 8px; font-size: 11px; cursor: pointer; }
	.actions button:hover:not(:disabled) { color: var(--color-text-primary); border-color: var(--accent); }
	.actions button:disabled { opacity: 0.45; cursor: not-allowed; }
	@media (max-width: 900px) { .form-grid { grid-template-columns: 1fr; } .project-grid { grid-template-columns: 1fr; } }
</style>
