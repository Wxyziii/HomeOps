<script lang="ts">
	let { status }: { status: string } = $props();
	let tone = $derived(status.split(' ')[0]);
</script>

<span class="status-pill s-{tone}">{status}</span>

<style>
	.status-pill { font-size: 10px; padding: 2px 8px; border-radius: 10px; font-weight: 500; line-height: 1.4; white-space: nowrap; }
	.s-running { background: var(--color-background-info); color: var(--color-text-info); }
	.s-done, .s-ready, .s-ok, .s-verified { background: var(--color-background-success); color: var(--color-text-success); }
	.s-failed, .s-error { background: var(--color-background-danger); color: var(--color-text-danger); }
	.s-queued, .s-later, .s-warn, .s-expiring { background: var(--color-background-warning); color: var(--color-text-warning); }
	.s-offline { background: var(--color-background-tertiary); color: var(--color-text-tertiary); }
</style>
