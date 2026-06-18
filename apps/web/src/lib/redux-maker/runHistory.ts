// H2.2 — local HomeOps Redux Maker run history (metadata only).
//
// Stored in localStorage ONLY (never server-side). Holds run metadata so the
// Studio can list and reload prior bridge runs. Clearing history removes the
// index only — it never deletes the `.tmp/homeops-runs` run folders on disk.

export interface RunHistoryEntry {
	runId: string;
	prompt: string;
	presetId: string | null;
	provider: string;
	mode: string;
	status: string;
	outDir: string;
	mvpReportPath: string;
	readyToApply: boolean | null;
	applied: boolean;
	corpusContextAttached: boolean;
	contextRecordCount: number;
	contextCategories: string[];
	copiedRpfShaBefore: string | null;
	copiedRpfShaAfter: string | null;
	createdAt: string;
	updatedAt: string;
}

const STORAGE_KEY = 'homeops.reduxMaker.runHistory.v1';
const MAX_ENTRIES = 50;

export function loadRunHistory(): RunHistoryEntry[] {
	if (typeof localStorage === 'undefined') return [];
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return [];
		const parsed = JSON.parse(raw);
		return Array.isArray(parsed) ? (parsed as RunHistoryEntry[]) : [];
	} catch {
		return [];
	}
}

function persist(entries: RunHistoryEntry[]): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(entries.slice(0, MAX_ENTRIES)));
	} catch {
		/* ignore quota / disabled storage */
	}
}

/** Insert or update a run entry by runId, keeping newest first. */
export function upsertRun(entry: RunHistoryEntry): RunHistoryEntry[] {
	const existing = loadRunHistory().filter((e) => e.runId !== entry.runId);
	const next = [entry, ...existing].slice(0, MAX_ENTRIES);
	persist(next);
	return next;
}

/** Patch fields of an existing run (e.g. status, applied, SHA after). */
export function patchRun(runId: string, patch: Partial<RunHistoryEntry>): RunHistoryEntry[] {
	const entries = loadRunHistory();
	const idx = entries.findIndex((e) => e.runId === runId);
	if (idx === -1) return entries;
	entries[idx] = { ...entries[idx], ...patch, updatedAt: new Date().toISOString() };
	persist(entries);
	return entries;
}

/** Clear the history index only. Run folders on disk are untouched. */
export function clearRunHistory(): RunHistoryEntry[] {
	if (typeof localStorage !== 'undefined') {
		try {
			localStorage.removeItem(STORAGE_KEY);
		} catch {
			/* ignore */
		}
	}
	return [];
}
