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

async function apiFetch<T>(
	serverUrl: string,
	path: string,
	init: RequestInit,
	timeoutMs: number
): Promise<T> {
	const baseUrl = normalizeServerUrl(serverUrl);
	const controller = new AbortController();
	const timeout = window.setTimeout(() => controller.abort(), timeoutMs);

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
		window.clearTimeout(timeout);
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
