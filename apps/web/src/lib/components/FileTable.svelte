<script lang="ts">
	import FileRow from './FileRow.svelte';
	import type { FileEntry } from '$lib/api/client';
	import type { UploadItem } from '$lib/stores/uploads.svelte';
	let {
		files,
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
		files: FileEntry[];
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
</script>

<div class="file-table">
	<table>
		<thead>
			<tr>
				<th class="check-col"><input type="checkbox" aria-label="Select all" title="Bulk selection planned for later" disabled /></th>
				<th>Name</th>
				<th class="size-col">Size</th>
				<th class="date-col">Modified</th>
				<th class="perm-col">Permissions</th>
				<th class="action-col"></th>
			</tr>
		</thead>
		<tbody>{#each files as file}<FileRow {file} {ondirectoryopen} {ondownload} {onextract} {onrename} {onmove} {ondelete} {allowDelete} {isUploading} {uploadForPath} />{/each}</tbody>
	</table>
</div>

<style>
	.file-table { flex: 1; overflow: auto; background: var(--bg-app); }
	table { width: 100%; border-collapse: collapse; table-layout: fixed; }
	th { height: 36px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-faint); padding: 0 10px; text-align: left; border-bottom: 1px solid var(--color-border-tertiary); background: var(--bg-surface); position: sticky; top: 0; z-index: 2; }
	.check-col { width: 36px; }
	.size-col { width: 100px; }
	.date-col { width: 150px; }
	.perm-col { width: 115px; }
	.action-col { width: 140px; text-align: right; }
</style>
