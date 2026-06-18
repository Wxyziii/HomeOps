<script lang="ts">
	import type { BridgeStatus, RunStatus } from '$lib/redux-maker/bridge';

	let {
		desktop = false,
		bridge = null,
		bridgeLoading = false,
		bridgeError = null,
		corpusConnected = false,
		runStatus = null,
		onRefresh = () => {},
		onOpenSettings = () => {},
		onOpenCorpus = () => {}
	}: {
		desktop?: boolean;
		bridge?: BridgeStatus | null;
		bridgeLoading?: boolean;
		bridgeError?: string | null;
		corpusConnected?: boolean;
		runStatus?: RunStatus | null;
		onRefresh?: () => void;
		onOpenSettings?: () => void;
		onOpenCorpus?: () => void;
	} = $props();

	const targetLabel = $derived(
		(runStatus?.report?.targetRpf as string) ?? bridge?.copiedRpfPath ?? 'no run loaded'
	);
	const bridgeLabel = $derived(
		!desktop
			? 'browser mode'
			: bridgeLoading
				? 'checking…'
				: bridgeError
					? 'error'
					: bridge?.available
						? 'ready'
						: 'unavailable'
	);
	const bridgeOk = $derived(desktop && !!bridge?.available);
</script>

<header class="topbar">
	<div class="topbar-left">
		<div class="brand">
			<div class="brand-icon">RS</div>
			<div class="brand-name">AI GTA V Redux Maker Studio</div>
		</div>
		<span class="brand-ver">Redux Maker Studio</span>
		<div class="topbar-sep"></div>
		<div class="path-pill"><span style="color:var(--text-3)">⌂</span><span class="p">{targetLabel}</span></div>
	</div>

	<div class="topbar-right">
		<button class="topbar-btn" type="button" onclick={onRefresh}>↻ Refresh</button>
		<button class="topbar-btn" type="button" onclick={onOpenCorpus}>Corpus</button>
		<button class="topbar-btn" type="button" onclick={onOpenSettings}>Settings</button>
		<div class="topbar-sep"></div>
		<div class="guard-badge"><span class="guard-dot"></span>◈ ReduxScannerEngine · writerAllowed=false</div>
	</div>
</header>

<div class="chips-row">
	<span class="chip" class:ok={bridgeOk} class:err={desktop && !bridgeOk} class:warn={!desktop}
		title={bridgeError ?? bridge?.reason ?? 'Local bridge runs only in the HomeOps desktop app'}>
		local bridge: <b>{bridgeLabel}</b>
	</span>
	{#if desktop && bridge}
		<span class="chip" class:ok={bridge.scannerBinaryExists} class:err={!bridge.scannerBinaryExists}>
			scanner: <b>{bridge.scannerBinaryExists ? 'found' : 'missing'}</b>
		</span>
		<span class="chip" class:ok={bridge.copiedRpfClean} class:err={!bridge.copiedRpfClean}>
			copied RPF: <b>{bridge.copiedRpfExists ? (bridge.copiedRpfClean ? 'clean' : 'dirty') : 'missing'}</b>
		</span>
		<span class="chip" class:ok={bridge.codewalkerReachable} class:err={!bridge.codewalkerReachable}>
			CodeWalker: <b>{bridge.codewalkerReachable ? 'reachable' : bridge.codewalkerLoopback ? 'offline' : 'non-loopback'}</b>
		</span>
		{#if bridge.localAiReachable !== null}
			<span class="chip" class:ok={bridge.localAiReachable} class:err={!bridge.localAiReachable}>
				local AI: <b>{bridge.localAiReachable ? 'reachable' : 'unreachable'}</b>
			</span>
		{/if}
	{/if}
	<span class="chip" class:ok={corpusConnected}>corpus server: <b>{corpusConnected ? 'ready' : 'idle'}</b></span>
	<span class="chip ok">safety: <b>server never edits RPF</b></span>
</div>
