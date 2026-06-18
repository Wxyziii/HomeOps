<script lang="ts">
	import type { ApplyOutput } from '$lib/redux-maker/bridge';
	import type { RunHistoryEntry } from '$lib/redux-maker/runHistory';

	let {
		bridgeReady = false,
		applyEnabled = false,
		applyReasons = [],
		applyResult = null,
		applyError = null,
		actionMessage = null,
		history = [],
		scannerPath = '',
		copiedRpfPath = '',
		onApply = () => {},
		onLoadRun = (_entry: RunHistoryEntry) => {},
		onClearHistory = () => {},
		onOpenSettings = () => {},
		onOpenCorpus = () => {},
		onRefresh = () => {},
		onCopyScannerPath = () => {},
		onCopyRpfPath = () => {},
		onCopyDevCommand = () => {},
		onCopyRollback = (_cmd: string) => {}
	}: {
		bridgeReady?: boolean;
		applyEnabled?: boolean;
		applyReasons?: string[];
		applyResult?: ApplyOutput | null;
		applyError?: string | null;
		actionMessage?: string | null;
		history?: RunHistoryEntry[];
		scannerPath?: string;
		copiedRpfPath?: string;
		onApply?: () => void;
		onLoadRun?: (entry: RunHistoryEntry) => void;
		onClearHistory?: () => void;
		onOpenSettings?: () => void;
		onOpenCorpus?: () => void;
		onRefresh?: () => void;
		onCopyScannerPath?: () => void;
		onCopyRpfPath?: () => void;
		onCopyDevCommand?: () => void;
		onCopyRollback?: (cmd: string) => void;
	} = $props();
</script>

<div class="action-dock">
	{#if applyResult?.rollbackManifestPath}
		<div class="rollback-block">
			<div class="rollback-head">
				<span>↩ rollback (display-only)</span>
				{#if applyResult.rollbackCommand}
					<button class="icon-btn" type="button" onclick={() => onCopyRollback(applyResult!.rollbackCommand!)} title="Copy rollback command">⧉</button>
				{/if}
			</div>
			<div class="command-preview"><code>{applyResult.rollbackCommand ?? applyResult.rollbackManifestPath}</code></div>
		</div>
	{/if}

	{#if applyResult}
		<div class="apply-grid" style="font-size:11.5px">
			<span>apply</span><span class={applyResult.applied ? 'c-green' : 'c-red'}>{applyResult.status}</span>
			<span>SHA before</span><span class="c-text1 sha">{applyResult.shaBefore.slice(0, 14)}…</span>
			<span>SHA after</span><span class="c-text1 sha">{applyResult.shaAfter ? applyResult.shaAfter.slice(0, 14) + '…' : '—'}</span>
			<span>replace calls</span><span class="c-amber">{applyResult.replaceRpfEntryCallCount}</span>
			<span>forbidden</span><span class={applyResult.forbiddenEndpointCallCount === 0 ? 'c-green' : 'c-red'}>{applyResult.forbiddenEndpointCallCount}</span>
		</div>
	{/if}
	{#if applyError}<div class="prog-sub c-red">{applyError}</div>{/if}

	<button class="btn-inject" type="button" disabled={!applyEnabled} onclick={onApply}
		title={applyEnabled ? 'Open the SHA + confirmation gated apply modal' : applyReasons.join(' · ')}>
		{#if applyResult?.applied}Applied · copied RPF{:else}Review &amp; Apply Plan{/if}
	</button>
	<div class="action-status">
		<span class="act-exe">{applyEnabled ? 'exact confirmation required' : applyReasons[0] ?? 'apply disabled'}</span>
		<span class="act-guard">◈ copied-RPF only</span>
	</div>

	<div class="dock-divider"></div>

	{#if history.length}
		<div class="hist-head"><span>run history ({history.length})</span><button class="link-btn" type="button" onclick={onClearHistory}>clear</button></div>
		<div class="hist-list">
			{#each history.slice(0, 10) as h (h.runId)}
				<button class="hist-row" type="button" onclick={() => onLoadRun(h)} title={h.prompt}>
					<span class="hist-status" class:ok={h.status === 'finished' || h.status === 'applied'} class:bad={h.status === 'failed' || h.status === 'apply_failed'}>{h.status}</span>
					<span class="hist-prompt">{h.prompt.slice(0, 42)}</span>
					{#if h.corpusContextAttached}<span class="c-orange" style="font-size:10px">⛬</span>{/if}
					{#if h.applied}<span class="c-green" style="font-size:9px">applied</span>{/if}
				</button>
			{/each}
		</div>
		<div class="dock-divider"></div>
	{/if}

	<button class="btn-row" type="button" onclick={onOpenSettings}>⚙ Bridge settings</button>
	<button class="btn-row" type="button" onclick={onOpenCorpus}>⌗ Open Redux Corpus</button>
	<button class="btn-row" type="button" onclick={onCopyScannerPath} disabled={!scannerPath} title={scannerPath}>⧉ Copy scanner path</button>
	<button class="btn-row" type="button" onclick={onCopyRpfPath} disabled={!copiedRpfPath} title={copiedRpfPath}>⧉ Copy copied RPF path</button>
	<button class="btn-row" type="button" onclick={onCopyDevCommand}>⌘ Copy local dev command</button>
	<button class="btn-row" type="button" onclick={onRefresh}>↻ Refresh bridge + corpus</button>

	{#if actionMessage}<div class="toast">{actionMessage}</div>{/if}
	<div class="act-exe" style="line-height:1.5">🛡 HomeOps server does not edit RPF. Apply is local desktop only, copied-RPF only, via the scanner's single /api/replace-rpf-entry path. {bridgeReady ? '' : 'Bridge offline.'}</div>
</div>
