<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import FileToolbar from '$lib/components/FileToolbar.svelte';
	import FileTable from '$lib/components/FileTable.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import { files } from '$lib/data/mock';
</script>

<svelte:head><title>Files · HomeOps Panel</title></svelte:head>
<h2 class="sr-only">Files page showing directory listing with file names, sizes, dates, and actions</h2>
<div class="files-page">
	<Topbar title="Files" flush>
		<SearchInput placeholder="Search files…" />
		<SmallButton icon="ti-upload" label="Upload" />
		<SmallButton icon="ti-folder-plus" label="New folder" />
	</Topbar>
	<FileToolbar />
	<FileTable {files} />
	<StatusBar items={['8 items', '1.4 TB free of 4 TB', '0 selected']} />
</div>

<style>
	.files-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
</style>
