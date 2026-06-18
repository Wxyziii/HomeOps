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
		standalonePath = '',
		onApply = () => {},
		onLoadRun = (_entry: RunHistoryEntry) => {},
		onClearHistory = () => {},
		onOpenSettings = () => {},
		onOpenCorpus = () => {},
		onRefresh = () => {},
		onCopyPath = () => {},
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
		standalonePath?: string;
		onApply?: () => void;
		onLoadRun?: (entry: RunHistoryEntry) => void;
		onClearHistory?: () => void;
		onOpenSettings?: () => void;
		onOpenCorpus?: () => void;
		onRefresh?: () => void;
		onCopyPath?: () => void;
		onCopyScannerPath?: () => void;
		onCopyRpfPath?: () => void;
		onCopyDevCommand?: () => void;
		onCopyRollback?: (cmd: string) => void;
	} = $props();
</script>

<div class="dock">
	<div class="card-title">Action dock</div>

	<button class="btn primary block" type="button" disabled={!applyEnabled} onclick={onApply}
		title={applyEnabled ? 'Open the SHA + confirmation gated apply modal' : applyReasons.join(' · ')}>
		✓ Review &amp; Apply Plan
	</button>
	{#if !applyEnabled}
		<div class="reason"><span class="lock">⊘</span> Apply disabled — <b>{applyReasons[0] ?? 'not ready'}</b>. Copied-RPF only, SHA + exact-confirmation gated.</div>
	{:else}
		<div class="reason ok">Ready — apply targets ONLY the copied test RPF behind the exact confirmation gate.</div>
	{/if}

	{#if applyError}
		<div class="apply-box err">apply error: {applyError}</div>
	{/if}

	{#if applyResult}
		<div class="apply-box" class:ok={applyResult.applied} class:err={!applyResult.applied}>
			<div class="ab-row">status <b>{applyResult.status}</b></div>
			<div class="ab-row">SHA before <code>{applyResult.shaBefore.slice(0, 12)}…</code></div>
			<div class="ab-row">SHA after <code>{applyResult.shaAfter ? applyResult.shaAfter.slice(0, 12) + '…' : '—'}</code></div>
			<div class="ab-row">replace-rpf-entry calls <b>{applyResult.replaceRpfEntryCallCount}</b></div>
			<div class="ab-row">forbidden endpoint calls <b class:bad={applyResult.forbiddenEndpointCallCount > 0}>{applyResult.forbiddenEndpointCallCount}</b></div>
			{#if applyResult.rollbackManifestPath}
				<div class="ab-row mono">rollback manifest:<br />{applyResult.rollbackManifestPath}</div>
			{/if}
			{#if applyResult.rollbackCommand}
				<button class="btn small" type="button" onclick={() => onCopyRollback(applyResult!.rollbackCommand!)}>
					⧉ Copy rollback command (display-only)
				</button>
			{/if}
		</div>
	{/if}

	<div class="divider"></div>

	{#if history.length}
		<div class="hist-head">
			<span>Run history ({history.length})</span>
			<button class="link-btn" type="button" onclick={onClearHistory}>clear</button>
		</div>
		<div class="hist-list">
			{#each history.slice(0, 12) as h (h.runId)}
				<button class="hist-row" type="button" onclick={() => onLoadRun(h)} title={h.prompt}>
					<span class="hist-status hs-{h.status}">{h.status}</span>
					<span class="hist-prompt">{h.prompt.slice(0, 48)}</span>
					{#if h.corpusContextAttached}<span class="hist-ctx" title="corpus context attached">⛬</span>{/if}
					{#if h.applied}<span class="hist-applied">applied</span>{/if}
				</button>
			{/each}
		</div>
		<div class="divider"></div>
	{/if}

	<button class="btn block" type="button" onclick={onOpenSettings}>⚙ Bridge settings</button>
	<button class="btn block" type="button" onclick={onOpenCorpus}>⌗ Open Redux Corpus</button>
	<button class="btn block" type="button" onclick={onCopyScannerPath} disabled={!scannerPath} title={scannerPath}>⧉ Copy scanner path</button>
	<button class="btn block" type="button" onclick={onCopyRpfPath} disabled={!copiedRpfPath} title={copiedRpfPath}>⧉ Copy copied RPF path</button>
	<button class="btn block" type="button" onclick={onCopyPath}>⧉ Copy Redux Maker path</button>
	<button class="btn block" type="button" onclick={onCopyDevCommand}>⌘ Copy local dev command</button>
	<button class="btn block" type="button" onclick={onRefresh}>↻ Refresh bridge + corpus</button>

	<div class="path" title={standalonePath}>{standalonePath}</div>

	{#if actionMessage}<div class="toast">{actionMessage}</div>{/if}

	<div class="safety">🛡 HomeOps server does not edit RPF files. Apply is local desktop only, copied-RPF only, via the scanner's single /api/replace-rpf-entry path. {bridgeReady ? '' : 'Bridge offline.'}</div>
</div>

<style>
	.dock { padding: 10px 12px; display: flex; flex-direction: column; gap: 7px; background: var(--bg-sidebar); min-height: 0; }
	.card-title { color: var(--color-text-primary); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.btn { padding: 7px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; text-align: left; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn.block { width: 100%; }
	.btn.small { font-size: 10.5px; padding: 4px 8px; margin-top: 4px; }
	.btn.primary { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.reason { color: var(--color-text-tertiary); font-size: 10.5px; line-height: 1.5; }
	.reason.ok { color: var(--color-text-success); }
	.reason b { color: var(--color-text-warning); }
	.lock { color: var(--color-text-warning); }
	.apply-box { border: 0.5px solid var(--color-border-tertiary); border-radius: 4px; padding: 7px 9px; font-size: 11px; display: flex; flex-direction: column; gap: 3px; }
	.apply-box.ok { border-color: var(--color-border-success); }
	.apply-box.err { border-color: var(--color-border-danger, var(--red)); }
	.ab-row { color: var(--color-text-tertiary); }
	.ab-row b { color: var(--color-text-secondary); }
	.ab-row b.bad { color: var(--color-text-danger); }
	.ab-row.mono { font-family: var(--font-mono); font-size: 9.5px; overflow-wrap: anywhere; }
	code { font-family: var(--font-mono); color: var(--accent); }
	.divider { height: 0.5px; background: var(--color-border-tertiary); margin: 4px 0; }
	.hist-head { display: flex; justify-content: space-between; align-items: center; color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; }
	.link-btn { background: none; border: none; color: var(--accent); font-size: 10px; cursor: pointer; }
	.hist-list { display: flex; flex-direction: column; gap: 3px; max-height: 170px; overflow: auto; }
	.hist-row { display: flex; align-items: center; gap: 6px; text-align: left; border: 0.5px solid var(--color-border-tertiary); border-radius: 4px; background: var(--bg-surface); padding: 4px 6px; cursor: pointer; }
	.hist-row:hover { border-color: var(--accent); }
	.hist-status { font-size: 9px; font-family: var(--font-mono); padding: 1px 4px; border-radius: 2px; color: var(--color-text-tertiary); flex: none; }
	.hist-status.hs-finished, .hist-status.hs-applied { color: var(--color-text-success); }
	.hist-status.hs-failed, .hist-status.hs-apply_failed, .hist-status.hs-timed_out { color: var(--color-text-danger); }
	.hist-prompt { font-size: 10.5px; color: var(--color-text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0; }
	.hist-ctx { color: var(--accent); font-size: 10px; flex: none; }
	.hist-applied { color: var(--color-text-success); font-size: 9px; flex: none; }
	.path { color: var(--text-faint); font-family: var(--font-mono); font-size: 10px; overflow-wrap: anywhere; }
	.toast { color: var(--color-text-success); font-size: 11px; }
	.safety { margin-top: 4px; color: var(--color-text-success); font-size: 10.5px; line-height: 1.5; border-top: 0.5px solid var(--color-border-tertiary); padding-top: 8px; }
</style>
