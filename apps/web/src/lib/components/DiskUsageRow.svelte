<script lang="ts">
	let { disk }: { disk: { mount: string; percent: number; color: string } } = $props();
</script>

<div class="sparkline-row">
	<span class="spark-label">{disk.mount}</span>
	<div class="spark-bar-bg"><div class="spark-bar" style={`width:${disk.percent}%;background:${disk.color}`}></div></div>
	<span class="spark-val">{disk.percent}%</span>
</div>

<style>
	.sparkline-row { display: flex; align-items: center; gap: 8px; padding: 7px 0; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.sparkline-row:last-child { border-bottom: none; }
	.spark-label { font-size: 12px; color: var(--color-text-secondary); width: 38px; }
	.spark-bar-bg { flex: 1; height: 6px; background: var(--color-background-secondary); border-radius: 3px; overflow: hidden; }
	.spark-bar { height: 100%; border-radius: 3px; }
	.spark-val { font-size: 12px; font-weight: 500; color: var(--color-text-primary); min-width: 32px; text-align: right; }
</style>
