<script lang="ts">
	import { onMount } from 'svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SmallButton from '$lib/components/SmallButton.svelte';
	import {
		getServerStoragePool,
		resolvePlacement,
		bootstrapStandardFolders,
		type SmartStoragePool,
		type PlacementDecision,
		type PlacementIntent
	} from '$lib/api/client';
	import { serverConnection } from '$lib/stores/serverConnection.svelte';

	let pool = $state<SmartStoragePool | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let actionMessage = $state<string | null>(null);

	// Placement preview tool inputs.
	let previewName = $state('mod-pack.zip');
	let previewSizeGiB = $state(3);
	let previewIntent = $state<PlacementIntent>('upload');
	let previewPath = $state('uploads');
	let previewDecision = $state<PlacementDecision | null>(null);
	let previewBusy = $state(false);

	const intents: PlacementIntent[] = [
		'upload',
		'archive_extract',
		'corpus_inbox',
		'corpus_work',
		'dataset',
		'report',
		'generic'
	];

	onMount(() => {
		serverConnection.load();
		void refresh();
	});

	async function refresh() {
		loading = true;
		error = null;
		try {
			const response = await getServerStoragePool(serverConnection.serverUrl);
			pool = response.pool;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not load storage pool.';
		} finally {
			loading = false;
		}
	}

	async function runPreview() {
		previewBusy = true;
		error = null;
		try {
			const sizeBytes = Math.max(0, Math.round(previewSizeGiB * 1024 * 1024 * 1024));
			const relativePath = previewPath.trim()
				? `${previewPath.trim().replace(/\/+$/, '')}/${previewName.trim()}`
				: previewName.trim();
			const response = await resolvePlacement(serverConnection.serverUrl, {
				intent: previewIntent,
				relativePath,
				fileName: previewName.trim(),
				sizeBytes
			});
			previewDecision = response.decision;
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not resolve placement.';
		} finally {
			previewBusy = false;
		}
	}

	async function bootstrap() {
		actionMessage = null;
		error = null;
		try {
			const response = await bootstrapStandardFolders(serverConnection.serverUrl);
			const created = response.created.length;
			const existing = response.existing.length;
			actionMessage = `Standard folders ready on '${response.rootId}': ${created} created, ${existing} already existed.`;
			await refresh();
		} catch (caught) {
			error = caught instanceof Error ? caught.message : 'Could not bootstrap folders.';
		}
	}

	function formatBytes(bytes: number) {
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = bytes;
		let unit = 0;
		while (size >= 1024 && unit < units.length - 1) {
			size /= 1024;
			unit += 1;
		}
		return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
	}

	function usagePercent(root: { usedBytes: number; totalBytes: number }) {
		if (root.totalBytes <= 0) return 0;
		return Math.min(100, Math.round((root.usedBytes / root.totalBytes) * 1000) / 10);
	}

	const poolUsage = $derived(pool && pool.totalBytes > 0 ? Math.round((pool.usedBytes / pool.totalBytes) * 1000) / 10 : 0);
</script>

<svelte:head><title>Storage · HomeOps Panel</title></svelte:head>
<div class="page">
	<Topbar title="Smart Storage Pool" flush>
		<SmallButton icon="ti-refresh" label={loading ? 'Loading' : 'Refresh'} onclick={refresh} />
		<SmallButton icon="ti-folder-plus" label="Bootstrap corpus folders" onclick={bootstrap} />
	</Topbar>

	<div class="content">
		{#if error}<div class="notice error">{error}</div>{/if}
		{#if actionMessage}<div class="notice success">{actionMessage}</div>{/if}

		{#if pool}
			<section class="panel pool-card">
				<div class="panel-head">
					<div>{pool.displayName}</div>
					<span class="health {pool.health}">{pool.health}</span>
				</div>
				<div class="pool-body">
					<div class="pool-stat">
						<span>Combined capacity</span>
						<strong>{formatBytes(pool.totalBytes)}</strong>
					</div>
					<div class="pool-stat">
						<span>Free</span>
						<strong>{formatBytes(pool.freeBytes)}</strong>
					</div>
					<div class="pool-stat">
						<span>Used</span>
						<strong>{formatBytes(pool.usedBytes)} · {poolUsage}%</strong>
					</div>
					<div class="pool-stat">
						<span>Roots</span>
						<strong>{pool.roots.length}</strong>
					</div>
				</div>
				<div class="bar wide"><div style={`width:${poolUsage}%`}></div></div>
				{#if pool.warnings.length}
					<div class="warns">{pool.warnings.join(' · ')}</div>
				{/if}
				<p class="note">
					Logical view only — disks are <strong>not</strong> physically merged. Roots stay separate;
					placement is chosen automatically by policy.
				</p>
			</section>

			<section class="panel">
				<div class="panel-head"><div>Placement policy</div></div>
				<div class="policy-grid">
					<div class="policy-item"><i class="ti ti-database"></i><div><strong>Large files → bulk</strong><span>≥ {formatBytes(pool.policy.largeFileThresholdBytes)} routed to bulk</span></div></div>
					<div class="policy-item"><i class="ti ti-package"></i><div><strong>Redux corpus → bulk</strong><span>redux-corpus/* always on the bulk root</span></div></div>
					<div class="policy-item"><i class="ti ti-archive"></i><div><strong>Archives → bulk</strong><span>{pool.policy.archiveExtensions.map((e) => '.' + e).join(' ')}</span></div></div>
					<div class="policy-item"><i class="ti ti-file-text"></i><div><strong>Metadata/small → main</strong><span>small files & reports default to main</span></div></div>
					<div class="policy-item"><i class="ti ti-shield"></i><div><strong>Reserves</strong><span>main {formatBytes(pool.policy.mainReserveBytes)} · bulk {formatBytes(pool.policy.bulkReserveBytes)}</span></div></div>
					<div class="policy-item"><i class="ti ti-route"></i><div><strong>Routing roots</strong><span>default {pool.defaultRoot ?? '—'} · corpus {pool.corpusRoot ?? '—'}</span></div></div>
				</div>
			</section>

			<section class="panel">
				<div class="panel-head"><div>Roots</div><span>{pool.roots.length} configured</span></div>
				<div class="root-grid">
					{#each pool.roots as root}
						<div class="root-card">
							<div class="root-title">
								<strong>{root.label}</strong>
								<span class="role">{root.role}</span>
							</div>
							<div class="root-id">{root.rootId}</div>
							<div class="root-path">{root.path}</div>
							<div class="root-meta">
								<span>{root.available ? 'available' : 'unavailable'}</span>
								<span>{root.writable ? 'writable' : 'read-only'}</span>
								<span>reserve {formatBytes(root.reservedBytes)}</span>
							</div>
							<div class="root-cap">{formatBytes(root.freeBytes)} free / {formatBytes(root.totalBytes)}</div>
							<div class="bar wide"><div style={`width:${usagePercent(root)}%`}></div></div>
							{#if root.warnings.length}<div class="warns">{root.warnings.join(' · ')}</div>{/if}
						</div>
					{/each}
				</div>
			</section>

			<section class="panel">
				<div class="panel-head"><div>Placement preview</div></div>
				<div class="preview">
					<div class="preview-inputs">
						<label>File name<input type="text" bind:value={previewName} /></label>
						<label>Size (GiB)<input type="number" min="0" step="0.5" bind:value={previewSizeGiB} /></label>
						<label>Intent
							<select bind:value={previewIntent}>
								{#each intents as intent}<option value={intent}>{intent}</option>{/each}
							</select>
						</label>
						<label>Folder<input type="text" bind:value={previewPath} placeholder="uploads" /></label>
						<SmallButton icon="ti-player-play" label={previewBusy ? 'Resolving' : 'Resolve placement'} onclick={runPreview} />
					</div>
					{#if previewDecision}
						<div class="decision {previewDecision.allowed ? 'ok' : 'blocked'}">
							<div class="decision-head">
								<strong>{previewDecision.allowed ? `→ ${previewDecision.selectedRootId}` : 'Blocked'}</strong>
								<span>{previewDecision.reason}</span>
							</div>
							{#if previewDecision.selectedRelativePath}
								<div class="decision-path">{previewDecision.selectedRelativePath}</div>
							{/if}
							{#if previewDecision.warnings.length}
								<div class="warns">{previewDecision.warnings.join(' · ')}</div>
							{/if}
							<div class="decision-meta">
								<span>required {formatBytes(previewDecision.requiredFreeBytes)}</span>
								{#if previewDecision.rootFreeBytes !== null}<span>root free {formatBytes(previewDecision.rootFreeBytes)}</span>{/if}
								{#if previewDecision.alternatives.length}<span>alts: {previewDecision.alternatives.join(', ')}</span>{/if}
							</div>
						</div>
					{/if}
				</div>
			</section>

			<section class="panel">
				<div class="panel-head"><div>Redux corpus</div><span>bulk policy</span></div>
				<div class="corpus">
					<p>Redux corpus folders resolve to the <strong>bulk</strong> root and are bootstrapped under <code>redux-corpus/</code>:</p>
					<div class="corpus-tags">
						{#each ['inbox', 'input', 'work', 'out', 'datasets', 'reports', 'quarantine'] as sub}
							<span class="tag">redux-corpus/{sub}</span>
						{/each}
					</div>
					<p class="note">T2.2 (HomeOps Redux Corpus Job Integration) will use this smart pool policy to place corpus inbox/datasets on bulk automatically.</p>
				</div>
			</section>
		{:else if loading}
			<div class="notice">Loading storage pool…</div>
		{:else}
			<div class="notice">No storage pool loaded yet.</div>
		{/if}
	</div>
</div>

<style>
	.page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
	.content { flex: 1; overflow: auto; padding: 20px; display: flex; flex-direction: column; gap: 14px; background: var(--bg-surface); }
	.notice { padding: 8px 10px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); color: var(--color-text-secondary); font-size: 12px; }
	.notice.error { color: var(--color-text-danger); background: var(--color-background-danger); }
	.notice.success { color: var(--color-text-success); background: rgba(47, 143, 31, 0.08); }
	.panel { background: var(--bg-app); border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-lg); overflow: hidden; }
	.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 14px; border-bottom: 0.5px solid var(--color-border-tertiary); color: var(--color-text-primary); font-size: 13px; font-weight: 600; }
	.panel-head span { color: var(--color-text-secondary); font-size: 11px; }
	.health { text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; padding: 2px 8px; border-radius: 999px; border: 0.5px solid var(--color-border-tertiary); }
	.health.healthy { color: var(--color-text-success); }
	.health.degraded { color: var(--color-text-danger); }
	.pool-body { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; padding: 14px; }
	.pool-stat span { display: block; color: var(--color-text-tertiary); font-size: 11px; }
	.pool-stat strong { display: block; margin-top: 4px; color: var(--color-text-primary); font-size: 18px; font-weight: 600; }
	.note { padding: 0 14px 12px; color: var(--color-text-tertiary); font-size: 11px; }
	.warns { padding: 0 14px 10px; color: var(--color-text-danger); font-size: 11px; }
	.bar.wide { display: block; width: calc(100% - 28px); height: 5px; margin: 0 14px 10px; border-radius: 999px; background: var(--bg-surface-2); overflow: hidden; }
	.bar.wide div { height: 100%; background: var(--accent); }
	.policy-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; padding: 12px; }
	.policy-item { display: flex; gap: 10px; align-items: flex-start; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-sidebar); padding: 10px; }
	.policy-item i { color: var(--accent); font-size: 16px; margin-top: 1px; }
	.policy-item strong { display: block; color: var(--color-text-primary); font-size: 12px; }
	.policy-item span { color: var(--color-text-tertiary); font-size: 11px; }
	.root-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; padding: 12px; }
	.root-card { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-sidebar); padding: 10px; min-width: 0; }
	.root-title { display: flex; justify-content: space-between; gap: 10px; color: var(--color-text-primary); font-size: 13px; }
	.role { color: var(--accent); font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; }
	.root-id { color: var(--color-text-tertiary); font-size: 11px; margin-top: 2px; }
	.root-path { margin-top: 6px; color: var(--color-text-secondary); font-family: var(--font-mono); font-size: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.root-meta { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 8px; color: var(--color-text-tertiary); font-size: 11px; }
	.root-cap { margin-top: 6px; color: var(--color-text-secondary); font-size: 11px; }
	.preview { padding: 12px; display: flex; flex-direction: column; gap: 12px; }
	.preview-inputs { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 10px; }
	.preview-inputs label { display: flex; flex-direction: column; gap: 4px; color: var(--color-text-tertiary); font-size: 11px; }
	.preview-inputs input, .preview-inputs select { height: 32px; min-width: 130px; border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); background: var(--bg-app); color: var(--color-text-primary); font-size: 12px; padding: 0 8px; }
	.decision { border: 0.5px solid var(--color-border-tertiary); border-radius: var(--border-radius-md); padding: 10px; background: var(--bg-sidebar); }
	.decision.ok { border-color: color-mix(in srgb, var(--accent) 50%, var(--color-border-tertiary)); }
	.decision.blocked { border-color: color-mix(in srgb, var(--color-text-danger) 40%, var(--color-border-tertiary)); }
	.decision-head { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
	.decision-head strong { color: var(--color-text-primary); font-size: 14px; }
	.decision-head span { color: var(--color-text-secondary); font-size: 12px; }
	.decision-path { margin-top: 6px; color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 11px; }
	.decision-meta { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 10px; color: var(--color-text-tertiary); font-size: 11px; }
	.corpus { padding: 12px 14px; color: var(--color-text-secondary); font-size: 12px; }
	.corpus-tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 8px 0; }
	.tag { border: 0.5px solid var(--color-border-tertiary); border-radius: 999px; background: var(--bg-sidebar); color: var(--color-text-tertiary); font-family: var(--font-mono); font-size: 10px; padding: 3px 8px; }
	code { font-family: var(--font-mono); color: var(--accent); }
	@media (max-width: 1100px) { .pool-body { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
