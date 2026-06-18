// H2.1 — Frontend client for the local Redux Maker Tauri bridge.
//
// Every call is desktop-only: in the browser/server build `isTauri()` is false
// and the call throws (the UI disables the controls and shows why). The Rust
// bridge re-validates everything; this layer is a typed convenience wrapper.

import { isLoopbackUrl } from './bridgeSettings';
import type { RunMode } from './presets';

export const APPLY_CONFIRM_PHRASE = 'APPLY_REDUX_MODULE_TO_COPIED_RPF';
export const ROLLBACK_CONFIRM_PHRASE = 'ROLLBACK_REDUX_MODULE_COPIED_RPF';

export interface BridgeStatus {
  available: boolean;
  reason: string | null;
  isDesktop: boolean;
  bridgeEnabled: boolean;
  scannerPath: string;
  scannerBinaryExists: boolean;
  workspaceRoot: string;
  workspaceRootExists: boolean;
  copiedRpfPath: string;
  copiedRpfExists: boolean;
  copiedRpfSha: string | null;
  expectedCopiedRpfSha: string;
  copiedRpfClean: boolean;
  codewalkerUrl: string;
  codewalkerLoopback: boolean;
  codewalkerReachable: boolean;
  localAiUrl: string | null;
  localAiReachable: boolean | null;
  applyConfirmPhrase: string;
  rollbackConfirmPhrase: string;
}

export interface StartRunInput {
  prompt: string;
  presetId?: string;
  provider?: 'rule_based' | 'ollama_local' | 'lmstudio_local';
  mode?: RunMode;
  allowLocalAi?: boolean;
  localAiUrl?: string;
  model?: string;
  codewalkerUrl?: string;
}

export interface StartRunOutput {
  runId: string;
  phase: string;
  outDir: string;
  mvpReportPath: string;
  commandPreview: string;
  startedAt: string;
}

export type RunPhase = 'queued' | 'running' | 'finished' | 'failed' | 'cancelled' | 'timed_out';

export interface RunStatus {
  runId: string;
  phase: RunPhase;
  startedAt: string;
  finishedAt: string | null;
  exitCode: number | null;
  stdoutTail: string;
  stderrTail: string;
  outDir: string;
  mvpReportPath: string;
  commandPreview: string;
  report: Record<string, unknown> | null;
  error: string | null;
  moduleSafe: boolean | null;
  readyToApply: boolean | null;
  applied: boolean;
  generatedAssets: number;
  replacementPlans: number;
  applyPlanPath: string | null;
  localModelCalled: boolean;
  fallbackUsed: boolean;
  cloudAiCalled: boolean;
  publicNetworkCall: boolean;
  forbiddenEndpointCallCount: number;
}

export interface ApplyInput {
  runId: string;
  confirmation: string;
  expectedCopiedRpfSha: string;
  codewalkerUrl?: string;
}

export interface ApplyOutput {
  status: string;
  applied: boolean;
  exitCode: number | null;
  applyReportPath: string;
  rollbackManifestPath: string | null;
  rollbackCommand: string | null;
  commandPreview: string;
  shaBefore: string;
  shaAfter: string | null;
  replaceRpfEntryCallCount: number;
  forbiddenEndpointCallCount: number;
  startedAt: string;
  finishedAt: string;
  stdout: string;
  stderr: string;
  report: Record<string, unknown> | null;
  error: string | null;
}

/** True when running inside the Tauri desktop webview. */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

const BROWSER_MSG =
  'Local bridge unavailable in browser mode — open the HomeOps desktop app.';

async function invoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error(BROWSER_MSG);
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export async function getBridgeStatus(opts?: {
  codewalkerUrl?: string;
  localAiUrl?: string;
  checkLocalAi?: boolean;
}): Promise<BridgeStatus> {
  return invoke<BridgeStatus>('redux_maker_bridge_status', { input: opts ?? {} });
}

export function isExactApplyConfirmation(text: string): boolean {
  return text === APPLY_CONFIRM_PHRASE;
}

export async function startRun(input: StartRunInput): Promise<StartRunOutput> {
  if (!input.prompt.trim()) throw new Error('prompt is required');
  if (
    input.provider &&
    input.provider !== 'rule_based' &&
    (!input.allowLocalAi || !input.localAiUrl || !isLoopbackUrl(input.localAiUrl))
  ) {
    throw new Error('local AI requires allowLocalAi + a loopback localAiUrl');
  }
  return invoke<StartRunOutput>('redux_maker_start_run', { input });
}

export async function getRunStatus(runId: string): Promise<RunStatus> {
  return invoke<RunStatus>('redux_maker_get_run_status', { input: { runId } });
}

export async function cancelRun(runId: string): Promise<RunStatus> {
  return invoke<RunStatus>('redux_maker_cancel_run', { input: { runId } });
}

export async function applyReviewedPlan(input: ApplyInput): Promise<ApplyOutput> {
  if (!isExactApplyConfirmation(input.confirmation)) {
    throw new Error(`exact confirmation required: ${APPLY_CONFIRM_PHRASE}`);
  }
  if (input.codewalkerUrl && !isLoopbackUrl(input.codewalkerUrl)) {
    throw new Error('CodeWalker URL must be loopback');
  }
  return invoke<ApplyOutput>('redux_maker_apply_reviewed_plan', { input });
}

export interface ApplyReadiness {
  enabled: boolean;
  reasons: string[];
}

/** Pure: decide whether Review & Apply may be enabled, with blocking reasons. */
export function applyReadiness(opts: {
  bridgeAvailable: boolean;
  copiedRpfClean: boolean;
  status: RunStatus | null;
  running: boolean;
  codewalkerLoopback: boolean;
}): ApplyReadiness {
  const reasons: string[] = [];
  if (!opts.bridgeAvailable) reasons.push('local bridge unavailable');
  if (!opts.status) reasons.push('no run loaded');
  if (opts.status && opts.status.phase !== 'finished') reasons.push('run not finished');
  if (opts.status && opts.status.readyToApply !== true) reasons.push('report is not readyToApply');
  if (opts.status && opts.status.applied) reasons.push('module already applied');
  if (opts.status && !opts.status.applyPlanPath) reasons.push('no apply plan in report');
  if (opts.running) reasons.push('a run/apply is in progress');
  if (!opts.copiedRpfClean) reasons.push('copied RPF is not clean');
  if (!opts.codewalkerLoopback) reasons.push('CodeWalker URL must be loopback');
  if (opts.status && opts.status.forbiddenEndpointCallCount > 0) {
    reasons.push('forbidden endpoint count > 0');
  }
  return { enabled: reasons.length === 0, reasons };
}
