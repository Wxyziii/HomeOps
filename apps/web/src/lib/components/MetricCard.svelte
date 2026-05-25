<script lang="ts">
	let { metric }: { metric: { label: string; icon: string; value: string; unit: string; sub: string; percent: number; color: string } } = $props();
</script>

<div class="metric-card">
	<div class="metric-label"><i class="ti {metric.icon}" aria-hidden="true"></i> {metric.label}</div>
	<div class="metric-val">{metric.value}<span>{metric.unit}</span></div>
	<div class="bar-bg"><div class="bar-fill" style={`width:${metric.percent}%;background:${metric.color}`}></div></div>
	<div class="metric-sub">{metric.sub}</div>
</div>

<style>
	.metric-card { background: var(--bg-surface); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 12px 14px; min-width: 0; }
	.metric-label { font-size: 11px; color: var(--color-text-secondary); margin-bottom: 6px; display: flex; align-items: center; gap: 5px; }
	.metric-val { font-size: 22px; font-weight: 500; color: var(--color-text-primary); line-height: 1; }
	.metric-val span { font-size: 13px; font-weight: 400; color: var(--color-text-secondary); }
	.metric-sub { font-size: 11px; color: var(--color-text-tertiary); margin-top: 4px; }
	.bar-bg { height: 4px; background: var(--color-background-primary); border-radius: 2px; margin-top: 8px; overflow: hidden; }
	.bar-fill { height: 100%; border-radius: 2px; }
</style>
