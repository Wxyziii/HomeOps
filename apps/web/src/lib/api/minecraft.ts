import { DEFAULT_TIMEOUT_MS, apiFetch, type Job } from '$lib/api/client';

export type MinecraftStatus = {
	ok: true;
	enabled: boolean;
	serviceName: string;
	serviceState: string;
	running: boolean;
	uptimeSeconds: number | null;
	mainPid: number | null;
	memoryBytes: number | null;
	serverVersion: string | null;
	loader: string | null;
	worldName: string | null;
	maxPlayers: number | null;
	serverPort: number | null;
	motd: string | null;
	rconConfigured: boolean;
	onlinePlayers: number | null;
	onlinePlayerNames: string[] | null;
	playerDataSource: 'rcon' | 'rcon_error' | 'unavailable';
	modsTotal: number | null;
	modsDisabled: number | null;
	backupsTotal: number | null;
	serverRoot: string;
};

export type MinecraftConsole = {
	ok: true;
	source: string;
	lines: string[];
};

export type MinecraftFileEntry = {
	name: string;
	relativePath: string;
	kind: 'file' | 'directory' | 'symlink' | 'other';
	sizeBytes: number;
	modifiedAt: string | null;
	extension: string | null;
	editable: boolean;
	protected: boolean;
};

export type MinecraftFileList = {
	ok: true;
	path: string;
	items: MinecraftFileEntry[];
};

export type MinecraftFileRead = {
	ok: true;
	path: string;
	content: string;
	sizeBytes: number;
};

export type MinecraftConfigEntry = {
	key: string;
	value: string;
	redacted: boolean;
};

export type MinecraftConfigResponse = {
	ok: true;
	path: string;
	entries: MinecraftConfigEntry[];
	restartRequiredNote: string;
};

export type MinecraftPlayer = {
	name: string;
	uuid: string | null;
	whitelisted: boolean;
	op: boolean;
	opLevel: number | null;
	lastSeenExpires: string | null;
	banned: boolean;
	banReason: string | null;
};

export type MinecraftPlayersResponse = {
	ok: true;
	players: MinecraftPlayer[];
	onlinePlayers: string[] | null;
	onlineSource: 'rcon' | 'rcon_error' | 'unavailable';
	actionsAvailable: boolean;
};

export type PlayerAction =
	| 'op'
	| 'deop'
	| 'kick'
	| 'ban'
	| 'pardon'
	| 'whitelist_add'
	| 'whitelist_remove';

export type ManagedServer = {
	id: string;
	name: string;
	kind: 'main' | 'instance';
	unit: string;
	state: string;
	running: boolean;
	port: number | null;
	gameVersion: string | null;
	loader: string;
	memoryMb: number | null;
	motd: string | null;
	maxPlayers: number | null;
	path: string;
};

export type ManagedServersResponse = {
	ok: true;
	instanceSupport: boolean;
	instanceSupportReason: string | null;
	servers: ManagedServer[];
};

export type CreateServerRequest = {
	name: string;
	gameVersion: string;
	port: number;
	memoryMb: number;
	motd: string;
	maxPlayers: number;
	acceptEula: boolean;
};

export type ModrinthSearchHit = {
	projectId: string;
	slug: string;
	title: string;
	description: string;
	iconUrl: string | null;
	downloads: number;
	follows: number;
	author: string;
	categories: string[];
	latestVersion: string | null;
};

export type ModrinthSearchResponse = {
	ok: true;
	query: string;
	projectType: 'mod' | 'modpack';
	totalHits: number;
	offset: number;
	hits: ModrinthSearchHit[];
};

export type CurseForgeStatus = {
	ok: true;
	configured: boolean;
	reason: string | null;
};

export type MinecraftWorld = {
	name: string;
	active: boolean;
	sizeBytes: number;
	sizeTruncated: boolean;
	modifiedAt: string | null;
};

export type MinecraftBackup = {
	name: string;
	kind: 'archive' | 'directory';
	sizeBytes: number;
	createdAt: string | null;
	restorable: boolean;
};

export type MinecraftBackupsResponse = {
	ok: true;
	backupRoot: string;
	backups: MinecraftBackup[];
};

export type MinecraftMod = {
	fileName: string;
	displayName: string;
	enabled: boolean;
	sizeBytes: number;
	modifiedAt: string | null;
};

export type MinecraftModsResponse = {
	ok: true;
	mods: MinecraftMod[];
	installEnabled: boolean;
	gameVersion: string | null;
};

export type MinecraftModInstallResponse = {
	ok: true;
	fileName: string;
	version: string;
	sizeBytes: number;
};

type JobResponse = { ok: true; job: Job };

const JSON_HEADERS = { 'Content-Type': 'application/json' };

export async function getMinecraftStatus(serverUrl: string): Promise<MinecraftStatus> {
	return apiFetch(serverUrl, '/api/minecraft/status', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function minecraftServiceAction(
	serverUrl: string,
	action: 'start' | 'stop' | 'restart'
): Promise<{ ok: true; action: string; serviceState: string }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/service',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ action }) },
		60_000
	);
}

export async function getMinecraftConsole(
	serverUrl: string,
	lines = 200,
	server = 'main'
): Promise<MinecraftConsole> {
	const params = new URLSearchParams({ lines: String(lines), server });
	return apiFetch(
		serverUrl,
		`/api/minecraft/console/recent?${params.toString()}`,
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function getManagedServers(serverUrl: string): Promise<ManagedServersResponse> {
	return apiFetch(serverUrl, '/api/minecraft/servers', { method: 'GET' }, 15_000);
}

export async function createManagedServer(
	serverUrl: string,
	request: CreateServerRequest
): Promise<{ ok: true; job: Job }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/servers',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify(request) },
		30_000
	);
}

export async function managedServerAction(
	serverUrl: string,
	id: string,
	action: 'start' | 'stop' | 'restart'
): Promise<{ ok: true; server: string; action: string; state: string }> {
	return apiFetch(
		serverUrl,
		`/api/minecraft/servers/${encodeURIComponent(id)}/service`,
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ action }) },
		60_000
	);
}

export async function searchModrinth(
	serverUrl: string,
	query: string,
	type: 'mod' | 'modpack',
	offset = 0,
	gameVersion = ''
): Promise<ModrinthSearchResponse> {
	const params = new URLSearchParams({ query, type, offset: String(offset) });
	if (gameVersion) params.set('game_version', gameVersion);
	return apiFetch(
		serverUrl,
		`/api/minecraft/modrinth/search?${params.toString()}`,
		{ method: 'GET' },
		20_000
	);
}

export async function installModpackAsServer(
	serverUrl: string,
	project: string,
	request: CreateServerRequest
): Promise<{ ok: true; job: Job }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/modpacks/install',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ project, ...request }) },
		30_000
	);
}

export async function getCurseForgeStatus(serverUrl: string): Promise<CurseForgeStatus> {
	return apiFetch(serverUrl, '/api/minecraft/curseforge/status', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function minecraftPlayerAction(
	serverUrl: string,
	action: PlayerAction,
	player: string,
	reason?: string
): Promise<{ ok: true; response: string }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/players/action',
		{
			method: 'POST',
			headers: JSON_HEADERS,
			body: JSON.stringify({ action, player, reason: reason || undefined })
		},
		15_000
	);
}

export function formatDownloads(value: number): string {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
	return String(value);
}

export async function sendMinecraftCommand(
	serverUrl: string,
	command: string
): Promise<{ ok: true; response: string }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/console/command',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ command }) },
		15_000
	);
}

export async function listMinecraftFiles(serverUrl: string, path = ''): Promise<MinecraftFileList> {
	const query = path ? `?path=${encodeURIComponent(path)}` : '';
	return apiFetch(serverUrl, `/api/minecraft/files${query}`, { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function readMinecraftFile(serverUrl: string, path: string): Promise<MinecraftFileRead> {
	return apiFetch(
		serverUrl,
		`/api/minecraft/files/read?path=${encodeURIComponent(path)}`,
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function writeMinecraftFile(
	serverUrl: string,
	path: string,
	content: string
): Promise<MinecraftFileRead> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/files/write',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ path, content }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function renameMinecraftFile(
	serverUrl: string,
	from: string,
	to: string
): Promise<{ ok: true }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/files/rename',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ from, to }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function deleteMinecraftFile(
	serverUrl: string,
	path: string
): Promise<{ ok: true; trashedPath: string }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/files/delete',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ path }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function getMinecraftServerConfig(serverUrl: string): Promise<MinecraftConfigResponse> {
	return apiFetch(serverUrl, '/api/minecraft/config', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function updateMinecraftServerConfig(
	serverUrl: string,
	properties: Record<string, string>
): Promise<MinecraftConfigResponse> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/config',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ properties }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function getMinecraftPlayers(serverUrl: string): Promise<MinecraftPlayersResponse> {
	return apiFetch(serverUrl, '/api/minecraft/players', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getMinecraftWorlds(serverUrl: string): Promise<{ ok: true; worlds: MinecraftWorld[] }> {
	return apiFetch(serverUrl, '/api/minecraft/worlds', { method: 'GET' }, 15_000);
}

export async function getMinecraftBackups(serverUrl: string): Promise<MinecraftBackupsResponse> {
	return apiFetch(serverUrl, '/api/minecraft/backups', { method: 'GET' }, 15_000);
}

export async function createMinecraftBackup(serverUrl: string): Promise<JobResponse> {
	return apiFetch(serverUrl, '/api/minecraft/backups/create', { method: 'POST' }, DEFAULT_TIMEOUT_MS);
}

export async function restoreMinecraftBackup(serverUrl: string, name: string): Promise<JobResponse> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/backups/restore',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ name, confirm: true }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function getMinecraftMods(serverUrl: string): Promise<MinecraftModsResponse> {
	return apiFetch(serverUrl, '/api/minecraft/mods', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function setMinecraftModEnabled(
	serverUrl: string,
	fileName: string,
	enabled: boolean
): Promise<{ ok: true }> {
	return apiFetch(
		serverUrl,
		`/api/minecraft/mods/${enabled ? 'enable' : 'disable'}`,
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ fileName }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function deleteMinecraftMod(
	serverUrl: string,
	fileName: string
): Promise<{ ok: true; trashedPath: string }> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/mods/delete',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ fileName }) },
		DEFAULT_TIMEOUT_MS
	);
}

export async function installMinecraftMod(
	serverUrl: string,
	project: string
): Promise<MinecraftModInstallResponse> {
	return apiFetch(
		serverUrl,
		'/api/minecraft/mods/install',
		{ method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ project }) },
		120_000
	);
}

export function formatBytes(bytes: number | null | undefined): string {
	if (bytes === null || bytes === undefined) return 'Unknown';
	const units = ['B', 'KB', 'MB', 'GB', 'TB'];
	let size = bytes;
	let unit = 0;
	while (size >= 1024 && unit < units.length - 1) {
		size /= 1024;
		unit += 1;
	}
	return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function formatUptime(seconds: number | null): string {
	if (seconds === null || seconds < 0) return 'Unavailable';
	const days = Math.floor(seconds / 86_400);
	const hours = Math.floor((seconds % 86_400) / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	if (days > 0) return `${days}d ${hours}h ${minutes}m`;
	if (hours > 0) return `${hours}h ${minutes}m`;
	return `${minutes}m`;
}
