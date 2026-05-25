<script lang="ts">
	import { page } from '$app/state';
	let { href, icon, label, badge = '', badgeTone = '' }: { href: string; icon: string; label: string; badge?: string; badgeTone?: string } = $props();
	let active = $derived(href === '/' ? page.url.pathname === '/' : page.url.pathname === href || page.url.pathname.startsWith(`${href}/`));
</script>

<a class:active class="nav-item" {href}>
	<i class="ti {icon}" aria-hidden="true"></i>
	<span>{label}</span>
	{#if badge}<span class="nav-badge {badgeTone}">{badge}</span>{/if}
</a>

<style>
	.nav-item { display: flex; align-items: center; gap: 10px; padding: 8px 16px; font-size: 13px; color: var(--color-text-primary); cursor: pointer; margin: 1px 8px; border-radius: 6px; min-height: 32px; }
	.nav-item:hover { background: var(--bg-surface-2); color: var(--color-text-primary); }
	.nav-item.active { background: var(--bg-surface); color: var(--color-text-primary); font-weight: 600; }
	.nav-item.active i { color: var(--accent); }
	.nav-item i { font-size: 16px; width: 16px; text-align: center; }
	.nav-item span:first-of-type { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.nav-badge { margin-left: auto; background: var(--color-background-danger); color: var(--color-text-danger); font-size: 10px; font-weight: 500; padding: 2px 6px; border-radius: 10px; }
	.nav-badge.ok { background: var(--color-background-success); color: var(--color-text-success); }
</style>
