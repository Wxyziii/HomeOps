export const DEFAULT_SERVER_URL = 'http://127.0.0.1:8787';
export const DEFAULT_TIMEOUT_MS = 5000;

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
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileListResponse> {
	const query = path ? `?path=${encodeURIComponent(path)}` : '';
	return apiFetch<FileListResponse>(serverUrl, `/api/files/list${query}`, { method: 'GET' }, timeoutMs);
}

export async function createFolder(
	serverUrl: string,
	path: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return apiFetch<FileActionResponse>(
		serverUrl,
		'/api/files/create-folder',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ path })
		},
		timeoutMs
	);
}

export async function renameFile(
	serverUrl: string,
	from: string,
	to: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return twoPathRequest(serverUrl, '/api/files/rename', from, to, timeoutMs);
}

export async function moveFile(
	serverUrl: string,
	from: string,
	to: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<FileActionResponse> {
	return twoPathRequest(serverUrl, '/api/files/move', from, to, timeoutMs);
}

export async function deleteFile(
	serverUrl: string,
	path: string,
	timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<{ ok: true }> {
	return apiFetch<{ ok: true }>(
		serverUrl,
		'/api/files/delete',
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ path })
		},
		timeoutMs
	);
}

export function downloadFileUrl(serverUrl: string, path: string): string {
	return `${normalizeServerUrl(serverUrl)}/api/files/download?path=${encodeURIComponent(path)}`;
}

export async function uploadFiles(
	serverUrl: string,
	path: string,
	files: FileList,
	timeoutMs = 0
): Promise<UploadFilesResponse> {
	const form = new FormData();
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

async function twoPathRequest(
	serverUrl: string,
	url: string,
	from: string,
	to: string,
	timeoutMs: number
): Promise<FileActionResponse> {
	return apiFetch<FileActionResponse>(
		serverUrl,
		url,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ from, to })
		},
		timeoutMs
	);
}

async function apiFetch<T>(
	serverUrl: string,
	path: string,
	init: RequestInit,
	timeoutMs: number
): Promise<T> {
	const baseUrl = normalizeServerUrl(serverUrl);
	const controller = new AbortController();
	const timeout = timeoutMs > 0 ? window.setTimeout(() => controller.abort(), timeoutMs) : undefined;

	try {
		const response = await fetch(`${baseUrl}${path}`, {
			...init,
			headers: { Accept: 'application/json', ...init.headers },
			signal: controller.signal
		});

		if (!response.ok) {
			throw new Error(await readApiError(response));
		}

		return (await response.json()) as T;
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
			window.clearTimeout(timeout);
		}
	}
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
