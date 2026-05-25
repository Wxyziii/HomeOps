<script lang="ts">
	import FileRow from './FileRow.svelte';
	let { files }: { files: Array<{ type: string; name: string; size: string; modified: string; permissions: string }> } = $props();
</script>

<div class="file-table">
	<table>
		<thead>
			<tr>
				<th class="check-col"><input type="checkbox" aria-label="Select all" /></th>
				<th>Name</th>
				<th class="size-col">Size</th>
				<th class="date-col">Modified</th>
				<th class="perm-col">Permissions</th>
				<th class="action-col"></th>
			</tr>
		</thead>
		<tbody>{#each files as file}<FileRow {file} />{/each}</tbody>
	</table>
</div>

<style>
	.file-table { flex: 1; overflow: auto; background: var(--bg-surface); }
	table { width: 100%; border-collapse: collapse; table-layout: fixed; }
	th { font-size: 11px; font-weight: 600; color: var(--color-text-secondary); padding: 8px 12px; text-align: left; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-app); position: sticky; top: 0; }
	.check-col { width: 40px; }
	.size-col { width: 80px; }
	.date-col { width: 130px; }
	.perm-col { width: 90px; }
	.action-col { width: 70px; text-align: right; }
</style>
