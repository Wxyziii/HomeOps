<script lang="ts">
	let { status }: { status: string } = $props();
	let tone = $derived(status.split(' ')[0]);
</script>

<span class="status-pill s-{tone}">{status}</span>

<style>
	.status-pill { font-size: 10.5px; padding: 2px 7px; border-radius: 2px; border: 1px solid var(--color-border-tertiary); font-weight: 600; line-height: 1.35; white-space: nowrap; }
	.s-running { background: var(--color-background-info); color: var(--color-text-info); border-color: var(--color-border-info); }
	.s-done, .s-ready, .s-ok, .s-verified, .s-active, .s-finished { background: var(--color-background-success); color: var(--color-text-success); border-color: var(--color-border-success); }
	.s-failed, .s-error { background: var(--color-background-danger); color: var(--color-text-danger); border-color: var(--color-border-danger); }
	.s-queued, .s-later, .s-warn, .s-expiring, .s-cancelled { background: var(--color-background-warning); color: var(--color-text-warning); border-color: var(--color-border-warning); }
	.s-offline, .s-archived { background: var(--color-background-tertiary); color: var(--color-text-tertiary); border-color: var(--color-border-tertiary); }
</style>
