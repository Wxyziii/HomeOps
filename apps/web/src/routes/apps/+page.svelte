<script lang="ts">
	import Topbar from '$lib/components/Topbar.svelte';
	import ModuleCard from '$lib/components/ModuleCard.svelte';
	import { modules } from '$lib/data/mock';
</script>

<svelte:head><title>Apps · HomeOps Panel</title></svelte:head>
<div class="page-pad">
	<Topbar title="Apps" />
	<div class="module-grid">{#each modules as module}<ModuleCard {module} />{/each}</div>
</div>

<style>
	.page-pad { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
	.module-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
</style>
