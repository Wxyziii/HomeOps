<script lang="ts">
	import ReduxMakerStudio from '$lib/components/redux-maker/ReduxMakerStudio.svelte';
</script>

<svelte:head><title>Redux Maker · HomeOps Panel</title></svelte:head>
<div class="page">
	<ReduxMakerStudio />
</div>

<style>
	.page { height: 100%; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
</style>
