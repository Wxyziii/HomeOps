<script lang="ts">
	import { untrack } from 'svelte';
	import { isLoopbackUrl, type BridgeSettings } from '$lib/redux-maker/bridgeSettings';

	let {
		settings,
		localAiReachable = null,
		onSave = (_s: BridgeSettings) => {},
		onCancel = () => {}
	}: {
		settings: BridgeSettings;
		localAiReachable?: boolean | null;
		onSave?: (s: BridgeSettings) => void;
		onCancel?: () => void;
	} = $props();

	// Local editable copy (modal remounts per open); paths + SHA are fixed in the
	// bridge and shown read-only. Seed once from the initial settings snapshot.
	let provider = $state(untrack(() => settings.provider));
	let allowLocalAi = $state(untrack(() => settings.allowLocalAi));
	let localAiUrl = $state(untrack(() => settings.localAiUrl));
	let model = $state(untrack(() => settings.model));
	let codewalkerUrl = $state(untrack(() => settings.codewalkerUrl));

	const localProvider = $derived(provider !== 'rule_based');
	const localUrlOk = $derived(isLoopbackUrl(localAiUrl));
	const cwUrlOk = $derived(isLoopbackUrl(codewalkerUrl));
	const error = $derived(
		!cwUrlOk
			? 'CodeWalker URL must be loopback (127.0.0.1 / localhost)'
			: localProvider && allowLocalAi && !localUrlOk
				? 'Local AI URL must be loopback (public model URLs are blocked)'
				: localProvider && !allowLocalAi
					? 'Local AI provider requires "Allow local AI" enabled'
					: ''
	);

	function save() {
		if (error) return;
		onSave({ ...settings, provider, allowLocalAi, localAiUrl, model, codewalkerUrl });
	}
</script>

<div class="rms-overlay" role="dialog" aria-modal="true">
	<div class="rms-modal">
		<div class="rms-head">
			<strong>Redux Maker bridge settings</strong>
			<button class="rms-x" type="button" onclick={onCancel}>✕</button>
		</div>

		<div class="rms-body">
			<label class="rms-row" for="rms-provider">
				<span>Provider</span>
				<select id="rms-provider" bind:value={provider}>
					<option value="ollama_local">ollama_local (default)</option>
					<option value="lmstudio_local">lmstudio_local</option>
					<option value="rule_based">rule_based (no AI, offline)</option>
				</select>
			</label>

			{#if localProvider}
				<label class="rms-row rms-toggle" for="rms-allow">
					<input id="rms-allow" type="checkbox" bind:checked={allowLocalAi} />
					<span>Allow local AI</span>
				</label>
				<label class="rms-row" for="rms-aiurl">
					<span>Local AI URL</span>
					<input id="rms-aiurl" type="text" bind:value={localAiUrl} spellcheck="false" autocomplete="off" />
				</label>
				<label class="rms-row" for="rms-model">
					<span>Model <em>(optional)</em></span>
					<input id="rms-model" type="text" bind:value={model} placeholder="e.g. qwen2.5:7b" spellcheck="false" autocomplete="off" />
				</label>
				<div class="rms-hint">
					Local AI:
					<b class:rms-ok={localAiReachable === true} class:rms-bad={localAiReachable === false}>
						{localAiReachable === true ? 'reachable' : localAiReachable === false ? 'unreachable' : 'unknown'}
					</b>
					· loopback-only · public URLs blocked · falls back to rule_based if down.
				</div>
			{/if}

			<label class="rms-row" for="rms-cw">
				<span>CodeWalker URL</span>
				<input id="rms-cw" type="text" bind:value={codewalkerUrl} spellcheck="false" autocomplete="off" />
			</label>

			<div class="rms-fixed">
				<div class="rms-fixed-title">Fixed (bridge-owned, read-only)</div>
				<div class="rms-fixed-row">scanner: <code>{settings.scannerBinaryPath}</code></div>
				<div class="rms-fixed-row">copied RPF: <code>{settings.copiedRpfPath}</code></div>
				<div class="rms-fixed-row">expected clean SHA: <code>{settings.expectedCopiedRpfSha}</code></div>
			</div>

			{#if error}<div class="rms-error">{error}</div>{/if}
		</div>

		<div class="rms-foot">
			<button class="rms-btn" type="button" onclick={onCancel}>Cancel</button>
			<button class="rms-btn rms-save" type="button" disabled={!!error} onclick={save}>Save</button>
		</div>
	</div>
</div>

<style>
	.rms-overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.6); display: flex; align-items: center; justify-content: center; z-index: 2000; padding: 16px; }
	.rms-modal { width: min(520px, 100%); max-height: 90vh; overflow: auto; background: var(--bg-app); border: 0.5px solid var(--color-border-secondary); border-radius: 8px; display: flex; flex-direction: column; box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5); }
	.rms-head { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 0.5px solid var(--color-border-tertiary); }
	.rms-head strong { color: var(--color-text-primary); font-size: 14px; }
	.rms-x { background: none; border: none; color: var(--color-text-tertiary); cursor: pointer; font-size: 14px; }
	.rms-body { padding: 14px 16px; display: flex; flex-direction: column; gap: 12px; }
	.rms-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; font-size: 12px; color: var(--color-text-secondary); margin: 0; }
	.rms-row > span { flex: none; }
	.rms-row em { color: var(--text-faint); font-style: normal; }
	.rms-row select, .rms-row input[type='text'] { flex: 1; min-width: 0; padding: 6px 8px; border: 0.5px solid var(--color-border-secondary); border-radius: 6px; background: var(--bg-surface); color: var(--color-text-primary); font-size: 12px; font-family: var(--font-mono); }
	.rms-toggle { justify-content: flex-start; gap: 8px; }
	.rms-toggle input { flex: none; }
	.rms-hint { font-size: 10.5px; color: var(--text-faint); line-height: 1.5; }
	.rms-hint b.rms-ok { color: var(--color-text-success); }
	.rms-hint b.rms-bad { color: var(--color-text-danger); }
	.rms-fixed { border-top: 0.5px solid var(--color-border-tertiary); padding-top: 10px; display: flex; flex-direction: column; gap: 4px; }
	.rms-fixed-title { color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; }
	.rms-fixed-row { font-size: 10px; color: var(--color-text-tertiary); overflow-wrap: anywhere; }
	.rms-fixed code { font-family: var(--font-mono); color: var(--accent); }
	.rms-error { color: var(--color-text-danger); font-size: 11.5px; }
	.rms-foot { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 0.5px solid var(--color-border-tertiary); }
	.rms-btn { padding: 8px 14px; border: 0.5px solid var(--color-border-secondary); border-radius: 6px; background: var(--bg-surface); color: var(--color-text-secondary); font-size: 12px; cursor: pointer; }
	.rms-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--color-text-primary); }
	.rms-save { background: var(--orange-bg); border-color: var(--orange-border); color: var(--accent); }
	.rms-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
