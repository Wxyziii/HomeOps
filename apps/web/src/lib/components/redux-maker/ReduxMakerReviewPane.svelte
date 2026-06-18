<script lang="ts">
	import type { RunStatus } from '$lib/redux-maker/bridge';

	let {
		runStatus = null,
		running = false
	}: { runStatus?: RunStatus | null; running?: boolean } = $props();

	const report = $derived((runStatus?.report ?? null) as Record<string, unknown> | null);
	const s = (k: string) => (report ? String(report[k] ?? '—') : '—');
	const num = (k: string) => (report ? Number((report[k] as number) ?? 0) : 0);
	const badgeLabel = $derived(
		running ? 'RUNNING' : !report ? 'NO RUN' : 'RUN REPORT'
	);
	const sourceLabel = $derived(runStatus ? runStatus.runId : 'no run loaded');
	const added = $derived(num('replacementPlanCount'));
	const removed = $derived(num('blockedChildCount'));
	const logText = $derived(
		runStatus ? [runStatus.stdoutTail, runStatus.stderrTail].filter(Boolean).join('\n') : ''
	);
</script>

<div class="context-ribbon">
	<div class="crumb">
		<span style="color:var(--text-3)">Redux Maker</span>
		<span class="sep">/</span>
		<span style="color:var(--text-2)">Runs</span>
		<span class="sep">/</span>
		<span class="active-file">{sourceLabel}</span>
		<span class="ribbon-badge">● {badgeLabel}{report ? ` · ${s('status')}` : ''}</span>
	</div>
	<div class="crumb-right">
		<div class="crumb-stat red">blocked {removed}</div>
		<div class="crumb-stat green">plans {added}</div>
	</div>
</div>

<div class="diff-area">
	{#if !report}
		<div class="empty-center">
			<div class="empty-center-title">No run loaded</div>
			<div class="empty-center-sub">Enter a prompt or pick a preset, then Generate Module Plan in the desktop bridge. Run logs and the report appear here.</div>
		</div>
	{:else}
		<div class="diff-pane">
			<div class="diff-pane-header">
				<span class="dot dot-red"></span>
				<span style="color:var(--red);font-weight:600;font-size:10px;letter-spacing:.04em">RUN — {runStatus?.phase}</span>
				<span class="sha">{s('provider')} · {s('mode')}</span>
			</div>
			<div class="diff-scroll" style="padding:8px 12px;white-space:pre-wrap;line-height:1.6;font-size:12px;color:var(--text-2);overflow-wrap:anywhere">{logText || '(no log output yet)'}</div>
		</div>
		<div class="diff-pane">
			<div class="diff-pane-header">
				<span class="dot dot-green"></span>
				<span style="color:var(--green);font-weight:600;font-size:10px;letter-spacing:.04em">REVIEW — safe plan preview</span>
				<span class="sha">read-only · no POST</span>
			</div>
			<div class="diff-scroll">
				<div class="diff-line neutral"><span class="diff-num">1</span><span class="diff-sign"> </span><span class="diff-code"><span class="tok-com"># Safety result</span></span></div>
				<div class="diff-line added"><span class="diff-num">2</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">moduleSafe</span>       <span class="tok-new">{s('moduleSafe')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">3</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">readyToApply</span>     <span class="tok-new">{s('readyToApply')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">4</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">applied</span>          <span class="tok-new">{s('applied')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">5</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">rollbackReady</span>    <span class="tok-new">{s('rollbackReady')}</span></span></div>
				<div class="diff-line neutral"><span class="diff-num">6</span><span class="diff-sign"> </span><span class="diff-code"> </span></div>
				<div class="diff-line context"><span class="diff-num">7</span><span class="diff-sign"> </span><span class="diff-code"><span class="tok-com"># copied-RPF SHA gate</span></span></div>
				<div class="diff-line context"><span class="diff-num">8</span><span class="diff-sign"> </span><span class="diff-code"><span class="tok-key">expected</span>         <span class="tok-val">{s('expectedTargetRpfSha256')}</span></span></div>
				<div class="diff-line context"><span class="diff-num">9</span><span class="diff-sign"> </span><span class="diff-code"><span class="tok-key">target</span>           <span class="tok-val">{s('targetRpf')}</span></span></div>
				<div class="diff-line neutral"><span class="diff-num">10</span><span class="diff-sign"> </span><span class="diff-code"> </span></div>
				<div class="diff-line added"><span class="diff-num">11</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">stagedChildCount</span> <span class="tok-new">{num('stagedChildCount')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">12</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">blockedChildCount</span> <span class="tok-new">{num('blockedChildCount')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">13</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">generatedAssets</span>  <span class="tok-new">{num('generatedAssetCount')}</span></span></div>
				<div class="diff-line added"><span class="diff-num">14</span><span class="diff-sign">+</span><span class="diff-code"><span class="tok-key">replacementPlans</span> <span class="tok-new">{num('replacementPlanCount')}</span></span></div>
			</div>
		</div>
	{/if}
</div>
