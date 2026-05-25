<script lang="ts">
	let { log }: { log: { time: string; level: string; kind: string; message: string } } = $props();
</script>

<div class="log-row">
	<span class="log-time">{log.time}</span>
	<span class="log-level l-{log.kind}">{log.level}</span>
	<span class="log-msg">{log.message}</span>
</div>

<style>
	.log-row { display: flex; align-items: flex-start; gap: 8px; padding: 6px 0; border-bottom: 0.5px solid var(--color-border-tertiary); font-size: 11px; font-family: var(--font-mono); }
	.log-row:last-child { border-bottom: none; }
	.log-time { color: var(--color-text-tertiary); flex-shrink: 0; min-width: 70px; }
	.log-level { min-width: 34px; font-weight: 500; }
	.log-msg { color: var(--color-text-secondary); line-height: 1.5; }
	.l-info { color: var(--color-text-info); }
	.l-warn { color: var(--color-text-warning); }
	.l-error { color: var(--color-text-danger); }
	.l-ok { color: var(--color-text-success); }
</style>
