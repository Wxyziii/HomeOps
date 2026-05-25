<script lang="ts">
	import IconButton from './IconButton.svelte';
	let { file }: { file: { type: string; name: string; size: string; modified: string; permissions: string } } = $props();
	const iconMap: Record<string, string> = { folder: 'ti-folder', image: 'ti-file-type-jpg', video: 'ti-file-type-mp4', doc: 'ti-file-type-pdf', zip: 'ti-file-zip', code: 'ti-file-code' };
</script>

<tr>
	<td><input type="checkbox" aria-label={`Select ${file.name}`} /></td>
	<td><div class="file-name"><i class="ti {iconMap[file.type] ?? 'ti-file'} file-icon fi-{file.type}" aria-hidden="true"></i> {file.name}</div></td>
	<td class="size-col">{file.size}</td>
	<td class="date-col">{file.modified}</td>
	<td class="perm-col">{file.permissions}</td>
	<td class="action-col"><div class="row-actions">{#if file.type !== 'folder'}<IconButton icon="ti-download" label="Download placeholder" />{/if}<IconButton icon="ti-dots-vertical" label="More actions" /></div></td>
</tr>

<style>
	td { padding: 9px 12px; font-size: 13px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	tr:hover td { background: var(--bg-surface-2); }
	.file-name { display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
	.file-icon { font-size: 16px; flex-shrink: 0; }
	.fi-folder { color: #ba7517; }
	.fi-image { color: var(--accent); }
	.fi-video { color: #7f77dd; }
	.fi-doc { color: #1d9e75; }
	.fi-zip { color: #d85a30; }
	.fi-code { color: #888780; }
	.size-col { color: var(--color-text-secondary); width: 80px; }
	.date-col { color: var(--color-text-secondary); width: 130px; }
	.perm-col { color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 11px; width: 90px; }
	.action-col { width: 70px; text-align: right; }
	.row-actions { display: flex; gap: 4px; justify-content: flex-end; }
</style>
