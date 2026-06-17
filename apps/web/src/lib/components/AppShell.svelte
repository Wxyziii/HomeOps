<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import Sidebar from './Sidebar.svelte';
	let { children }: { children: Snippet } = $props();

	const navItems = [
		{ href: '/', label: 'Dashboard' },
		{ href: '/files', label: 'Files' },
		{ href: '/projects', label: 'Projects' },
		{ href: '/resources', label: 'Resources' }
	];

	const active = (href: string) => href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
</script>

<div class="shell-wrap">
	<header class="global-topbar">
		<div class="topbar-left">
			<div class="brand-icon">HO</div>
			<div class="brand-copy">
				<div class="brand-name">HomeOps Panel</div>
				<div class="brand-sub">v0.1 · server-agent</div>
			</div>
			<div class="topbar-sep"></div>
			<div class="endpoint"><i class="ti ti-home" aria-hidden="true"></i><span>marcel@100.68.7.42:8787</span></div>
		</div>
		<nav class="topbar-nav" aria-label="Primary quick navigation">
			{#each navItems as item}
				<a class:active={active(item.href)} href={item.href}>{item.label}</a>
			{/each}
			<div class="guard"><span></span>TAILSCALE PRIVATE · ACTIVE</div>
		</nav>
	</header>
	<div class="layout">
		<Sidebar />
		<main class="main">{@render children()}</main>
	</div>
</div>

<style>
	.shell-wrap { width: 100vw; height: 100vh; display: flex; flex-direction: column; overflow: hidden; background: var(--bg-app); }
	.global-topbar { flex: none; height: 42px; display: flex; align-items: center; justify-content: space-between; background: var(--bg-input); border-bottom: 1px solid var(--color-border-tertiary); overflow: hidden; }
	.topbar-left { height: 100%; display: flex; align-items: center; gap: 10px; padding: 0 14px; border-right: 1px solid var(--color-border-tertiary); min-width: 0; }
	.brand-icon { width: 24px; height: 24px; border-radius: 4px; display: flex; align-items: center; justify-content: center; background: var(--accent); color: #fff; font-size: 12px; font-weight: 700; line-height: 1; flex: none; }
	.brand-copy { min-width: 0; display: flex; align-items: baseline; gap: 10px; }
	.brand-name { color: var(--color-text-primary); font-size: 14px; font-weight: 700; white-space: nowrap; }
	.brand-sub { color: var(--color-text-tertiary); font-size: 11px; white-space: nowrap; }
	.topbar-sep { width: 1px; height: 100%; background: var(--color-border-tertiary); flex: none; }
	.endpoint { display: flex; align-items: center; gap: 6px; min-width: 0; color: var(--color-text-tertiary); font-size: 12px; }
	.endpoint span { color: var(--color-text-warning); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.topbar-nav { margin-left: auto; height: 100%; display: flex; align-items: center; }
	.topbar-nav a, .guard { height: 100%; display: flex; align-items: center; padding: 0 12px; border-left: 1px solid var(--color-border-tertiary); color: var(--color-text-tertiary); font-size: 12px; white-space: nowrap; }
	.topbar-nav a:hover, .topbar-nav a.active { color: var(--color-text-primary); background: var(--bg-hover); }
	.guard { gap: 6px; color: var(--color-text-success); font-size: 10.5px; letter-spacing: 0.04em; }
	.guard span { width: 6px; height: 6px; border-radius: 50%; background: var(--color-text-success); box-shadow: 0 0 0 0 rgba(90, 158, 111, 0.45); animation: pulse 2.4s ease-in-out infinite; }
	.layout { display: flex; flex: 1; min-height: 0; overflow: hidden; background: var(--bg-surface); }
	.main { flex: 1; min-width: 0; overflow: auto; background: var(--bg-app); }
	@keyframes pulse { 50% { opacity: 0.65; box-shadow: 0 0 0 4px rgba(90, 158, 111, 0); } }
	@media (max-width: 980px) {
		.endpoint, .brand-sub { display: none; }
		.topbar-nav a { padding: 0 9px; }
		.guard { display: none; }
	}
	/* Small desktop windows: drop the quick-nav (the sidebar rail covers nav)
	   and keep the brand + status reachable without overflow. */
	@media (max-width: 760px) {
		.topbar-nav a { display: none; }
		.topbar-left { padding: 0 10px; gap: 8px; }
	}
</style>
