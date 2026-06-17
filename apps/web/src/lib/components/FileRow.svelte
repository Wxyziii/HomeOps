<script lang="ts">
	import IconButton from './IconButton.svelte';
	import type { FileEntry } from '$lib/api/client';
	import type { UploadItem } from '$lib/stores/uploads.svelte';
	let {
		file,
		ondirectoryopen,
		ondownload,
		onextract,
		onrename,
		onmove,
		ondelete,
		allowDelete = false,
		isUploading,
		uploadForPath
	}: {
		file: FileEntry;
		ondirectoryopen?: (file: FileEntry) => void;
		ondownload?: (file: FileEntry) => void;
		onextract?: (file: FileEntry) => void;
		onrename?: (file: FileEntry) => void;
		onmove?: (file: FileEntry) => void;
		ondelete?: (file: FileEntry) => void;
		allowDelete?: boolean;
		isUploading?: (path: string) => boolean;
		uploadForPath?: (path: string) => UploadItem | undefined;
	} = $props();
	const iconMap: Record<string, string> = { directory: 'ti-folder', file: 'ti-file', symlink: 'ti-link', other: 'ti-file-alert' };
	const typeClass = $derived(file.kind === 'directory' ? 'folder' : file.extension ?? file.kind);
	const canExtract = $derived(file.kind === 'file' && file.extension === 'zip');
	const uploading = $derived(isUploading?.(file.relativePath) ?? false);
	const upload = $derived(uploadForPath?.(file.relativePath));
	const disabledReason = $derived(uploading ? `${file.name} is uploading. Actions are disabled until upload completes.` : undefined);

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

<tr class:uploading>
	<td><input type="checkbox" aria-label={`Select ${file.name}`} title="Bulk selection planned for later" disabled /></td>
	<td>
		<button class="file-name" type="button" title={disabledReason} onclick={() => file.kind === 'directory' && !uploading && ondirectoryopen?.(file)} disabled={file.kind !== 'directory' || uploading}>
			<i class="ti {iconMap[file.kind] ?? 'ti-file'} file-icon fi-{typeClass}" aria-hidden="true"></i>
			<span>{file.name}</span>
			{#if upload}<em>Uploading {upload.percent}%</em>{/if}
			{#if file.warnings.length}<i class="ti ti-alert-triangle warn" title={file.warnings.join(' ')} aria-hidden="true"></i>{/if}
		</button>
	</td>
	<td class="size-col">{formatSize(file.sizeBytes)}</td>
	<td class="date-col">{formatDate(file.modifiedAt)}</td>
	<td class="perm-col">{modeText()}</td>
	<td class="action-col">
		<div class="row-actions">
			{#if file.kind === 'file'}<IconButton icon="ti-download" label={disabledReason ?? 'Download'} onclick={() => ondownload?.(file)} disabled={uploading} />{/if}
			{#if canExtract}<IconButton icon="ti-archive" label={disabledReason ?? 'Extract'} onclick={() => onextract?.(file)} disabled={uploading} />{/if}
			<IconButton icon="ti-pencil" label={disabledReason ?? 'Rename'} onclick={() => onrename?.(file)} disabled={uploading} />
			<IconButton icon="ti-arrows-move" label={disabledReason ?? 'Move to...'} onclick={() => onmove?.(file)} disabled={uploading} />
			<IconButton icon="ti-trash" label={allowDelete ? (disabledReason ?? 'Delete') : 'Delete is disabled by server config'} onclick={() => ondelete?.(file)} disabled={!allowDelete || uploading} />
			<IconButton icon="ti-dots-vertical" label="More actions planned for later" disabled />
		</div>
	</td>
</tr>

<style>
	td { height: 44px; padding: 0 10px; font-size: 12px; border-bottom: 1px solid var(--color-border-tertiary); color: var(--color-text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	tr:hover td { background: var(--bg-hover); }
	tr.uploading td { background: var(--orange-bg); }
	.file-name { width: 100%; display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; border: 0; padding: 0; background: transparent; color: inherit; font: inherit; text-align: left; }
	.file-name:not(:disabled) { cursor: pointer; }
	.file-name:disabled { cursor: default; }
	.file-name span { overflow: hidden; text-overflow: ellipsis; color: var(--color-text-primary); }
	.file-name em { flex-shrink: 0; color: var(--color-text-info); font-size: 11px; font-style: normal; }
	.file-icon { font-size: 16px; flex-shrink: 0; }
	.fi-folder { color: var(--accent); }
	.fi-jpg, .fi-jpeg, .fi-png, .fi-gif, .fi-webp { color: var(--accent); }
	.fi-mp4, .fi-mov, .fi-mkv { color: var(--purple); }
	.fi-pdf, .fi-doc, .fi-docx { color: var(--green); }
	.fi-zip, .fi-7z, .fi-rar { color: var(--blue); }
	.fi-rs, .fi-ts, .fi-js, .fi-json, .fi-md { color: var(--color-text-tertiary); }
	.warn { color: var(--color-text-warning); font-size: 13px; }
	.size-col { color: var(--color-text-secondary); width: 100px; }
	.date-col { color: var(--color-text-secondary); width: 150px; }
	.perm-col { color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 11px; width: 115px; }
	.action-col { width: 140px; text-align: right; }
	.row-actions { display: flex; gap: 4px; justify-content: flex-end; }
</style>
