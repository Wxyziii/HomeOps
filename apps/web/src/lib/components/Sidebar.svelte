<script lang="ts">
	import SidebarItem from './SidebarItem.svelte';
</script>

<aside class="sidebar">
	<div class="logo">
		<div class="logo-icon"><i class="ti ti-server" aria-hidden="true"></i></div>
		<div>
			<div class="logo-text">HomeOps</div>
			<div class="logo-sub">panel v0.1</div>
		</div>
	</div>
	<div class="nav-section">Main</div>
	<SidebarItem href="/" icon="ti-layout-dashboard" label="Dashboard" />
	<SidebarItem href="/files" icon="ti-folder" label="Files" />
	<SidebarItem href="/archives" icon="ti-archive" label="Archives" />
	<SidebarItem href="/projects" icon="ti-code" label="Projects" />
	<SidebarItem href="/jobs" icon="ti-player-play" label="Jobs" />
	<div class="nav-section">System</div>
	<SidebarItem href="/logs" icon="ti-file-text" label="Logs" />
	<SidebarItem href="/resources" icon="ti-cpu" label="Resources" />
	<SidebarItem href="/services" icon="ti-plug-connected" label="Services" />
	<SidebarItem href="/apps" icon="ti-apps" label="Apps" />
	<SidebarItem href="/settings" icon="ti-settings" label="Settings" />
	<div class="sidebar-footer"><div class="footer-user"><i class="ti ti-user-circle" aria-hidden="true"></i> marcel</div></div>
</aside>

<style>
	.sidebar { width: 200px; min-width: 200px; background: var(--bg-sidebar); border-right: 0.5px solid var(--color-border-tertiary); display: flex; flex-direction: column; padding: 16px 0; }
	.logo { padding: 0 16px 20px; display: flex; align-items: center; gap: 10px; border-bottom: 0.5px solid var(--color-border-tertiary); margin-bottom: 8px; }
	.logo-icon { width: 32px; height: 32px; background: var(--accent); border-radius: var(--border-radius-md); display: flex; align-items: center; justify-content: center; color: #e6f1fb; font-size: 16px; }
	.logo-text { font-size: 14px; font-weight: 500; color: var(--color-text-primary); }
	.logo-sub { font-size: 11px; color: var(--color-text-secondary); }
	.nav-section { font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-text-tertiary); padding: 12px 24px 4px; }
	.sidebar-footer { margin-top: auto; padding: 12px 8px 0; border-top: 0.5px solid var(--color-border-tertiary); }
	.footer-user { display: flex; align-items: center; gap: 10px; padding: 8px 16px; font-size: 13px; color: var(--color-text-primary); }
	.footer-user i { color: #d7c88f; }
</style>
