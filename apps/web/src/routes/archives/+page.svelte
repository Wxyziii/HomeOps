<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Topbar from '$lib/components/Topbar.svelte';

	onMount(() => {
		void goto('/files?filter=archives', { replaceState: true });
	});
</script>

<svelte:head><title>Archives · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Archives" flush />
	<div class="notice">Archives now live inside Files. Redirecting to the archive filter...</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; background: var(--bg-surface); }
	.notice { margin: 20px; padding: 10px 12px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; background: var(--bg-app); }
</style>
