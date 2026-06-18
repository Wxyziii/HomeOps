<script lang="ts">
	let {
		confirmPhrase,
		targetRpf,
		expectedSha,
		currentSha = null,
		codewalkerUrl,
		busy = false,
		error = null,
		onConfirm = (_phrase: string) => {},
		onCancel = () => {}
	}: {
		confirmPhrase: string;
		targetRpf: string;
		expectedSha: string;
		currentSha?: string | null;
		codewalkerUrl: string;
		busy?: boolean;
		error?: string | null;
		onConfirm?: (phrase: string) => void;
		onCancel?: () => void;
	} = $props();

	let typed = $state('');
	const shaMatches = $derived(!!currentSha && currentSha.toLowerCase() === expectedSha.toLowerCase());
	const canApply = $derived(typed === confirmPhrase && shaMatches && !busy);
</script>

<div class="overlay" role="dialog" aria-modal="true">
	<div class="modal">
		<div class="head">
			<strong>Review &amp; Apply to copied test RPF</strong>
			<button class="x" type="button" onclick={onCancel} disabled={busy}>✕</button>
		</div>

		<div class="body">
			<div class="warn-banner">
				Original GTA files are <b>NOT</b> touched. This writes ONLY to the copied test RPF, via the
				scanner's single <code>/api/replace-rpf-entry</code> path on loopback CodeWalker.
			</div>

			<dl class="facts">
				<dt>target copied RPF</dt>
				<dd class="mono">{targetRpf}</dd>
				<dt>expected clean SHA</dt>
				<dd class="mono">{expectedSha}</dd>
				<dt>current SHA</dt>
				<dd class="mono" class:ok={shaMatches} class:bad={!shaMatches}>
					{currentSha ?? 'unknown'} {shaMatches ? '✓ clean' : '✗ not clean'}
				</dd>
				<dt>CodeWalker URL</dt>
				<dd class="mono">{codewalkerUrl}</dd>
			</dl>

			{#if !shaMatches}
				<div class="block-note">Copied RPF is not clean — restore it before applying. Apply is blocked.</div>
			{/if}

			<label class="confirm-label" for="apply-confirm-input">
				Type the exact confirmation phrase to authorize:
				<code>{confirmPhrase}</code>
			</label>
			<input
				id="apply-confirm-input"
				class="confirm-input"
				type="text"
				bind:value={typed}
				disabled={busy || !shaMatches}
				placeholder={confirmPhrase}
				autocomplete="off"
				spellcheck="false"
			/>

			{#if error}<div class="error">{error}</div>{/if}
		</div>

		<div class="foot">
			<button class="btn" type="button" onclick={onCancel} disabled={busy}>Cancel</button>
			<button class="btn apply" type="button" disabled={!canApply} onclick={() => onConfirm(typed)}>
				{busy ? 'Applying…' : 'Apply to copied RPF'}
			</button>
		</div>
	</div>
</div>

<style>
	.overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); display: flex; align-items: center; justify-content: center; z-index: 1000; padding: 16px; }
	.modal { width: min(560px, 100%); max-height: 90vh; overflow: auto; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: var(--border-radius-lg, 8px); display: flex; flex-direction: column; }
	.head { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.head strong { color: var(--color-text-primary); font-size: 14px; }
	.x { background: none; border: none; color: var(--color-text-tertiary); cursor: pointer; font-size: 14px; }
	.body { padding: 14px 16px; display: flex; flex-direction: column; gap: 12px; }
	.warn-banner { font-size: 11.5px; line-height: 1.6; color: var(--color-text-secondary); background: var(--orange-bg); border: 0.5px solid var(--orange-border); border-radius: 6px; padding: 9px 11px; }
	.warn-banner b { color: var(--color-text-danger); }
	.facts { display: grid; grid-template-columns: 130px 1fr; gap: 4px 12px; margin: 0; font-size: 11.5px; }
	.facts dt { color: var(--color-text-tertiary); }
	.facts dd { margin: 0; color: var(--color-text-secondary); }
	.mono { font-family: var(--font-mono); font-size: 10.5px; overflow-wrap: anywhere; }
	.mono.ok { color: var(--color-text-success); }
	.mono.bad { color: var(--color-text-danger); }
	.block-note { color: var(--color-text-danger); font-size: 11.5px; }
	.confirm-label { color: var(--color-text-tertiary); font-size: 11.5px; display: flex; flex-direction: column; gap: 5px; }
	.confirm-label code { color: var(--accent); font-family: var(--font-mono); font-size: 11px; }
	.confirm-input { padding: 8px 10px; border: 0.5px solid var(--color-border-secondary); border-radius: 6px; background: var(--bg-surface); color: var(--color-text-primary); font-family: var(--font-mono); font-size: 12px; outline: none; }
	.confirm-input:focus { border-color: var(--accent); }
	.confirm-input:disabled { opacity: 0.5; }
	.error { color: var(--color-text-danger); font-size: 11.5px; }
	.foot { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 0.5px solid var(--color-border-tertiary); }
	.btn { padding: 8px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: 6px; background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; }
	.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.btn.apply { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
