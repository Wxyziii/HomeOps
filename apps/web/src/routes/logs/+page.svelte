<script lang="ts">
	import SearchInput from '$lib/components/SearchInput.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import { logLines } from '$lib/data/mock';
</script>

<svelte:head><title>Logs · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Logs" flush>
		<SearchInput placeholder="Filter logs... (e.g. error, db-snapshot)" />
		<SmallButton icon="ti-download" label="Export" />
		<SmallButton icon="ti-player-pause" label="Pause" />
	</Topbar>
	<div class="filterbar">
		<span class="active">All</span><span>ERR</span><span>WARN</span><span>OK</span><span>INFO</span><span>DEBUG</span>
		<div class="spacer"></div>
		<select><option>All sources</option></select>
		<select><option>Last 1h</option></select>
	</div>
	<div class="log-pane">
		{#each logLines as log}
			<div class="log-row {log.kind}">
				<span class="line">{log.line}</span>
				<span class="time">{log.time}</span>
				<span class="source">[{log.source}]</span>
				<span class="level">{log.level}</span>
				<span class="message">{log.message}</span>
			</div>
		{/each}
	</div>
	<div class="statusbar"><span><i></i>Live</span><span>12 lines shown</span><span>1 error · 1 warning</span><span>Source: all</span></div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.filterbar { display: flex; align-items: center; gap: 8px; padding: 8px 20px; border-bottom: 0.5px solid var(--color-border-tertiary); background: var(--bg-sidebar); }
	.filterbar span { font-size: 11px; padding: 3px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: 20px; color: var(--color-text-secondary); }
	.filterbar .active { background: var(--bg-app); color: var(--color-text-primary); }
	.spacer { flex: 1; }
	select { font-size: 12px; padding: 4px 8px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-secondary); }
	.log-pane { flex: 1; overflow: auto; background: var(--bg-surface); font-family: var(--font-mono); font-size: 12px; padding: 8px 0; }
	.log-row { display: flex; align-items: flex-start; gap: 0; padding: 4px 20px; line-height: 1.6; border-left: 3px solid transparent; }
	.log-row:hover { background: var(--bg-app); }
	.log-row.err { border-left-color: var(--danger); background: color-mix(in srgb, var(--danger) 8%, transparent); }
	.log-row.warn { border-left-color: var(--warning); }
	.log-row.ok { border-left-color: var(--success); }
	.line { color: var(--color-text-tertiary); min-width: 36px; user-select: none; font-size: 11px; }
	.time { color: var(--color-text-tertiary); min-width: 96px; }
	.source { min-width: 128px; font-weight: 600; }
	.level { min-width: 42px; font-weight: 600; }
	.err .source, .err .level { color: var(--color-text-danger); }
	.warn .source, .warn .level { color: var(--color-text-warning); }
	.ok .source, .ok .level { color: var(--color-text-success); }
	.info .source, .info .level { color: var(--color-text-info); }
	.dbg .source, .dbg .level { color: var(--color-text-tertiary); }
	.message { color: var(--color-text-secondary); flex: 1; }
	.statusbar { padding: 6px 20px; border-top: 0.5px solid var(--color-border-tertiary); font-size: 11px; color: var(--color-text-tertiary); display: flex; gap: 16px; background: var(--bg-sidebar); }
	.statusbar i { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: var(--success); margin-right: 4px; vertical-align: middle; }
</style>
