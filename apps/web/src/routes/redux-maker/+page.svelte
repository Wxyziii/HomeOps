<script lang="ts">
	import ReduxMakerPrototype from '$lib/components/redux-maker/ReduxMakerPrototype.svelte';
</script>

<svelte:head><title>Redux Maker · HomeOps Panel</title></svelte:head>
<div class="page">
	<ReduxMakerPrototype />
</div>

<style>
	.page { height: 100%; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
</style>
