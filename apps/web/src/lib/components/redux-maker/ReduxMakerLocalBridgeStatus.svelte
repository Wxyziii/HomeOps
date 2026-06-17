<script lang="ts">
	let {
		bridgeConnected = false,
		corpusConnected = false,
		corpusLoading = false,
		corpusUnavailable = false,
		latestScan = null
	}: {
		bridgeConnected?: boolean;
		corpusConnected?: boolean;
		corpusLoading?: boolean;
		corpusUnavailable?: boolean;
		latestScan?: string | null;
	} = $props();

	const corpusLabel = $derived(
		corpusLoading
			? 'checking…'
			: corpusUnavailable
				? 'unavailable'
				: corpusConnected
					? 'scanner ready'
					: 'scanner missing'
	);
</script>

<div class="ribbon">
	<div class="title">
		<span class="dot"></span>
		<strong>Redux Maker Studio</strong>
		<span class="sub">embedded workspace</span>
	</div>
	<div class="chips">
		<span class="chip warn" title="The safe local bridge lands in H2.1">
			local bridge: <b>{bridgeConnected ? 'connected' : 'not connected'}</b>
		</span>
		<span class="chip" class:ok={corpusConnected} class:err={corpusUnavailable}>
			corpus server: <b>{corpusLabel}</b>
		</span>
		{#if latestScan}<span class="chip">latest scan: <b>{latestScan}</b></span>{/if}
		<span class="chip safe" title="HomeOps server never edits RPF files">
			safety: <b>server does not edit RPF</b>
		</span>
	</div>
</div>

<style>
	.ribbon { flex: none; display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; padding: 8px 14px; background: var(--bg-input); border-bottom: 1px solid var(--color-border-tertiary); }
	.title { display: flex; align-items: center; gap: 8px; min-width: 0; }
	.title strong { color: var(--color-text-primary); font-size: 13px; }
	.title .sub { color: var(--color-text-tertiary); font-size: 11px; }
	.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); flex: none; }
	.chips { display: flex; flex-wrap: wrap; gap: 6px; }
	.chip { font-size: 11px; color: var(--color-text-tertiary); border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; padding: 2px 9px; white-space: nowrap; }
	.chip b { color: var(--color-text-secondary); font-weight: 600; }
	.chip.ok b { color: var(--color-text-success); }
	.chip.err b { color: var(--color-text-danger); }
	.chip.warn b { color: var(--color-text-warning); }
	.chip.safe { border-color: var(--color-border-success); }
	.chip.safe b { color: var(--color-text-success); }
	@media (max-width: 760px) { .title .sub { display: none; } }
</style>
