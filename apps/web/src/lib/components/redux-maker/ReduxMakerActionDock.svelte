<script lang="ts">
	let {
		bridgeConnected = false,
		actionMessage = null,
		standalonePath = '',
		onOpenCorpus = () => {},
		onRefresh = () => {},
		onCopyPath = () => {},
		onCopyDevCommand = () => {}
	}: {
		bridgeConnected?: boolean;
		actionMessage?: string | null;
		standalonePath?: string;
		onOpenCorpus?: () => void;
		onRefresh?: () => void;
		onCopyPath?: () => void;
		onCopyDevCommand?: () => void;
	} = $props();

	const applyReason = $derived(
		bridgeConnected ? '' : 'No local run loaded / local bridge required'
	);
</script>

<div class="dock">
	<div class="card-title">Action dock</div>

	<!-- Disabled apply: HomeOps never applies. -->
	<button class="btn primary block" type="button" disabled title={applyReason}>
		✓ Review &amp; Apply Plan
	</button>
	<div class="reason"><span class="lock">⊘</span> Apply disabled — <b>{applyReason}</b>. Apply runs only in the local Redux Maker app (copied-RPF, SHA + confirm gated).</div>

	<div class="divider"></div>

	<button class="btn block" type="button" onclick={onOpenCorpus}>⌗ Open Redux Corpus</button>
	<button class="btn block" type="button" onclick={onCopyPath}>⧉ Copy Redux Maker path</button>
	<button class="btn block" type="button" onclick={onCopyDevCommand}>⌘ Copy local dev command</button>
	<button class="btn block" type="button" onclick={onRefresh}>↻ Refresh corpus status</button>

	<div class="path" title={standalonePath}>{standalonePath}</div>

	{#if actionMessage}<div class="toast">{actionMessage}</div>{/if}

	<div class="safety">🛡 HomeOps server does not edit RPF files. No server-side apply, no CodeWalker, no RPF write endpoints.</div>
</div>

<style>
	.dock { padding: 10px 12px; display: flex; flex-direction: column; gap: 7px; background: var(--bg-sidebar); min-height: 0; }
	.card-title { color: var(--color-text-primary); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.btn { padding: 7px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; text-align: left; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn.block { width: 100%; }
	.btn.primary { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.reason { color: var(--color-text-tertiary); font-size: 10.5px; line-height: 1.5; }
	.reason b { color: var(--color-text-warning); }
	.lock { color: var(--color-text-warning); }
	.divider { height: 0.5px; background: var(--color-border-tertiary); margin: 4px 0; }
	.path { color: var(--text-faint); font-family: var(--font-mono); font-size: 10px; overflow-wrap: anywhere; }
	.toast { color: var(--color-text-success); font-size: 11px; }
	.safety { margin-top: 4px; color: var(--color-text-success); font-size: 10.5px; line-height: 1.5; border-top: 0.5px solid var(--color-border-tertiary); padding-top: 8px; }
</style>
