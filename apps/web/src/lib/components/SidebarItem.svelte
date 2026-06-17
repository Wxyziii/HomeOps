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
	.nav-item { display: flex; align-items: center; gap: 7px; padding: 5px 10px 5px 18px; min-height: 30px; border-left: 2px solid transparent; color: var(--color-text-secondary); font-size: 13px; cursor: pointer; transition: background 0.1s, color 0.1s, border-color 0.1s; }
	.nav-item:hover { background: var(--bg-hover); color: var(--color-text-primary); }
	.nav-item.active { background: var(--orange-bg); border-left-color: var(--accent); color: var(--accent); font-weight: 600; }
	.nav-item.active i { color: var(--accent); }
	.nav-item i { font-size: 14px; width: 16px; color: var(--color-text-tertiary); text-align: center; }
	.nav-item span:first-of-type { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.nav-badge { margin-left: auto; background: var(--color-background-danger); color: var(--color-text-danger); border: 1px solid var(--color-border-danger); font-size: 10px; font-weight: 500; padding: 1px 6px; border-radius: 2px; }
	.nav-badge.ok { background: var(--color-background-success); color: var(--color-text-success); }
</style>
