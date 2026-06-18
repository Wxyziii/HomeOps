import { browser } from '$app/environment';

export const LEGACY_TUNNEL_SERVER_URL = 'http://127.0.0.1:8787';
export const DEFAULT_SERVER_URL = 'http://100.68.7.42:8787';
export const DEFAULT_TIMEOUT_MS = 5000;
export const API_TOKEN_KEY = 'homeops.apiToken';

export type HealthResponse = {
	ok: boolean;
	service: string;
};

export type ApiErrorResponse = {
	ok: false;
	error: string;
	code: string;
};

export type BackendSettingsResponse = {
	ok: true;
	config: {
		app_name: string;
		bind_host: string;
		bind_port: number;
		workspace_root: string;
		data_dir: string;
		logs_dir: string;
		allow_delete: boolean;
		max_parallel_jobs: number;
		allow_archive_extract: boolean;
		max_archive_extract_bytes: number;
		max_archive_entries: number;
		direct_tailscale_enabled: boolean;
		api_token_configured: boolean;
	};
	settings: Record<string, { value: string; updated_at: string }>;
	modules: Array<{
		id: string;
		name: string;
		slug: string;
		enabled: boolean;
		description: string | null;
		workspace_path: string | null;
	}>;
};

export type UpdateSettingsResponse = {
	ok: true;
	settings: BackendSettingsResponse['settings'];
};

export type WorkspaceStatusResponse = {
	ok: true;
	workspace_root: string;
	exists: boolean;
	writable: boolean;
	writable_reason: string | null;
	free_bytes: number | null;
	safety: {
		ok: boolean;
		message: string;
	};
	storage_roots: StorageRootStatus[];
};

export type StorageRootStatus = {
	id: string;
	label: string;
	path: string;
	exists: boolean;
	writable: boolean;
	writableReason: string | null;
	totalBytes: number | null;
	freeBytes: number | null;
	usedBytes: number | null;
	usagePercent: number | null;
};

export type FileKind = 'file' | 'directory' | 'symlink' | 'other';

export type FileEntry = {
	name: string;
	relativePath: string;
	kind: FileKind;
	sizeBytes: number;
	modifiedAt: string | null;
	readonly: boolean;
	extension: string | null;
	safeToOpen: boolean;
	warnings: string[];
};

export type FileListResponse = {
	ok: true;
	path: string;
	items: FileEntry[];
};

export type FileActionResponse = {
	ok: true;
	item: FileEntry;
};

export type UploadFilesResponse = {
	ok: true;
	destination: string;
	uploaded: Array<{
		name: string;
		relativePath: string;
		sizeBytes: number;
	}>;
	skipped: Array<{
		name: string;
		reason: string;
	}>;
};

export type DeleteFileResponse = {
	ok: true;
	trashedPath: string;
};

export type Job = {
	id: string;
	jobType: string;
	status: 'queued' | 'running' | 'finished' | 'failed' | 'cancelled';
	title: string;
	createdAt: string;
	startedAt: string | null;
	finishedAt: string | null;
	progress: number;
	error: string | null;
};

export type JobLog = {
	id: number;
	jobId: string;
	ts: string;
	line: string;
};

export type OperationLog = {
	id: number;
	ts: string;
	level: string;
	source: string;
	message: string;
};

export type JobsResponse = { ok: true; jobs: Job[] };
export type JobResponse = { ok: true; job: Job };
export type JobLogsResponse = { ok: true; jobId: string; logs: JobLog[] };
export type OperationLogsResponse = { ok: true; logs: OperationLog[] };
export type ExtractArchiveResponse = JobResponse;
export type HomeOpsStateBackup = {
	name: string;
	relativePath: string;
	sizeBytes: number;
	createdAt: string | null;
	containsSensitiveData: boolean;
};
export type HomeOpsStateBackupsResponse = { ok: true; backups: HomeOpsStateBackup[] };

export type ProjectStatus = 'active' | 'archived';

export type ProjectStats = {
	sizeBytes: number;
	fileCount: number;
	folderCount: number;
	lastModifiedAt: string | null;
	truncated: boolean;
};

export type Project = {
	id: string;
	name: string;
	rootId: string;
	rootLabel: string;
	relativePath: string;
	notes: string | null;
	status: ProjectStatus;
	tags: string[];
	pinned: boolean;
	createdAt: string;
	updatedAt: string;
	lastOpenedAt: string | null;
	folderExists: boolean;
	folderMissingReason: string | null;
	stats: ProjectStats | null;
};

export type ProjectsResponse = { ok: true; projects: Project[] };
export type ProjectResponse = { ok: true; project: Project };
export type CreateProjectRequest = {
	name: string;
	rootId: string;
	relativePath: string;
	notes?: string;
	status?: ProjectStatus;
	tags?: string[];
	pinned?: boolean;
	createFolder?: boolean;
	attachExisting?: boolean;
};
export type UpdateProjectRequest = Partial<CreateProjectRequest>;

export type ResourceSnapshotResponse = {
	ok: true;
	timestamp: string;
	summary: {
		hostname: string;
		os: string;
		uptimeSeconds: number;
		cpuUsagePercent: number;
		cpuCoreCount: number;
		loadAverage: [number, number, number];
		memoryTotalBytes: number;
		memoryUsedBytes: number;
		memoryFreeBytes: number;
		swapTotalBytes: number;
		swapUsedBytes: number;
	};
	disks: Array<{
		mountPoint: string;
		fileSystem: string;
		totalBytes: number;
		usedBytes: number;
		freeBytes: number;
		usagePercent: number;
	}>;
	workspace: {
		path: string;
		exists: boolean;
		totalBytes: number;
		usedBytes: number;
		freeBytes: number;
		usagePercent: number;
	};
	processes: Array<{
		pid: number;
		name: string;
		command: string;
		cpuUsagePercent: number;
		memoryBytes: number;
		status: string;
		user: string;
	}>;
};

// ---- H1.0 Smart Storage Pool ----------------------------------------------

export type SmartStoragePoolRoot = {
	rootId: string;
	label: string;
	path: string;
	role: string;
	totalBytes: number;
	freeBytes: number;
	usedBytes: number;
	reservedBytes: number;
	available: boolean;
	writable: boolean;
	warnings: string[];
};

export type PlacementPolicySummary = {
	largeFileThresholdBytes: number;
	mainReserveBytes: number;
	bulkReserveBytes: number;
	archiveExtensions: string[];
	reduxCorpusForcesBulk: boolean;
};

export type SmartStoragePool = {
	poolId: string;
	displayName: string;
	roots: SmartStoragePoolRoot[];
	totalBytes: number;
	freeBytes: number;
	usedBytes: number;
	health: string;
	warnings: string[];
	defaultRoot: string | null;
	largeFileRoot: string | null;
	metadataRoot: string | null;
	corpusRoot: string | null;
	policy: PlacementPolicySummary;
};

export type StoragePoolsResponse = { ok: true; pools: SmartStoragePool[] };
export type StoragePoolResponse = { ok: true; pool: SmartStoragePool };

export type PlacementIntent =
	| 'upload'
	| 'archive_extract'
	| 'corpus_inbox'
	| 'corpus_work'
	| 'dataset'
	| 'report'
	| 'generic';

export type PlacementRequest = {
	intent: PlacementIntent;
	relativePath: string;
	fileName?: string;
	sizeBytes?: number;
	extension?: string;
	preferredRootId?: string;
};

export type PlacementDecision = {
	allowed: boolean;
	selectedRootId: string | null;
	selectedRelativePath: string | null;
	selectedAbsolutePath: string | null;
	reason: string;
	warnings: string[];
	alternatives: string[];
	requiredFreeBytes: number;
	rootFreeBytes: number | null;
};

export type PlacementResponse = { ok: true; decision: PlacementDecision };

export type BootstrapFoldersResponse = {
	ok: true;
	rootId: string;
	created: string[];
	existing: string[];
	warnings: string[];
};

export async function getStoragePools(serverUrl: string): Promise<StoragePoolsResponse> {
	return apiFetch<StoragePoolsResponse>(serverUrl, '/api/storage/pools', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getServerStoragePool(serverUrl: string): Promise<StoragePoolResponse> {
	return apiFetch<StoragePoolResponse>(serverUrl, '/api/storage/pools/server', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function resolvePlacement(
	serverUrl: string,
	request: PlacementRequest
): Promise<PlacementResponse> {
	return apiFetch<PlacementResponse>(
		serverUrl,
		'/api/storage/pools/server/resolve-placement',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(request)
		},
		DEFAULT_TIMEOUT_MS
	);
}

export async function bootstrapStandardFolders(serverUrl: string): Promise<BootstrapFoldersResponse> {
	return apiFetch<BootstrapFoldersResponse>(
		serverUrl,
		'/api/storage/pools/server/bootstrap-standard-folders',
		{ method: 'POST' },
		DEFAULT_TIMEOUT_MS
	);
}

// ---- T2.2 Redux corpus job integration ------------------------------------

export type ReduxCorpusActiveJob = {
	id: string;
	status: string;
	title: string;
	progress: number;
};

export type ReduxCorpusLatestReport = {
	reportRoot: string;
	batchReportJson: string;
	batchReportMd: string | null;
	coverageReportMd: string | null;
	finishedAt: string | null;
	packagesTotal: number;
	packagesScanned: number;
	packagesQuarantined: number;
	datasetRecords: number;
};

export type ReduxCorpusStatus = {
	enabled: boolean;
	scannerConfigured: boolean;
	scannerPath: string;
	scannerVersion: string | null;
	corpusRoot: string;
	inboxRoot: string;
	inputRoot: string;
	workRoot: string;
	outRoot: string;
	datasetRoot: string;
	reportRoot: string;
	quarantineRoot: string;
	directoriesOk: boolean;
	diskFree: number | null;
	smartPoolRootId: string | null;
	activeJob: ReduxCorpusActiveJob | null;
	latestBatchReport: ReduxCorpusLatestReport | null;
};

export type ReduxCorpusDatasetSummary = {
	packagesTotal: number;
	packagesScanned: number;
	packagesReused: number;
	packagesSkippedDuplicate: number;
	packagesQuarantined: number;
	packagesFailed: number;
	datasetRecords: number;
	featureRecords: number;
	targetPatterns: number;
	categoryCoverage: Record<string, number>;
};

export type ReduxCorpusQuarantineEntry = {
	packageId: string;
	packageName: string;
	sourcePath: string;
	reason: string;
	detectedKind: string;
	sizeBytes: number;
};

export type ReduxCorpusQuarantineSummary = {
	total: number;
	entries: ReduxCorpusQuarantineEntry[];
};

export type ReduxCorpusStatusResponse = { ok: true; status: ReduxCorpusStatus };
export type ReduxCorpusReportResponse = { ok: true; report: ReduxCorpusLatestReport };
export type ReduxCorpusDatasetResponse = { ok: true; summary: ReduxCorpusDatasetSummary };
export type ReduxCorpusQuarantineResponse = { ok: true; quarantine: ReduxCorpusQuarantineSummary };

export async function getReduxCorpusStatus(serverUrl: string): Promise<ReduxCorpusStatusResponse> {
	return apiFetch<ReduxCorpusStatusResponse>(serverUrl, '/api/redux-corpus/status', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function bootstrapReduxCorpus(serverUrl: string): Promise<BootstrapFoldersResponse> {
	return apiFetch<BootstrapFoldersResponse>(serverUrl, '/api/redux-corpus/bootstrap', { method: 'POST' }, DEFAULT_TIMEOUT_MS);
}

export async function startReduxCorpusScan(serverUrl: string): Promise<JobResponse> {
	return apiFetch<JobResponse>(serverUrl, '/api/redux-corpus/scan', { method: 'POST' }, DEFAULT_TIMEOUT_MS);
}

export async function getReduxCorpusLatestReport(serverUrl: string): Promise<ReduxCorpusReportResponse> {
	return apiFetch<ReduxCorpusReportResponse>(serverUrl, '/api/redux-corpus/reports/latest', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getReduxCorpusDatasetSummary(serverUrl: string): Promise<ReduxCorpusDatasetResponse> {
	return apiFetch<ReduxCorpusDatasetResponse>(serverUrl, '/api/redux-corpus/dataset/summary', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getReduxCorpusQuarantine(serverUrl: string): Promise<ReduxCorpusQuarantineResponse> {
	return apiFetch<ReduxCorpusQuarantineResponse>(serverUrl, '/api/redux-corpus/quarantine', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

// H2.2 — read-only dataset records for prompt context retrieval.
export type ReduxCorpusDatasetRecord = {
	id: string;
	packageId: string;
	category: string;
	intent: string;
	targetPatterns: string[];
	fileTypes: string[];
	sourceEvidence: string[];
	safePatchPlanTemplateCandidates: string[];
	blockedReasons: string[];
	confidence: number;
	notes: string;
};

export type ReduxCorpusDatasetRecordsResult = {
	datasetFile: string;
	totalMatched: number;
	returned: number;
	limit: number;
	truncated: boolean;
	scannedLines: number;
	malformedSkipped: number;
	categories: string[];
	records: ReduxCorpusDatasetRecord[];
};

export type ReduxCorpusDatasetRecordsResponse = {
	ok: true;
	result: ReduxCorpusDatasetRecordsResult;
};

export async function getReduxCorpusDatasetRecords(
	serverUrl: string,
	filters: { category?: string; targetPattern?: string; packageId?: string; limit?: number } = {}
): Promise<ReduxCorpusDatasetRecordsResponse> {
	const qs = new URLSearchParams();
	if (filters.category?.trim()) qs.set('category', filters.category.trim());
	if (filters.targetPattern?.trim()) qs.set('targetPattern', filters.targetPattern.trim());
	if (filters.packageId?.trim()) qs.set('packageId', filters.packageId.trim());
	if (typeof filters.limit === 'number') qs.set('limit', String(filters.limit));
	const suffix = qs.toString() ? `?${qs.toString()}` : '';
	return apiFetch<ReduxCorpusDatasetRecordsResponse>(
		serverUrl,
		`/api/redux-corpus/dataset/records${suffix}`,
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export function normalizeServerUrl(value: string): string {
	const trimmed = value.trim().replace(/\/+$/, '');

	if (!trimmed) {
		throw new Error('Server URL is required.');
	}

	if (!/^https?:\/\//i.test(trimmed)) {
		throw new Error('Server URL must start with http:// or https://.');
	}

	try {
		const url = new URL(trimmed);
		if (!url.hostname) {
			throw new Error('Server URL must include a host.');
		}
		return url.toString().replace(/\/+$/, '');
	} catch {
		throw new Error('Server URL is not a valid URL.');
	}
}

export function getStoredApiToken(): string | null {
	if (!browser) return null;
	const token = localStorage.getItem(API_TOKEN_KEY)?.trim();
	return token || null;
}

export function hasStoredApiToken(): boolean {
	return getStoredApiToken() !== null;
}

export function saveStoredApiToken(value: string): void {
	if (!browser) return;
	const token = value.trim();
	if (!token) {
		throw new Error('API token cannot be empty.');
	}
	localStorage.setItem(API_TOKEN_KEY, token);
}

export function clearStoredApiToken(): void {
	if (!browser) return;
	localStorage.removeItem(API_TOKEN_KEY);
}

export async function getHealth(serverUrl: string, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<HealthResponse> {
	return apiFetch<HealthResponse>(serverUrl, '/health', { method: 'GET' }, timeoutMs);
}

export async function getSettings(
	serverUrl: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<BackendSettingsResponse> {
	return apiFetch<BackendSettingsResponse>(serverUrl, '/api/settings', { method: 'GET' }, timeoutMs);
}

export async function updateSettings(
	serverUrl: string,
	settings: Record<string, string | number | boolean>,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<UpdateSettingsResponse> {
	return apiFetch<UpdateSettingsResponse>(
		serverUrl,
		'/api/settings',
		{
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ settings })
		},
		timeoutMs
	);
}

export async function getWorkspaceStatus(
	serverUrl: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<WorkspaceStatusResponse> {
	return apiFetch<WorkspaceStatusResponse>(serverUrl, '/api/workspace', { method: 'GET' }, timeoutMs);
}

export async function listFiles(
	serverUrl: string,
	path = '',
	rootId = '',
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileListResponse> {
	const params = new URLSearchParams();
	if (path) params.set('path', path);
	if (rootId) params.set('rootId', rootId);
	const query = params.toString() ? `?${params.toString()}` : '';
	return apiFetch<FileListResponse>(serverUrl, `/api/files/list${query}`, { method: 'GET' }, timeoutMs);
}

export async function createFolder(
	serverUrl: string,
	path: string,
	rootId = '',
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return apiFetch<FileActionResponse>(
		serverUrl,
		'/api/files/create-folder',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ path, rootId: rootId || undefined })
		},
		timeoutMs
	);
}

export async function renameFile(
	serverUrl: string,
	from: string,
	to: string,
	rootId = '',
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return twoPathRequest(serverUrl, '/api/files/rename', from, to, rootId, timeoutMs);
}

export async function moveFile(
	serverUrl: string,
	from: string,
	to: string,
	rootId = '',
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return twoPathRequest(serverUrl, '/api/files/move', from, to, rootId, timeoutMs);
}

export async function deleteFile(
	serverUrl: string,
	path: string,
	rootId = '',
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<DeleteFileResponse> {
	return apiFetch<DeleteFileResponse>(
		serverUrl,
		'/api/files/delete',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ path, rootId: rootId || undefined })
		},
		timeoutMs
	);
}

export function downloadFileUrl(serverUrl: string, path: string, rootId = ''): string {
	const params = new URLSearchParams({ path });
	if (rootId) params.set('rootId', rootId);
	return `${normalizeServerUrl(serverUrl)}/api/files/download?${params.toString()}`;
}

export async function downloadFile(serverUrl: string, path: string, rootId = '', timeoutMs = 0): Promise<string> {
	const params = new URLSearchParams({ path });
	if (rootId) params.set('rootId', rootId);
	const response = await fetchResponse(
		serverUrl,
		`/api/files/download?${params.toString()}`,
		{ method: 'GET' },
		timeoutMs
	);
	const blob = await response.blob();
	const filename = filenameFromContentDisposition(response.headers.get('Content-Disposition')) ?? fallbackFilename(path);
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = filename;
	document.body.append(anchor);
	anchor.click();
	anchor.remove();
	URL.revokeObjectURL(url);
	return filename;
}

export async function uploadFiles(
	serverUrl: string,
	path: string,
	files: FileList,
	rootId = '',
	timeoutMs = 0
): Promise<UploadFilesResponse> {
	const form = new FormData();
	if (rootId) form.append('rootId', rootId);
	form.append('path', path);
	for (const file of Array.from(files)) {
		form.append('files', file, file.name);
	}

	return apiFetch<UploadFilesResponse>(
		serverUrl,
		'/api/files/upload',
		{
			method: 'POST',
			body: form
		},
		timeoutMs
	);
}

export function uploadFilesWithProgress(
	serverUrl: string,
	path: string,
	files: File[],
	rootId: string,
	onProgress: (uploadedBytes: number, totalBytes: number) => void
): Promise<UploadFilesResponse> {
	const form = new FormData();
	if (rootId) form.append('rootId', rootId);
	form.append('path', path);
	for (const file of files) {
		form.append('files', file, file.name);
	}

	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open('POST', `${normalizeServerUrl(serverUrl)}/api/files/upload`);

		const token = getStoredApiToken();
		if (token) {
			xhr.setRequestHeader('Authorization', `Bearer ${token}`);
		}

		xhr.upload.onprogress = (event) => {
			if (event.lengthComputable) {
				onProgress(event.loaded, event.total);
			}
		};

		xhr.onload = () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				try {
					resolve(JSON.parse(xhr.responseText) as UploadFilesResponse);
				} catch {
					reject(new Error('Upload completed but the server response was invalid.'));
				}
				return;
			}

			reject(new Error(readXhrApiError(xhr)));
		};

		xhr.onerror = () => reject(new Error('Network error while uploading file.'));
		xhr.onabort = () => reject(new Error('Upload cancelled.'));
		xhr.send(form);
	});
}

export async function listJobs(serverUrl: string): Promise<JobsResponse> {
	return apiFetch<JobsResponse>(serverUrl, '/api/jobs', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getJob(serverUrl: string, id: string): Promise<JobResponse> {
	return apiFetch<JobResponse>(serverUrl, `/api/jobs/${encodeURIComponent(id)}`, { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function getJobLogs(serverUrl: string, id: string, limit = 500): Promise<JobLogsResponse> {
	return apiFetch<JobLogsResponse>(
		serverUrl,
		`/api/jobs/${encodeURIComponent(id)}/logs?limit=${encodeURIComponent(String(limit))}`,
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function runTestSleepJob(serverUrl: string): Promise<JobResponse> {
	return apiFetch<JobResponse>(serverUrl, '/api/jobs/test-sleep', { method: 'POST' }, DEFAULT_TIMEOUT_MS);
}

export async function runTestFailJob(serverUrl: string): Promise<JobResponse> {
	return apiFetch<JobResponse>(serverUrl, '/api/jobs/test-fail', { method: 'POST' }, DEFAULT_TIMEOUT_MS);
}

export async function cancelJob(serverUrl: string, id: string): Promise<{ ok: true }> {
	return apiFetch<{ ok: true }>(
		serverUrl,
		`/api/jobs/${encodeURIComponent(id)}/cancel`,
		{ method: 'POST' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function listOperationLogs(serverUrl: string, limit = 100): Promise<OperationLogsResponse> {
	return apiFetch<OperationLogsResponse>(
		serverUrl,
		`/api/logs/operations?limit=${encodeURIComponent(String(limit))}`,
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function extractArchive(
	serverUrl: string,
	archivePath: string,
	destinationPath: string,
	rootId = ''
): Promise<ExtractArchiveResponse> {
	return apiFetch<ExtractArchiveResponse>(
		serverUrl,
		'/api/archives/extract',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ archivePath, destinationPath, rootId: rootId || undefined })
		},
		DEFAULT_TIMEOUT_MS
	);
}

export async function createHomeOpsStateBackup(serverUrl: string): Promise<JobResponse> {
	return apiFetch<JobResponse>(
		serverUrl,
		'/api/backups/homeops-state',
		{ method: 'POST' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function listHomeOpsStateBackups(serverUrl: string): Promise<HomeOpsStateBackupsResponse> {
	return apiFetch<HomeOpsStateBackupsResponse>(
		serverUrl,
		'/api/backups/homeops-state',
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function getResourceSnapshot(serverUrl: string): Promise<ResourceSnapshotResponse> {
	return apiFetch<ResourceSnapshotResponse>(
		serverUrl,
		'/api/resources/snapshot',
		{ method: 'GET' },
		DEFAULT_TIMEOUT_MS
	);
}

export async function listProjects(serverUrl: string): Promise<ProjectsResponse> {
	return apiFetch<ProjectsResponse>(serverUrl, '/api/projects', { method: 'GET' }, DEFAULT_TIMEOUT_MS);
}

export async function createProject(serverUrl: string, request: CreateProjectRequest): Promise<ProjectResponse> {
	return apiFetch<ProjectResponse>(
		serverUrl,
		'/api/projects',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(request)
		},
		DEFAULT_TIMEOUT_MS
	);
}

export async function updateProject(
	serverUrl: string,
	id: string,
	request: UpdateProjectRequest
): Promise<ProjectResponse> {
	return apiFetch<ProjectResponse>(
		serverUrl,
		`/api/projects/${encodeURIComponent(id)}`,
		{
			method: 'PATCH',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(request)
		},
		DEFAULT_TIMEOUT_MS
	);
}

export async function deleteProjectMetadata(serverUrl: string, id: string): Promise<{ ok: true; filesDeleted: false }> {
	return apiFetch<{ ok: true; filesDeleted: false }>(
		serverUrl,
		`/api/projects/${encodeURIComponent(id)}`,
		{ method: 'DELETE' },
		DEFAULT_TIMEOUT_MS
	);
}

async function twoPathRequest(
	serverUrl: string,
	url: string,
	from: string,
	to: string,
	rootId: string,
	timeoutMs: number
): Promise<FileActionResponse> {
	return apiFetch<FileActionResponse>(
		serverUrl,
		url,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ from, to, rootId: rootId || undefined })
		},
		timeoutMs
	);
}

export async function apiFetch<T>(
	serverUrl: string,
	path: string,
	init: RequestInit,
	timeoutMs: number
): Promise<T> {
	const response = await fetchResponse(serverUrl, path, init, timeoutMs);
	return (await response.json()) as T;
}

async function fetchResponse(
	serverUrl: string,
	path: string,
	init: RequestInit,
	timeoutMs: number
): Promise<Response> {
	const baseUrl = normalizeServerUrl(serverUrl);
	const controller = new AbortController();
	const timeout = timeoutMs > 0 ? setTimeout(() => controller.abort(), timeoutMs) : undefined;

	try {
		const response = await fetch(`${baseUrl}${path}`, {
			...init,
			headers: buildHeaders(path, init.headers),
			signal: controller.signal
		});

		if (!response.ok) {
			throw new Error(await readApiError(response));
		}

		return response;
	} catch (error) {
		if (error instanceof DOMException && error.name === 'AbortError') {
			throw new Error(`Connection timed out after ${Math.round(timeoutMs / 1000)} seconds.`);
		}

		if (error instanceof TypeError) {
			throw new Error('Network error while connecting to the server.');
		}

		throw error;
	} finally {
		if (timeout !== undefined) {
			clearTimeout(timeout);
		}
	}
}

function buildHeaders(path: string, initHeaders: HeadersInit | undefined): Headers {
	const headers = new Headers(initHeaders);
	if (!headers.has('Accept')) {
		headers.set('Accept', 'application/json');
	}

	const token = path.startsWith('/api/') ? getStoredApiToken() : null;
	if (token) {
		headers.set('Authorization', `Bearer ${token}`);
	}

	return headers;
}

async function readApiError(response: Response): Promise<string> {
	try {
		const body = (await response.json()) as Partial<ApiErrorResponse>;
		if (body.ok === false && typeof body.error === 'string') {
			return body.code ? `${body.error} (${body.code})` : body.error;
		}
	} catch {
		// Fall back to the HTTP status below.
	}
	return `Server returned HTTP ${response.status}.`;
}

function filenameFromContentDisposition(disposition: string | null): string | null {
	if (!disposition) return null;

	const utf8Match = disposition.match(/filename\*=UTF-8''([^;]+)/i);
	if (utf8Match?.[1]) {
		try {
			return decodeURIComponent(utf8Match[1].replace(/^"|"$/g, ''));
		} catch {
			return utf8Match[1].replace(/^"|"$/g, '');
		}
	}

	const match = disposition.match(/filename="?([^";]+)"?/i);
	return match?.[1] ?? null;
}

function fallbackFilename(path: string): string {
	return path.split('/').filter(Boolean).at(-1) ?? 'download';
}

function readXhrApiError(xhr: XMLHttpRequest): string {
	try {
		const body = JSON.parse(xhr.responseText) as Partial<ApiErrorResponse>;
		if (body.ok === false && typeof body.error === 'string') {
			return body.code ? `${body.error} (${body.code})` : body.error;
		}
	} catch {
		// Fall back to HTTP status below.
	}
	return `Server returned HTTP ${xhr.status}.`;
}
