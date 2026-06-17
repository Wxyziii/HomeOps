<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import AppShell from '$lib/components/AppShell.svelte';
	import { applyTheme, getStoredThemeId } from '$lib/theme/themes';

	let { children } = $props();

	// Tag the document root with the active theme. The default theme's tokens
	// live on :root, so this does not flash on load.
	$effect(() => {
		applyTheme(getStoredThemeId());
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<AppShell>{@render children()}</AppShell>
