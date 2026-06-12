<script lang="ts">
	import { page } from '$app/state';

	let { children } = $props();

	const tabs = [
		{ href: '/minecraft', icon: 'ti-layout-dashboard', label: 'Overview' },
		{ href: '/minecraft/servers', icon: 'ti-server', label: 'Server' },
		{ href: '/minecraft/console', icon: 'ti-terminal-2', label: 'Console' },
		{ href: '/minecraft/files', icon: 'ti-folder', label: 'Files' },
		{ href: '/minecraft/config', icon: 'ti-adjustments', label: 'Config' },
		{ href: '/minecraft/mods', icon: 'ti-puzzle', label: 'Mods' },
		{ href: '/minecraft/modpacks', icon: 'ti-packages', label: 'Modpacks' },
		{ href: '/minecraft/worlds', icon: 'ti-world', label: 'Worlds' },
		{ href: '/minecraft/players', icon: 'ti-users', label: 'Players' },
		{ href: '/minecraft/backups', icon: 'ti-database-export', label: 'Backups' },
		{ href: '/minecraft/settings', icon: 'ti-settings', label: 'Settings' }
	];

	function isActive(href: string): boolean {
		if (href === '/minecraft') return page.url.pathname === '/minecraft';
		return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
	}
</script>

<div class="minecraft-module">
	<nav class="subnav" aria-label="Minecraft module navigation">
		{#each tabs as tab}
			<a class:active={isActive(tab.href)} class="subnav-item" href={tab.href}>
				<i class="ti {tab.icon}" aria-hidden="true"></i>
				<span>{tab.label}</span>
			</a>
		{/each}
	</nav>
	{@render children()}
</div>

<style>
	.minecraft-module { display: flex; flex-direction: column; min-height: 100%; }
	.subnav { display: flex; align-items: center; gap: 2px; padding: 8px 20px 0; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-surface); overflow-x: auto; }
	.subnav-item { display: flex; align-items: center; gap: 6px; padding: 8px 12px; font-size: 12px; color: var(--color-text-secondary); border-bottom: 2px solid transparent; white-space: nowrap; }
	.subnav-item:hover { color: var(--color-text-primary); }
	.subnav-item.active { color: var(--color-text-primary); font-weight: 600; border-bottom-color: var(--accent); }
	.subnav-item.active i { color: var(--accent); }
	.subnav-item i { font-size: 14px; }
</style>
