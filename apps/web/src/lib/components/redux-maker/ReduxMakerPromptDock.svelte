<script lang="ts">
	let {
		bridgeConnected = false,
		onCopyDevCommand = () => {}
	}: { bridgeConnected?: boolean; onCopyDevCommand?: () => void } = $props();

	// The prompt is a composer only. Generation runs in the local Redux Maker
	// app — there is no server-side generate/apply, so the button stays disabled
	// with a truthful reason until the H2.1 bridge connects.
	let prompt = $state('');
	const disabledReason = $derived(
		bridgeConnected ? '' : 'Local bridge not connected'
	);
</script>

<div class="prompt-dock">
	<div class="prompt-head">
		<span class="sym">&gt;</span>
		<span class="ph-title">AI Patch Prompt</span>
		<span class="ph-sep">·</span>
		<span class="ph-mode">local maker · plan-only · apply disabled</span>
		<span class="guard"><span class="g-dot"></span>no execution from HomeOps</span>
	</div>

	<div class="input-row">
		<div class="input-wrap">
			<textarea
				bind:value={prompt}
				rows="3"
				placeholder="Describe a safe Redux module intent. Generation runs in the local Redux Maker app (plan-only ai-redux-maker) — HomeOps never applies, never writes copied-RPF, never calls CodeWalker."
			></textarea>
			<span class="counter">{prompt.length} / 4096</span>
		</div>
		<div class="actions">
			<button class="btn primary" type="button" disabled title={disabledReason}>⚡ Generate Module Plan</button>
			<button class="btn" type="button" onclick={onCopyDevCommand} title="Copy the local dev command">⧉ Copy local command</button>
		</div>
	</div>

	<div class="reason">
		<span class="lock">⊘</span> Generate is disabled — <b>{disabledReason}</b>. Run the local Redux Maker app to generate and review plans.
	</div>
</div>

<style>
	.prompt-dock { flex: none; border-top: 0.5px solid var(--color-border-tertiary); background: var(--bg-sidebar); padding: 10px 14px; }
	.prompt-head { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
	.sym { color: var(--accent); font-weight: 700; }
	.ph-title { color: var(--color-text-primary); font-size: 13px; }
	.ph-sep { color: var(--text-faint); }
	.ph-mode { color: var(--color-text-tertiary); font-size: 11px; }
	.guard { margin-left: auto; display: flex; align-items: center; gap: 5px; color: var(--color-text-success); font-size: 10.5px; }
	.g-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--green); }
	.input-row { display: flex; gap: 10px; align-items: flex-end; margin-top: 8px; }
	.input-wrap { position: relative; flex: 1; min-width: 0; }
	textarea { width: 100%; resize: vertical; min-height: 56px; padding: 8px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; outline: none; }
	textarea:focus { border-color: var(--accent); }
	.counter { position: absolute; right: 8px; bottom: 6px; color: var(--text-faint); font-size: 10px; }
	.actions { display: flex; flex-direction: column; gap: 6px; flex: none; }
	.btn { padding: 7px 11px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; white-space: nowrap; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn.primary { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.reason { margin-top: 8px; color: var(--color-text-tertiary); font-size: 11px; }
	.reason b { color: var(--color-text-warning); }
	.lock { color: var(--color-text-warning); }
	@media (max-width: 720px) { .input-row { flex-direction: column; align-items: stretch; } .actions { flex-direction: row; flex-wrap: wrap; } }
</style>
