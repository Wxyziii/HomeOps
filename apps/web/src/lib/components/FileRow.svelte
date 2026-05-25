<script lang="ts">
	import IconButton from './IconButton.svelte';
	import type { FileEntry } from '$lib/api/client';
	let {
		file,
		ondirectoryopen,
		ondownload,
		onextract,
		onrename,
		allowDelete = false
	}: {
		file: FileEntry;
		ondirectoryopen?: (file: FileEntry) => void;
		ondownload?: (file: FileEntry) => void;
		onextract?: (file: FileEntry) => void;
		onrename?: (file: FileEntry) => void;
		allowDelete?: boolean;
	} = $props();
	const iconMap: Record<string, string> = { directory: 'ti-folder', file: 'ti-file', symlink: 'ti-link', other: 'ti-file-alert' };
	const typeClass = $derived(file.kind === 'directory' ? 'folder' : file.extension ?? file.kind);
	const canExtract = $derived(file.kind === 'file' && file.extension === 'zip');

	function formatSize(bytes: number) {
		if (file.kind === 'directory') return '—';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	function formatDate(value: string | null) {
		if (!value) return '—';
		return new Date(value).toLocaleString([], {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function modeText() {
		if (file.kind === 'directory') return file.readonly ? 'dr-x' : 'drwx';
		return file.readonly ? '-r--' : '-rw-';
	}
</script>

<tr>
	<td><input type="checkbox" aria-label={`Select ${file.name}`} /></td>
	<td>
		<button class="file-name" type="button" onclick={() => file.kind === 'directory' && ondirectoryopen?.(file)} disabled={file.kind !== 'directory'}>
			<i class="ti {iconMap[file.kind] ?? 'ti-file'} file-icon fi-{typeClass}" aria-hidden="true"></i>
			<span>{file.name}</span>
			{#if file.warnings.length}<i class="ti ti-alert-triangle warn" title={file.warnings.join(' ')} aria-hidden="true"></i>{/if}
		</button>
	</td>
	<td class="size-col">{formatSize(file.sizeBytes)}</td>
	<td class="date-col">{formatDate(file.modifiedAt)}</td>
	<td class="perm-col">{modeText()}</td>
	<td class="action-col">
		<div class="row-actions">
			{#if file.kind === 'file'}<IconButton icon="ti-download" label="Download" onclick={() => ondownload?.(file)} />{/if}
			{#if canExtract}<IconButton icon="ti-archive" label="Extract" onclick={() => onextract?.(file)} />{/if}
			<IconButton icon="ti-pencil" label="Rename" onclick={() => onrename?.(file)} />
			{#if allowDelete}<IconButton icon="ti-trash" label="Delete disabled" />{/if}
			<IconButton icon="ti-dots-vertical" label="More actions" />
		</div>
	</td>
</tr>

<style>
	td { padding: 9px 12px; font-size: 13px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	tr:hover td { background: var(--bg-surface-2); }
	.file-name { width: 100%; display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; border: 0; padding: 0; background: transparent; color: inherit; font: inherit; text-align: left; }
	.file-name:not(:disabled) { cursor: pointer; }
	.file-name:disabled { cursor: default; }
	.file-name span { overflow: hidden; text-overflow: ellipsis; }
	.file-icon { font-size: 16px; flex-shrink: 0; }
	.fi-folder { color: #ba7517; }
	.fi-jpg, .fi-jpeg, .fi-png, .fi-gif, .fi-webp { color: var(--accent); }
	.fi-mp4, .fi-mov, .fi-mkv { color: #7f77dd; }
	.fi-pdf, .fi-doc, .fi-docx { color: #1d9e75; }
	.fi-zip, .fi-7z, .fi-rar { color: #d85a30; }
	.fi-rs, .fi-ts, .fi-js, .fi-json, .fi-md { color: #888780; }
	.warn { color: var(--color-text-warning); font-size: 13px; }
	.size-col { color: var(--color-text-secondary); width: 80px; }
	.date-col { color: var(--color-text-secondary); width: 130px; }
	.perm-col { color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 11px; width: 90px; }
	.action-col { width: 70px; text-align: right; }
	.row-actions { display: flex; gap: 4px; justify-content: flex-end; }
</style>
