<script lang="ts">
	import { PRESETS, type RunMode } from '$lib/redux-maker/presets';
	import type { ContextPack } from '$lib/redux-maker/contextPack';
	import type { RunStatus } from '$lib/redux-maker/bridge';

	let {
		desktop = false,
		bridgeReady = false,
		running = false,
		prompt = $bindable(''),
		mode = $bindable<RunMode>('planOnly'),
		provider = 'rule_based',
		runStatus = null,
		runError = null,
		attachedContext = null,
		onPreset = (_id: string) => {},
		onGenerate = () => {},
		onCancel = () => {},
		onClearContext = () => {}
	}: {
		desktop?: boolean;
		bridgeReady?: boolean;
		running?: boolean;
		prompt?: string;
		mode?: RunMode;
		provider?: string;
		runStatus?: RunStatus | null;
		runError?: string | null;
		attachedContext?: ContextPack | null;
		onPreset?: (id: string) => void;
		onGenerate?: () => void;
		onCancel?: () => void;
		onClearContext?: () => void;
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
	const command = $derived(
		runStatus?.commandPreview ??
			`rpf_backend_rs ai-redux-maker --prompt <prompt> --provider ${provider} --mode full_plan_no_execute${mode === 'applyReadyProof' ? ' --include-ytd-build --target-rpf <copied.rpf> --expect-sha <clean>' : ''}`
	);
</script>

<div class="ai-terminal">
	<div class="ai-prompt-header">
		<span class="prompt-sym">&gt;</span>
		<span style="color:var(--text-1);font-size:13px">AI Patch Prompt</span>
		<span style="color:var(--text-3)">·</span>
		<span class="prompt-model">{provider} · plan-only generation · apply SHA+confirm gated</span>
		<div class="prompt-guard">
			<span style="width:5px;height:5px;border-radius:50%;background:var(--green);display:inline-block"></span>
			{running ? 'running plan-only…' : 'no apply during generation'}
		</div>
	</div>

	<div class="macro-row">
		<span class="macro-label">presets</span>
		{#each PRESETS as p}
			<button class="macro-pill" type="button" disabled={!desktop || !bridgeReady || running} onclick={() => onPreset(p.id)} title={p.prompt}>{p.pill}</button>
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
			<textarea bind:value={prompt} rows="3" maxlength="4096" disabled={running}
				placeholder="Describe a safe Redux module intent. Generation runs the local plan-only ai-redux-maker; apply (if any) targets only the copied test RPF behind a SHA + exact-confirmation gate."></textarea>
			<span class="input-counter">{prompt.length} / 4096</span>
		</div>
		<div class="input-actions">
			<label class="input-check" title="Apply-ready proof builds a real YTD + copied-RPF replacement plan">
				<input type="checkbox" disabled={running} checked={mode === 'applyReadyProof'}
					onchange={(e) => (mode = (e.currentTarget as HTMLInputElement).checked ? 'applyReadyProof' : 'planOnly')} />
				apply-ready proof
			</label>
			{#if running}
				<button class="btn-secondary" type="button" onclick={onCancel}>✕ Cancel Run</button>
			{:else}
				<button class="btn-primary" type="button" disabled={!canGenerate} title={disabledReason} onclick={onGenerate}>⚡ Generate Module Plan</button>
			{/if}
		</div>
	</div>

	{#if runError}
		<div class="command-preview"><code style="color:var(--red)">{runError}</code></div>
	{:else if disabledReason}
		<div class="command-preview">generate disabled: <code style="color:var(--amber)">{disabledReason}</code></div>
	{:else}
		<div class="command-preview">plan-only command: <code>{command}</code></div>
	{/if}
</div>
