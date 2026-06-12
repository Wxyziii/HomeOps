<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';
	import {
		deleteMinecraftFile,
		formatBytes,
		listMinecraftFiles,
		readMinecraftFile,
		renameMinecraftFile,
		writeMinecraftFile,
		type MinecraftFileEntry
	} from '$lib/api/minecraft';

	let currentPath = $state('');
	let items = $state<MinecraftFileEntry[]>([]);
	let loading = $state(false);
	let pageError = $state<string | null>(null);

	let editorPath = $state<string | null>(null);
	let editorContent = $state('');
	let editorDirty = $state(false);
	let editorBusy = $state(false);
	let editorError = $state<string | null>(null);
	let editorSavedAt = $state<string | null>(null);

	onMount(async () => {
		serverConnection.load();
		await load('');
	});

	const breadcrumbs = $derived.by(() => {
		const parts = currentPath.split('/').filter(Boolean);
		return parts.map((part, index) => ({
			label: part,
			path: parts.slice(0, index + 1).join('/')
		}));
	});

	async function load(path: string) {
		loading = true;
		pageError = null;
		try {
			const response = await listMinecraftFiles(serverConnection.serverUrl, path);
			currentPath = response.path;
			items = response.items;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not list files.';
		} finally {
			loading = false;
		}
	}

	async function open(item: MinecraftFileEntry) {
		if (item.kind === 'directory') {
			await load(item.relativePath);
			return;
		}
		if (!item.editable) return;
		editorError = null;
		editorSavedAt = null;
		try {
			const response = await readMinecraftFile(serverConnection.serverUrl, item.relativePath);
			editorPath = response.path;
			editorContent = response.content;
			editorDirty = false;
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Could not open file.';
		}
	}

	async function saveEditor() {
		if (!editorPath || editorBusy) return;
		editorBusy = true;
		editorError = null;
		try {
			await writeMinecraftFile(serverConnection.serverUrl, editorPath, editorContent);
			editorDirty = false;
			editorSavedAt = new Date().toLocaleTimeString();
		} catch (error) {
			editorError = error instanceof Error ? error.message : 'Could not save file.';
		} finally {
			editorBusy = false;
		}
	}

	async function rename(item: MinecraftFileEntry) {
		const target = prompt(`Rename ${item.name} to:`, item.name);
		if (!target || target === item.name) return;
		const parent = item.relativePath.split('/').slice(0, -1).join('/');
		const to = parent ? `${parent}/${target}` : target;
		try {
			await renameMinecraftFile(serverConnection.serverUrl, item.relativePath, to);
			await load(currentPath);
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Rename failed.';
		}
	}

	async function remove(item: MinecraftFileEntry) {
		if (!confirm(`Move ${item.name} to the HomeOps trash folder?`)) return;
		try {
			await deleteMinecraftFile(serverConnection.serverUrl, item.relativePath);
			await load(currentPath);
		} catch (error) {
			pageError = error instanceof Error ? error.message : 'Delete failed.';
		}
	}

	function iconFor(item: MinecraftFileEntry): string {
		if (item.kind === 'directory') return 'ti-folder';
		if (item.extension === 'jar') return 'ti-package';
		if (item.editable) return 'ti-file-text';
		return 'ti-file';
	}
</script>

<svelte:head><title>Minecraft files · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Server files">
		<SmallButton icon="ti-refresh" label="Refresh" onclick={() => void load(currentPath)} />
	</Topbar>

	{#if pageError}<div class="notice error">{pageError}</div>{/if}

	<div class="crumbs">
		<button class="crumb" onclick={() => void load('')}>server root</button>
		{#each breadcrumbs as crumb}
			<span class="crumb-sep">/</span>
			<button class="crumb" onclick={() => void load(crumb.path)}>{crumb.label}</button>
		{/each}
	</div>

	<div class="file-table">
		<div class="file-row header">
			<span>Name</span><span>Size</span><span>Modified</span><span>Actions</span>
		</div>
		{#if loading}
			<div class="empty-state">Loading…</div>
		{:else}
			{#each items as item}
				<div class="file-row">
					<button class="file-name" onclick={() => void open(item)} disabled={item.kind !== 'directory' && !item.editable}>
						<i class="ti {iconFor(item)}" aria-hidden="true"></i>
						<span>{item.name}</span>
						{#if item.protected}<span class="tag">protected</span>{/if}
					</button>
					<span>{item.kind === 'directory' ? '—' : formatBytes(item.sizeBytes)}</span>
					<span>{item.modifiedAt ? new Date(item.modifiedAt).toLocaleString() : '—'}</span>
					<span class="row-actions">
						{#if !item.protected}
							<button class="icon-btn" title="Rename" onclick={() => void rename(item)}><i class="ti ti-pencil" aria-hidden="true"></i></button>
							<button class="icon-btn danger" title="Move to trash" onclick={() => void remove(item)}><i class="ti ti-trash" aria-hidden="true"></i></button>
						{/if}
					</span>
				</div>
			{:else}
				<div class="empty-state">This folder is empty.</div>
			{/each}
		{/if}
	</div>

	{#if editorPath}
		<div class="editor">
			<div class="editor-head">
				<strong class="mono">{editorPath}</strong>
				<div class="editor-actions">
					{#if editorSavedAt}<span class="saved">Saved {editorSavedAt}</span>{/if}
					{#if editorDirty}<span class="dirty">Unsaved changes</span>{/if}
					<SmallButton icon="ti-device-floppy" label={editorBusy ? 'Saving…' : 'Save'} disabled={editorBusy || !editorDirty} onclick={saveEditor} />
					<SmallButton icon="ti-x" label="Close" onclick={() => { editorPath = null; }} />
				</div>
			</div>
			{#if editorError}<div class="notice error">{editorError}</div>{/if}
			<textarea bind:value={editorContent} oninput={() => (editorDirty = true)} spellcheck="false"></textarea>
		</div>
	{/if}
</div>

<style>
	.page { padding: 20px; display: flex; flex-direction: column; gap: 12px; }
	.notice { padding: 8px 10px; border-radius: var(--border-radius-md); font-size: 12px; border: 0.5px solid var(--color-border-tertiary); }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.crumbs { display: flex; align-items: center; gap: 4px; font-size: 12px; flex-wrap: wrap; }
	.crumb { background: none; border: none; color: var(--accent); cursor: pointer; padding: 2px 4px; font-size: 12px; }
	.crumb:hover { text-decoration: underline; }
	.crumb-sep { color: var(--color-text-tertiary); }
	.file-table { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; }
	.file-row { display: grid; grid-template-columns: minmax(0, 1fr) 90px 170px 80px; align-items: center; gap: 10px; padding: 7px 12px; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 12px; color: var(--color-text-secondary); }
	.file-row.header { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-text-tertiary); background: var(--bg-surface); }
	.file-name { display: flex; align-items: center; gap: 8px; background: none; border: none; color: var(--color-text-primary); cursor: pointer; font-size: 12px; padding: 0; min-width: 0; text-align: left; }
	.file-name:disabled { cursor: default; }
	.file-name span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.file-name i { color: var(--color-text-secondary); font-size: 15px; }
	.tag { font-size: 9px; padding: 1px 6px; border-radius: 8px; background: var(--color-background-warning); color: var(--color-text-warning); }
	.row-actions { display: flex; gap: 4px; justify-content: flex-end; }
	.icon-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; padding: 3px; border-radius: 4px; }
	.icon-btn:hover { background: var(--bg-surface-2); color: var(--color-text-primary); }
	.icon-btn.danger:hover { color: var(--color-text-danger); }
	.empty-state { color: var(--color-text-tertiary); font-size: 12px; padding: 14px 12px; }
	.editor { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px; display: flex; flex-direction: column; gap: 8px; }
	.editor-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
	.editor-actions { display: flex; align-items: center; gap: 8px; }
	.mono { font-family: var(--font-mono); font-size: 12px; color: var(--color-text-primary); }
	.saved { font-size: 11px; color: var(--color-text-success); }
	.dirty { font-size: 11px; color: var(--color-text-warning); }
	textarea { min-height: 320px; background: var(--bg-surface); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; padding: 10px; resize: vertical; }
</style>
