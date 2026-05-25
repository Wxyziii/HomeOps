<script lang="ts">
	import type { Snippet } from 'svelte';
	import Sidebar from './Sidebar.svelte';
	let { children }: { children: Snippet } = $props();
</script>

<div class="shell-wrap">
	<div class="layout">
		<Sidebar />
		<main class="main">{@render children()}</main>
	</div>
</div>

<style>
	.shell-wrap { min-height: 100vh; padding: 8px 6px; background: var(--bg-app); }
	.layout { display: flex; height: calc(100vh - 16px); min-height: 640px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; background: var(--bg-surface); }
	.main { flex: 1; min-width: 0; overflow: auto; }
</style>
