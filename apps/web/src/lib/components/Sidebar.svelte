<script lang="ts">
	import SidebarItem from './SidebarItem.svelte';
</script>

<aside class="sidebar">
	<div class="logo">
		<span>HomeOps Console</span>
		<div class="logo-actions"><i class="ti ti-refresh" aria-hidden="true"></i><i class="ti ti-layout-sidebar-left-collapse" aria-hidden="true"></i></div>
	</div>
	<div class="nav-section">Main</div>
	<SidebarItem href="/" icon="ti-layout-dashboard" label="Dashboard" />
	<SidebarItem href="/files" icon="ti-folder" label="Files" />
	<SidebarItem href="/projects" icon="ti-code" label="Projects" />
	<SidebarItem href="/operations" icon="ti-activity" label="Operations" />
	<SidebarItem href="/resources" icon="ti-cpu" label="Resources" />
	<div class="nav-section">Modules</div>
	<SidebarItem href="/minecraft" icon="ti-cube" label="Minecraft" />
	<SidebarItem href="/redux-maker" icon="ti-wand" label="Redux Maker" />
	<SidebarItem href="/redux-corpus" icon="ti-package" label="Redux Corpus" />
	<SidebarItem href="/redux-archive" icon="ti-library" label="Redux Archive" />
	<div class="nav-section">System</div>
	<SidebarItem href="/settings" icon="ti-settings" label="Settings" />
	<div class="storage-dock">
		<div class="storage-label"><span>workspace disk</span><span>safe</span></div>
		<div class="storage-bar"><div></div></div>
		<div class="storage-grid">
			<div><span>roots</span><strong>2</strong></div>
			<div><span>jobs</span><strong>live</strong></div>
			<div><span>delete</span><strong class="danger">off</strong></div>
		</div>
	</div>
</aside>

<style>
	.sidebar { width: 250px; min-width: 250px; background: var(--bg-input); border-right: 1px solid var(--color-border-tertiary); display: flex; flex-direction: column; overflow: hidden; }
	.logo { flex: none; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 10px 6px; border-bottom: 1px solid var(--color-border-tertiary); color: var(--color-text-tertiary); font-size: 11.5px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	.logo-actions { display: flex; gap: 6px; color: var(--text-faint); font-size: 13px; }
	.nav-section { font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-faint); padding: 12px 10px 3px; display: flex; align-items: center; gap: 4px; }
	.nav-section::before { content: '▾'; font-size: 9px; color: var(--color-text-tertiary); }
	.storage-dock { margin-top: auto; padding: 12px; border-top: 1px solid var(--color-border-tertiary); }
	.storage-label { display: flex; justify-content: space-between; font-size: 11.5px; color: var(--color-text-tertiary); margin-bottom: 5px; }
	.storage-label span:last-child { color: var(--accent); }
	.storage-bar { height: 3px; background: var(--color-border-tertiary); border-radius: 2px; overflow: hidden; margin-bottom: 7px; }
	.storage-bar div { width: 13%; height: 100%; background: var(--accent); }
	.storage-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; }
	.storage-grid div { background: var(--bg-surface); border: 1px solid var(--color-border-tertiary); border-radius: 3px; padding: 7px 8px; min-width: 0; }
	.storage-grid span { display: block; color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; }
	.storage-grid strong { display: block; margin-top: 2px; color: var(--color-text-secondary); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.storage-grid .danger { color: var(--color-text-danger); }
	/* Medium windows: narrow the sidebar but keep labels. */
	@media (max-width: 1100px) { .sidebar { width: 200px; min-width: 200px; } }
	/* Small desktop windows: collapse to a compact icon rail. */
	@media (max-width: 880px) {
		.sidebar { width: 52px; min-width: 52px; }
		.logo span, .nav-section, .logo-actions, .storage-dock { display: none; }
		.logo { justify-content: center; padding: 8px 0 6px; }
		.sidebar :global(.nav-item) { padding: 5px 0; justify-content: center; border-left-width: 0; }
		.sidebar :global(.nav-item.active) { border-left-width: 2px; }
		.sidebar :global(.nav-item span) { display: none; }
		.sidebar :global(.nav-item i) { width: auto; }
	}
</style>
