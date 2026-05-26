import { browser } from '$app/environment';
import {
	DEFAULT_SERVER_URL,
	LEGACY_TUNNEL_SERVER_URL,
	getHealth,
	normalizeServerUrl,
	type HealthResponse
} from '$lib/api/client';

const SERVER_URL_KEY = 'homeops.serverUrl';

export type ConnectionStatus = 'idle' | 'checking' | 'connected' | 'disconnected';

class ServerConnectionStore {
	serverUrl = $state(DEFAULT_SERVER_URL);
	connectionStatus = $state<ConnectionStatus>('idle');
	lastCheckedAt = $state<string | null>(null);
	lastError = $state<string | null>(null);
	lastHealth = $state<HealthResponse | null>(null);

	load() {
		if (!browser) return;
		const saved = localStorage.getItem(SERVER_URL_KEY);
		if (!saved) return;

		try {
			const normalized = normalizeServerUrl(saved);
			if (normalized === LEGACY_TUNNEL_SERVER_URL) {
				this.serverUrl = DEFAULT_SERVER_URL;
				localStorage.setItem(SERVER_URL_KEY, DEFAULT_SERVER_URL);
			} else {
				this.serverUrl = normalized;
			}
		} catch {
			this.serverUrl = DEFAULT_SERVER_URL;
		}
	}

	saveServerUrl(value: string) {
		const normalized = normalizeServerUrl(value);
		this.serverUrl = normalized;
		this.lastError = null;
		if (browser) {
			localStorage.setItem(SERVER_URL_KEY, normalized);
		}
		return normalized;
	}

	resetToDefault() {
		this.serverUrl = DEFAULT_SERVER_URL;
		this.lastError = null;
		if (browser) {
			localStorage.setItem(SERVER_URL_KEY, DEFAULT_SERVER_URL);
		}
		return DEFAULT_SERVER_URL;
	}

	async testConnection() {
		this.connectionStatus = 'checking';
		this.lastError = null;
		this.lastHealth = null;

		try {
			const health = await getHealth(this.serverUrl);
			this.connectionStatus = 'connected';
			this.lastHealth = health;
			this.lastCheckedAt = new Date().toISOString();
			return health;
		} catch (error) {
			this.connectionStatus = 'disconnected';
			this.lastCheckedAt = new Date().toISOString();
			this.lastError = error instanceof Error ? error.message : 'Unknown connection error.';
			throw error;
		}
	}
}

export const serverConnection = new ServerConnectionStore();
export { SERVER_URL_KEY };
