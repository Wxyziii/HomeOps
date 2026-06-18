// H2.1 — Local Redux Maker bridge settings (desktop-only, stored locally).
//
// These are display/intent values for the UI. The ACTUAL fixed paths, SHA gate,
// and loopback enforcement live in the Rust Tauri bridge (redux_bridge.rs) and
// cannot be overridden from the UI in H2.1. A future H2.2 may add a validated
// path picker. Settings persist in localStorage only — never in server config.

export interface BridgeSettings {
  bridgeEnabled: boolean;
  /** Fixed known scanner binary (display only in H2.1; Rust owns the real path). */
  scannerBinaryPath: string;
  workspaceRoot: string;
  copiedRpfPath: string;
  expectedCopiedRpfSha: string;
  codewalkerUrl: string;
  provider: 'rule_based' | 'ollama_local' | 'lmstudio_local';
  allowLocalAi: boolean;
  localAiUrl: string;
  model: string;
}

export const DEFAULT_BRIDGE_SETTINGS: BridgeSettings = {
  bridgeEnabled: true,
  scannerBinaryPath:
    'C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo\\rpf_backend_rs\\target\\release\\rpf_backend_rs.exe',
  workspaceRoot: 'C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo',
  copiedRpfPath: 'C:\\Users\\Marcel\\Downloads\\ReduxScannerTest\\test-copy\\update.rpf',
  expectedCopiedRpfSha: '32d6aa5395c6b9e06c7375a9627407eb824e197ebcd9cb3c4f318629545396dc',
  codewalkerUrl: 'http://127.0.0.1:5560',
  provider: 'rule_based',
  allowLocalAi: false,
  localAiUrl: 'http://127.0.0.1:11434',
  model: ''
};

const STORAGE_KEY = 'homeops.reduxMaker.bridgeSettings.v1';

/** Cloud/public URLs are never allowed for local AI or CodeWalker. */
export function isLoopbackUrl(url: string): boolean {
  const u = (url ?? '').trim().toLowerCase();
  const afterScheme = u.includes('://') ? u.split('://')[1] : u;
  const authority = afterScheme.split('/')[0] ?? '';
  const host = authority.includes('@') ? authority.split('@').pop()! : authority;
  const hostNoPort = host.startsWith('[')
    ? host.slice(0, host.indexOf(']') + 1)
    : host.split(':')[0];
  return (
    hostNoPort === 'localhost' ||
    hostNoPort === '127.0.0.1' ||
    hostNoPort === '[::1]' ||
    hostNoPort === '::1' ||
    hostNoPort.startsWith('127.')
  );
}

export function loadBridgeSettings(): BridgeSettings {
  if (typeof localStorage === 'undefined') return { ...DEFAULT_BRIDGE_SETTINGS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_BRIDGE_SETTINGS };
    const parsed = JSON.parse(raw) as Partial<BridgeSettings>;
    // Paths and SHA are always reset to the fixed known values — the UI cannot
    // persist an arbitrary scanner binary / target archive in H2.1.
    return {
      ...DEFAULT_BRIDGE_SETTINGS,
      provider: parsed.provider ?? DEFAULT_BRIDGE_SETTINGS.provider,
      allowLocalAi: parsed.allowLocalAi ?? DEFAULT_BRIDGE_SETTINGS.allowLocalAi,
      localAiUrl:
        parsed.localAiUrl && isLoopbackUrl(parsed.localAiUrl)
          ? parsed.localAiUrl
          : DEFAULT_BRIDGE_SETTINGS.localAiUrl,
      model: parsed.model ?? DEFAULT_BRIDGE_SETTINGS.model,
      codewalkerUrl:
        parsed.codewalkerUrl && isLoopbackUrl(parsed.codewalkerUrl)
          ? parsed.codewalkerUrl
          : DEFAULT_BRIDGE_SETTINGS.codewalkerUrl
    };
  } catch {
    return { ...DEFAULT_BRIDGE_SETTINGS };
  }
}

export function saveBridgeSettings(settings: BridgeSettings): void {
  if (typeof localStorage === 'undefined') return;
  // Persist only the user-tunable, loopback-safe subset.
  const safe = {
    provider: settings.provider,
    allowLocalAi: settings.allowLocalAi,
    localAiUrl: isLoopbackUrl(settings.localAiUrl)
      ? settings.localAiUrl
      : DEFAULT_BRIDGE_SETTINGS.localAiUrl,
    model: settings.model,
    codewalkerUrl: isLoopbackUrl(settings.codewalkerUrl)
      ? settings.codewalkerUrl
      : DEFAULT_BRIDGE_SETTINGS.codewalkerUrl
  };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(safe));
  } catch {
    /* ignore quota / disabled storage */
  }
}
