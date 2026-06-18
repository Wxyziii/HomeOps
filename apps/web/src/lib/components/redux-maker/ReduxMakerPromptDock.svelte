<script lang="ts">
	import { PRESETS, type RunMode } from '$lib/redux-maker/presets';
	import type { ContextPack } from '$lib/redux-maker/contextPack';

	let {
		desktop = false,
		bridgeReady = false,
		running = false,
		prompt = $bindable(''),
		mode = $bindable<RunMode>('planOnly'),
		provider = 'rule_based',
		runError = null,
		attachedContext = null,
		onPreset = (_id: string) => {},
		onGenerate = () => {},
		onCancel = () => {},
		onClearContext = () => {},
		onCopyDevCommand = () => {}
	}: {
		desktop?: boolean;
		bridgeReady?: boolean;
		running?: boolean;
		prompt?: string;
		mode?: RunMode;
		provider?: string;
		runError?: string | null;
		attachedContext?: ContextPack | null;
		onPreset?: (id: string) => void;
		onGenerate?: () => void;
		onCancel?: () => void;
		onClearContext?: () => void;
		onCopyDevCommand?: () => void;
	} = $props();

	const disabledReason = $derived(
		!desktop
			? 'Local bridge unavailable in browser mode'
			: !bridgeReady
				? 'Local bridge not ready'
				: !prompt.trim()
					? 'Enter a prompt or pick a preset'
					: ''
	);
	const canGenerate = $derived(desktop && bridgeReady && !!prompt.trim() && !running);
</script>

<div class="prompt-dock">
	<div class="prompt-head">
		<span class="sym">&gt;</span>
		<span class="ph-title">AI Patch Prompt</span>
		<span class="ph-sep">·</span>
		<span class="ph-mode">provider {provider} · plan-only generation · apply is SHA+confirm gated</span>
		<span class="guard"><span class="g-dot"></span>no apply during generation</span>
	</div>

	<div class="presets">
		{#each PRESETS as p}
			<button class="preset" type="button" disabled={!desktop || !bridgeReady || running}
				onclick={() => onPreset(p.id)} title={p.prompt}>{p.pill}</button>
		{/each}
	</div>

	{#if attachedContext}
		<div class="ctx-block">
			<div class="ctx-head">
				<span class="ctx-title">⛬ Attached corpus context</span>
				<span class="ctx-meta">{attachedContext.recordCount} rec · {attachedContext.categories.join(', ') || '—'} · ~{attachedContext.approxTokens} tok</span>
				<button class="ctx-clear" type="button" onclick={onClearContext} disabled={running}>clear</button>
			</div>
			<pre class="ctx-text">{attachedContext.text}</pre>
		</div>
	{/if}

	<div class="input-row">
		<div class="input-wrap">
			<textarea
				bind:value={prompt}
				rows="3"
				maxlength="4096"
				disabled={running}
				placeholder="Describe a safe Redux module intent. Generation runs the local plan-only ai-redux-maker; apply (if any) targets only the copied test RPF behind a SHA + exact-confirmation gate."
			></textarea>
			<span class="counter">{prompt.length} / 4096</span>
		</div>
		<div class="actions">
			<label class="mode-row" title="Apply-ready proof builds a real YTD + copied-RPF replacement plan">
				<input type="checkbox" disabled={running}
					checked={mode === 'applyReadyProof'}
					onchange={(e) => (mode = (e.currentTarget as HTMLInputElement).checked ? 'applyReadyProof' : 'planOnly')} />
				apply-ready proof
			</label>
			{#if running}
				<button class="btn warn" type="button" onclick={onCancel}>■ Cancel Run</button>
			{:else}
				<button class="btn primary" type="button" disabled={!canGenerate} title={disabledReason}
					onclick={onGenerate}>⚡ Generate Module Plan</button>
			{/if}
			<button class="btn" type="button" onclick={onCopyDevCommand} title="Copy the local dev command">⧉ Copy local command</button>
		</div>
	</div>

	{#if runError}
		<div class="reason err"><span class="lock">⊘</span> {runError}</div>
	{:else if disabledReason}
		<div class="reason"><span class="lock">⊘</span> Generate disabled — <b>{disabledReason}</b>.</div>
	{:else if running}
		<div class="reason"><span class="g-dot"></span> Run in progress — logs update live, app stays responsive.</div>
	{/if}
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
	.ctx-block { margin-top: 8px; border: 0.5px solid var(--orange-border); border-radius: 6px; background: var(--orange-bg); padding: 7px 9px; }
	.ctx-head { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
	.ctx-title { color: var(--accent); font-size: 11px; font-weight: 600; }
	.ctx-meta { color: var(--color-text-tertiary); font-size: 10px; }
	.ctx-clear { margin-left: auto; background: none; border: 0.5px solid var(--color-border-secondary); border-radius: 4px; color: var(--color-text-secondary); font-size: 10px; padding: 2px 7px; cursor: pointer; }
	.ctx-clear:disabled { opacity: 0.5; cursor: not-allowed; }
	.ctx-text { margin: 6px 0 0; max-height: 120px; overflow: auto; font-family: var(--font-mono); font-size: 10px; color: var(--color-text-tertiary); white-space: pre-wrap; overflow-wrap: anywhere; }
	.presets { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 8px; }
	.preset { font-size: 11px; padding: 3px 9px; border: 0.5px solid var(--color-border-secondary); border-radius: 999px; background: var(--bg-surface); color: var(--color-text-secondary); cursor: pointer; }
	.preset:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.preset:disabled { opacity: 0.45; cursor: not-allowed; }
	.input-row { display: flex; gap: 10px; align-items: flex-end; margin-top: 8px; }
	.input-wrap { position: relative; flex: 1; min-width: 0; }
	textarea { width: 100%; resize: vertical; min-height: 56px; padding: 8px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; outline: none; }
	textarea:focus { border-color: var(--accent); }
	textarea:disabled { opacity: 0.6; }
	.counter { position: absolute; right: 8px; bottom: 6px; color: var(--text-faint); font-size: 10px; }
	.actions { display: flex; flex-direction: column; gap: 6px; flex: none; }
	.mode-row { display: flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-text-tertiary); }
	.btn { padding: 7px 11px; border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-md); background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; white-space: nowrap; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn.primary { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.btn.warn { background: var(--red-bg, transparent); border-color: var(--color-border-danger, var(--red)); color: var(--color-text-danger); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.reason { margin-top: 8px; color: var(--color-text-tertiary); font-size: 11px; }
	.reason.err { color: var(--color-text-danger); }
	.reason b { color: var(--color-text-warning); }
	.lock { color: var(--color-text-warning); }
	@media (max-width: 720px) { .input-row { flex-direction: column; align-items: stretch; } .actions { flex-direction: row; flex-wrap: wrap; } }
</style>
