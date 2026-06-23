<svelte:options runes={false} />
<script>
  // @ts-nocheck
	import './prototype/prototype.css';
	import Sidebar from './prototype/Sidebar.svelte';
	import PromptComposer from './prototype/PromptComposer.svelte';
	import ChatWorkspace from './prototype/ChatWorkspace.svelte';
	import PreviewPane from './prototype/PreviewPane.svelte';

	const validationItems = [
		'Safe target detected',
		'No dangerous files',
		'No blocked file types',
		'Backup manifest will be included',
		'Ready to build update.rpf'
	];

	let prompt = '';
	let selectedPreset = '';
	let workflowState = 'idle';
	let changes = [];
	let selectedChangeId = '';
	let buildProgress = 0;
	let weaponViewMode = 'firstPerson';
	let visualPreviewMode = 'comparison';
	let previewCollapsed = false;
	let permissionMode = 'safe';
	let timers = [];
	let messages = [];
	let activeView = 'new';
	let packs = [];
	let lastPrompt = '';

	$: selectedChange = changes.find((change) => change.id === selectedChangeId) ?? changes[0];
	$: composerDocked = workflowState !== 'idle' || messages.length > 0;
	$: packTitle = selectedPreset || inferPackTitle(lastPrompt);

	function newId() {
		return globalThis.crypto?.randomUUID?.() ?? `msg-${Date.now()}-${Math.random().toString(16).slice(2)}`;
	}

	function inferPackTitle(text) {
		const lowered = text.toLowerCase();
		if (!text) return 'Redux Pack';
		if (lowered.includes('pvp')) return 'PvP Visual Pack';
		if (lowered.includes('gun') || lowered.includes('weapon') || lowered.includes('rifle')) return 'Gunpack Redux';
		if (lowered.includes('tracer')) return 'Tracer Redux Pack';
		return 'Redux Pack';
	}

	function createChangesFromPrompt(promptText) {
		const lowered = promptText.toLowerCase();
		const wantsGunpack =
			lowered.includes('gun') ||
			lowered.includes('weapon') ||
			lowered.includes('rifle') ||
			lowered.includes('pistol');
		const wantsSky = lowered.includes('sky') || lowered.includes('timecycle') || lowered.includes('night');
		const wantsHit = lowered.includes('hit') || lowered.includes('kill') || lowered.includes('effect');

		const nextChanges = [
			{
				id: 'tracers',
				title: 'Tracer Texture Pass',
				category: 'visual',
				previewMode: 'comparison',
				status: 'pending',
				description: 'Uses the supplied tracer pattern as the base and creates a cleaner blue modified version.'
			}
		];

		if (wantsSky) {
			nextChanges.push({
				id: 'sky',
				title: 'Sky and Timecycle Cleanup',
				category: 'visual',
				previewMode: 'comparison',
				status: 'pending',
				description: 'Adjusts sky contrast while keeping the pack readable for PvP.'
			});
		}

		if (wantsHit) {
			nextChanges.push({
				id: 'hit-effect',
				title: 'Hit Effect Visibility',
				category: 'visual',
				previewMode: 'comparison',
				status: 'pending',
				description: 'Improves hit feedback without adding heavy visual noise.'
			});
		}

		nextChanges.push({
			id: 'textures',
			title: 'Performance Texture Pass',
			category: 'visual',
			previewMode: 'comparison',
			status: 'pending',
			description: 'Keeps texture choices light and FPS-friendly for the final update.rpf.'
		});

		if (wantsGunpack) {
			nextChanges.push({
				id: 'carbine',
				title: 'Gunpack Model Preview',
				category: 'gunpack',
				previewMode: 'weapon3d',
				status: 'pending',
				description: 'Previews the requested gunpack change in the right workspace model viewer.',
				weapon: 'Selected weapon',
				model: 'Clean tactical replacement',
				previewManifestUrl: '/redux-previews/demo-rifle/weapon_preview_manifest.json'
			});
		}

		return nextChanges;
	}

	function clearTimers() {
		timers.forEach((timer) => clearTimeout(timer));
		timers = [];
	}

	function schedule(callback, delay) {
		const timer = setTimeout(callback, delay);
		timers = [...timers, timer];
	}

	function runWorkflow() {
		const promptText = prompt.trim();
		if (!promptText) return;

		clearTimers();
		activeView = 'new';
		lastPrompt = promptText;
		prompt = '';
		changes = applyPermissionMode(createChangesFromPrompt(promptText));
		selectedChangeId = changes[0]?.id ?? '';
		buildProgress = 0;
		workflowState = 'planning';
		messages = [
			...messages,
			{ id: newId(), role: 'user', text: promptText },
			{
				id: newId(),
				role: 'assistant',
				text: 'I am turning that into reviewable Redux changes for the workspace.',
				kind: 'thinking'
			}
		];

		schedule(() => {
			messages = [
				...messages.filter((message) => message.kind !== 'thinking'),
				{
					id: newId(),
					role: 'assistant',
					text:
						permissionMode === 'autoPilot'
							? 'Auto Pilot is enabled. I selected the changes and will validate/build automatically in this prototype.'
							: 'I created the changes below. Open each card to preview it, then accept, regenerate, or skip.',
					kind: 'changes'
				}
			];

			if (permissionMode === 'autoPilot') {
				startValidationAndBuild();
				return;
			}

			workflowState = 'reviewing';
			schedule(() => maybeAutoValidate(), 120);
		}, 850);
	}

	function applyPermissionMode(nextChanges) {
		if (permissionMode === 'safe') return nextChanges;

		return nextChanges.map((change) => {
			const shouldAutoAccept = permissionMode === 'autoPilot' || change.category !== 'visual';
			return shouldAutoAccept ? { ...change, status: 'accepted' } : change;
		});
	}

	function handlePreset(event) {
		selectedPreset = event.detail;
		prompt = prompt.trim() ? `${prompt.trim()} ${event.detail}.` : `${event.detail}.`;
	}

	function selectChange(event) {
		selectedChangeId = event.detail;
	}

	function updateChange(id, status) {
		changes = changes.map((change) => (change.id === id ? { ...change, status } : change));
	}

	function handleChangeAction(event) {
		const { id, action } = event.detail;
		selectedChangeId = id;

		if (action === 'regenerate') {
			updateChange(id, 'regenerating');
			schedule(() => updateChange(id, 'pending'), 850);
			return;
		}

		if (action === 'undo') {
			updateChange(id, 'pending');
			return;
		}

		updateChange(id, action === 'accept' ? 'accepted' : 'skipped');
		schedule(() => maybeAutoValidate(), 80);
	}

	function maybeAutoValidate() {
		const complete = changes.every((change) => change.status === 'accepted' || change.status === 'skipped');
		if (workflowState === 'reviewing' && complete) startValidationAndBuild();
	}

	function startValidationAndBuild() {
		clearTimers();
		workflowState = 'validating';
		buildProgress = 0;
		messages = [
			...messages,
			{
				id: newId(),
				role: 'assistant',
				text: 'All review decisions are complete. I am validating the pack now. This is a local UI simulation.',
				kind: 'validating'
			}
		];

		schedule(() => {
			workflowState = 'building';
			buildProgress = 8;
			messages = [
				...messages,
				{
					id: newId(),
					role: 'assistant',
					text: 'Validation passed. Simulating update.rpf build.',
					kind: 'building'
				}
			];
		}, 1050);

		[22, 44, 68, 86, 100].forEach((value, index) => {
			schedule(() => {
				buildProgress = value;
				if (value === 100) {
					workflowState = 'ready';
					packs = [
						{
							name: lastPrompt.length > 42 ? `${lastPrompt.slice(0, 42)}...` : lastPrompt,
							output: 'update.rpf',
							status: 'Ready to install'
						},
						...packs
					];
					messages = [
						...messages,
						{
							id: newId(),
							role: 'assistant',
							text: 'Prototype complete. The selected changes are validated and ready to install as update.rpf in the UI only.',
							kind: 'ready'
						}
					];
				}
			}, 1500 + index * 420);
		});
	}

	function restart() {
		clearTimers();
		prompt = '';
		selectedPreset = '';
		changes = [];
		selectedChangeId = '';
		workflowState = 'idle';
		buildProgress = 0;
		weaponViewMode = 'firstPerson';
		visualPreviewMode = 'comparison';
		previewCollapsed = false;
		messages = [];
		activeView = 'new';
		lastPrompt = '';
	}
</script>

<section class="rm-prototype" aria-label="Redux Maker prototype">
	<div class="app-window vercel-style">
		<div class:previewCollapsed class="app-body">
			<Sidebar
				{activeView}
				{packs}
				on:navigate={(event) => (activeView = event.detail)}
				on:newRedux={restart}
			/>

			<main class="main-pane">
				<ChatWorkspace
					{activeView}
					{messages}
					{changes}
					{selectedChangeId}
					{workflowState}
					{validationItems}
					{buildProgress}
					{permissionMode}
					{packs}
					{packTitle}
					on:selectChange={selectChange}
					on:changeAction={handleChangeAction}
					on:navigate={(event) => (activeView = event.detail)}
					on:newRedux={restart}
				/>

				{#if activeView === 'new'}
					<div class:docked={composerDocked} class:centered={!composerDocked} class="prompt-dock">
						<PromptComposer
							bind:prompt
							bind:permissionMode
							compact={composerDocked}
							{selectedPreset}
							on:preset={handlePreset}
							on:submit={runWorkflow}
						/>
					</div>
				{/if}
			</main>

			<PreviewPane
				{workflowState}
				{selectedChange}
				{weaponViewMode}
				{visualPreviewMode}
				{previewCollapsed}
				on:weaponView={(event) => (weaponViewMode = event.detail)}
				on:visualPreviewMode={(event) => (visualPreviewMode = event.detail)}
				on:toggleCollapse={() => (previewCollapsed = !previewCollapsed)}
				on:changeAction={handleChangeAction}
				on:restart={restart}
			/>
		</div>
	</div>
</section>

<style>
	.rm-prototype {
		height: 100%;
		min-height: 0;
		display: flex;
		background: #000;
	}

	.rm-prototype :global(.app-window) {
		width: 100%;
		height: 100%;
		min-height: 0;
	}

	.rm-prototype :global(.app-body) {
		height: 100%;
		min-height: 0;
	}

	.rm-prototype :global(.sidebar) {
		flex-basis: clamp(232px, 15vw, 280px);
	}

	.rm-prototype :global(.preview-pane) {
		flex-basis: clamp(400px, 32vw, 620px);
	}

	.rm-prototype :global(.app-body.previewCollapsed .main-pane) {
		flex: 1 1 auto;
	}

	.rm-prototype :global(.preview-reopen) {
		position: absolute;
		z-index: 20;
		right: 14px;
		top: 50%;
		display: inline-flex;
		align-items: center;
		gap: 7px;
		min-height: 34px;
		padding: 7px 10px;
		transform: translateY(-50%);
		border: 1px solid rgba(74, 158, 255, 0.24);
		border-radius: 999px;
		background: rgba(18, 42, 70, 0.88);
		color: var(--blue-2);
		box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
		cursor: pointer;
	}

	.rm-prototype :global(.preview-reopen:hover) {
		border-color: rgba(74, 158, 255, 0.5);
		background: rgba(24, 56, 94, 0.94);
		color: #eaf5ff;
	}

	.rm-prototype :global(.preview-pane.fullscreen) {
		position: fixed;
		inset: 0;
		z-index: 10000;
		flex-basis: auto;
		width: 100vw;
		height: 100vh;
		border: 0;
		box-shadow: none;
		background: #000;
	}

	.rm-prototype :global(.preview-pane.fullscreen .collapse-preview) {
		display: none;
	}

	.rm-prototype :global(.preview-pane.fullscreen .preview-toolbar.top-left) {
		top: 18px;
		left: 18px;
	}

	.rm-prototype :global(.preview-pane.fullscreen .preview-toolbar.top-right) {
		top: 18px;
		right: 18px;
	}

	.rm-prototype :global(.chat-workspace) {
		padding-top: 44px;
	}

	.rm-prototype :global(.chat-workspace.empty-state .chat-scroll) {
		padding-top: min(20vh, 170px);
	}

	.rm-prototype :global(.empty-chat) {
		max-width: 720px;
	}

	.rm-prototype :global(.prompt-dock.centered) {
		top: min(53vh, 520px);
		width: min(730px, calc(100% - 56px));
	}

	@media (max-width: 980px) {
		.rm-prototype :global(.app-window) {
			min-height: 100%;
		}

		.rm-prototype :global(.app-body) {
			grid-template-columns: 220px minmax(0, 1fr);
			grid-template-rows: minmax(620px, 1fr) 520px;
		}
	}
</style>
