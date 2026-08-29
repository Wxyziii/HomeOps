<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import {
		getWorkspaceStatus,
		getServerStoragePool,
		readServerFileJson,
		type DownloadStatus
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	// Absolute path of the status file the server-side sampler writes.
	const STATUS_ABS = '/mnt/storage/redux-maker/downloads/metadata/status.json';
	// Path of the status file relative to the redux-maker project dir.
	const STATUS_REL = 'redux-maker/downloads/metadata/status.json';
	const POLL_MS = 5000;

	let status = $state<DownloadStatus | null>(null);
	let resolvedRoot = $state<{ rootId: string; relPath: string } | null>(null);
	let error = $state<string | null>(null);
	let timer: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		serverConnection.load();
		void tick();
		timer = setInterval(() => void tick(), POLL_MS);
	});
	onDestroy(() => timer && clearInterval(timer));

	// Resolve which root + relative path actually serves the status file by
	// probing candidates (path strings from the API don't always line up with
	// the absolute path, so we just try to read and keep what works).
	async function resolveRoot() {
		// Gather roots from both the workspace and the storage pool (the bulk root
		// may only appear in one of them).
		const roots: Array<{ id: string; path: string }> = [];
		try {
			const ws = await getWorkspaceStatus(serverConnection.serverUrl);
			for (const r of ws.storage_roots ?? []) roots.push({ id: r.id, path: r.path });
		} catch {
			/* ignore */
		}
		try {
			const pool = await getServerStoragePool(serverConnection.serverUrl);
			for (const r of pool.pool?.roots ?? []) roots.push({ id: r.rootId, path: r.path });
		} catch {
			/* ignore */
		}

		const candidates: Array<{ rootId: string; relPath: string }> = [];
		const seen = new Set<string>();
		for (const root of roots) {
			const base = root.path.replace(/\/+$/, '');
			if (STATUS_ABS === base || STATUS_ABS.startsWith(base + '/')) {
				const key = `${root.id}|${STATUS_ABS.slice(base.length + 1)}`;
				if (!seen.has(key)) { seen.add(key); candidates.push({ rootId: root.id, relPath: STATUS_ABS.slice(base.length + 1) }); }
			}
			const key2 = `${root.id}|${STATUS_REL}`;
			if (!seen.has(key2)) { seen.add(key2); candidates.push({ rootId: root.id, relPath: STATUS_REL }); }
		}
		for (const c of candidates) {
			try {
				await readServerFileJson<DownloadStatus>(
					serverConnection.serverUrl,
					c.relPath,
					c.rootId
				);
				resolvedRoot = c;
				return;
			} catch {
				// try the next candidate
			}
		}
		throw new Error('downloads status not found in any storage root yet.');
	}

	async function tick() {
		try {
			if (!resolvedRoot) await resolveRoot();
			if (!resolvedRoot) return;
			status = await readServerFileJson<DownloadStatus>(
				serverConnection.serverUrl,
				resolvedRoot.relPath,
				resolvedRoot.rootId
			);
			error = null;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not read download status.';
		}
	}

	function fmtBytes(b: number): string {
		const u = ['B', 'KB', 'MB', 'GB', 'TB'];
		let s = b;
		let i = 0;
		while (s >= 1024 && i < u.length - 1) {
			s /= 1024;
			i += 1;
		}
		return `${s.toFixed(i === 0 ? 0 : 1)} ${u[i]}`;
	}

	function fmtEta(sec: number | null): string {
		if (sec === null || sec <= 0) return '—';
		const h = Math.floor(sec / 3600);
		const m = Math.floor((sec % 3600) / 60);
		if (h > 0) return `${h}h ${m}m`;
		if (m > 0) return `${m}m`;
		return `${sec}s`;
	}
</script>

{#if status}
	<section class="dlp">
		<div class="head">
			<div class="title">
				<span class="dot {status.running ? 'live' : 'idle'}"></span>
				Mod downloads
				<span class="sub">{status.running ? status.activeSessions.join(', ') || 'running' : 'idle'}</span>
			</div>
			<div class="pct">{status.percent}%</div>
		</div>

		<div class="bar"><div style={`width:${Math.min(status.percent, 100)}%`}></div></div>

		<div class="stats">
			<div><span>done</span><strong>{status.done}</strong></div>
			<div><span>remaining</span><strong>{status.remaining}</strong></div>
			<div><span>failed</span><strong class={status.failed ? 'warn' : ''}>{status.failed}</strong></div>
			<div><span>downloaded</span><strong>{fmtBytes(status.sizeBytes)}</strong></div>
			<div><span>speed</span><strong>{fmtBytes(status.rateBytesPerSec)}/s</strong></div>
			<div><span>eta</span><strong>{fmtEta(status.etaSeconds)}</strong></div>
		</div>

		{#if status.current}
			<div class="current">
				<i class="ti ti-download" aria-hidden="true"></i>
				<span class="cur-name" title={status.current.name}>{status.current.name}</span>
				<span class="cur-size">{fmtBytes(status.current.bytes)}</span>
			</div>
		{/if}
		<div class="foot">updated {status.updatedAt.replace('T', ' ')} · {status.filesPerMin}/min</div>
	</section>
{:else if error}
	<section class="dlp"><div class="muted">Downloads: {error}</div></section>
{/if}

<style>
	.dlp { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; }
	.head { display: flex; align-items: center; justify-content: space-between; }
	.title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; color: var(--color-text-primary); }
	.title .sub { color: var(--color-text-tertiary); font-size: 11px; font-weight: 400; }
	.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-faint); }
	.dot.live { background: var(--color-text-success); box-shadow: 0 0 0 3px rgba(47, 143, 31, 0.18); }
	.pct { font-size: 14px; font-weight: 700; color: var(--accent); }
	.bar { height: 6px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; }
	.bar div { height: 100%; background: var(--accent); transition: width 0.4s ease; }
	.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(90px, 1fr)); gap: 10px; }
	.stats span { display: block; color: var(--text-faint); font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; }
	.stats strong { display: block; margin-top: 2px; color: var(--color-text-primary); font-size: 14px; }
	.stats strong.warn { color: var(--color-text-danger); }
	.current { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--color-text-secondary); border-top: 0.5px solid var(--color-border-tertiary); padding-top: 8px; }
	.cur-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
	.cur-size { color: var(--color-text-tertiary); }
	.foot { color: var(--text-faint); font-size: 10px; }
	.muted { color: var(--color-text-tertiary); font-size: 12px; }
</style>
