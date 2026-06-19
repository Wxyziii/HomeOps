// H2.2 — Redux Corpus context pack builder.
//
// Turns selected read-only dataset records into a compact, human-visible context
// block that the user can attach to a prompt before running the local
// ReduxScannerEngine. Contains ONLY metadata + safe evidence (categories, target
// path patterns, safe PatchPlan template candidates, blocked reasons, evidence
// summaries) — never raw binary content and never raw copyrighted asset bytes.
//
// H2.3 adds a STRUCTURED JSON context pack (engine schema v1) alongside the
// visible text block, so the engine receives a typed input via --context-pack.

import type { ReduxCorpusDatasetRecord } from '$lib/api/client';

export const CONTEXT_OPEN = '[Redux Corpus Context]';
export const CONTEXT_CLOSE = '[/Redux Corpus Context]';

export interface ContextPack {
	text: string;
	recordCount: number;
	categories: string[];
	targetPatterns: string[];
	approxTokens: number;
}

const MAX_TARGETS_PER_RECORD = 6;
const MAX_EVIDENCE_PER_RECORD = 4;

/** Rough token estimate (~4 chars/token). Display-only. */
export function estimateTokens(text: string): number {
	return Math.ceil(text.length / 4);
}

function uniqueSorted(values: string[]): string[] {
	return Array.from(new Set(values.filter((v) => v && v.trim()))).sort();
}

/**
 * Build a context pack from selected dataset records. The output is a single
 * fenced block the prompt composer can show/clear; it never embeds binary data.
 */
export function buildContextPack(records: ReduxCorpusDatasetRecord[]): ContextPack {
	const cats = uniqueSorted(records.map((r) => r.category));
	const allTargets = uniqueSorted(records.flatMap((r) => r.targetPatterns));

	const lines: string[] = [CONTEXT_OPEN];
	lines.push(`Records: ${records.length}`);
	lines.push(`Categories: ${cats.join(', ') || '—'}`);
	lines.push('');

	for (const r of records) {
		lines.push(`• ${r.category} — ${r.intent || 'unclassified'}`);
		const targets = r.targetPatterns.slice(0, MAX_TARGETS_PER_RECORD);
		if (targets.length) {
			lines.push('  Common targets:');
			for (const t of targets) lines.push(`    - ${t}`);
		}
		if (r.safePatchPlanTemplateCandidates.length) {
			lines.push(`  Safe templates: ${r.safePatchPlanTemplateCandidates.join(', ')}`);
		}
		if (r.fileTypes.length) {
			lines.push(`  File types: ${r.fileTypes.join(', ')}`);
		}
		const evidence = r.sourceEvidence.slice(0, MAX_EVIDENCE_PER_RECORD);
		if (evidence.length) {
			lines.push(`  Evidence: ${evidence.join(', ')}${r.sourceEvidence.length > evidence.length ? ' …' : ''}`);
		}
		if (r.blockedReasons.length) {
			lines.push(`  Blocked/constraints: ${r.blockedReasons.join('; ')}`);
		}
		lines.push(`  Confidence: ${r.confidence.toFixed(2)}`);
	}

	lines.push('');
	lines.push('Safety:');
	lines.push('  - copied-RPF apply only; original GTA paths are never targeted');
	lines.push('  - metadata-only context; no raw binary or copyrighted asset bytes');
	lines.push('  - tracer orientation is vertical top-to-bottom');
	lines.push(CONTEXT_CLOSE);

	const text = lines.join('\n');
	return {
		text,
		recordCount: records.length,
		categories: cats,
		targetPatterns: allTargets,
		approxTokens: estimateTokens(text)
	};
}

/** Strip a previously-attached context block from a composed prompt. */
export function stripContextBlock(prompt: string): string {
	const start = prompt.indexOf(CONTEXT_OPEN);
	const end = prompt.indexOf(CONTEXT_CLOSE);
	if (start === -1 || end === -1 || end < start) return prompt;
	const before = prompt.slice(0, start);
	const after = prompt.slice(end + CONTEXT_CLOSE.length);
	return (before + after).replace(/\n{3,}/g, '\n\n').trim();
}

/**
 * Compose the final prompt sent to the scanner: user prompt + attached context
 * block, visually separated. The context is appended (not silently merged) so
 * the user can always see and clear it.
 */
export function composePrompt(userPrompt: string, contextText: string | null): string {
	const base = userPrompt.trim();
	if (!contextText) return base;
	return `${base}\n\n${contextText}`.trim();
}

// ── H2.3 — structured context pack (matches ReduxScannerEngine schema v1) ──────

/** Engine-side caps mirrored here so HomeOps never ships an oversized pack. */
export const STRUCTURED_SCHEMA_VERSION = 1;
export const STRUCTURED_MAX_RECORDS = 200;
export const STRUCTURED_MAX_BYTES = 256 * 1024;
const STRUCTURED_MAX_EVIDENCE_CHARS = 600;

export interface StructuredContextRecord {
	recordId: string;
	packageId: string;
	category: string;
	targetPatterns: string[];
	fileHints: string[];
	operationHints: string[];
	safeTemplateHints: string[];
	blockedReasonHints: string[];
	evidence: string;
}

export interface StructuredContextPack {
	schemaVersion: number;
	source: string;
	createdAt: string;
	query: { prompt: string; categories: string[]; targetPatterns: string[] };
	summary: {
		packageCount: number;
		recordCount: number;
		categoryCounts: Record<string, number>;
		targetPatternCounts: Record<string, number>;
	};
	records: StructuredContextRecord[];
	constraints: {
		noOriginalGtaPaths: true;
		copiedRpfOnly: true;
		noNativeRpfWrite: true;
		tracerOrientation: 'vertical-top-to-bottom';
		minimapAirSprite: 305;
	};
}

// Strips base64 data URIs from free-text evidence.
const DATA_URI_RE = /data:[^;\s]*;base64,[A-Za-z0-9+/=]+/gi;

/** Replace ASCII control chars (except space) with a space, by code point. */
function stripControlChars(s: string): string {
	let out = '';
	for (const ch of s) {
		const code = ch.codePointAt(0) ?? 0;
		out += code < 0x20 || code === 0x7f ? ' ' : ch;
	}
	return out;
}

/** A short, single-line evidence summary with NO raw binary / data URIs. */
function safeEvidence(record: ReduxCorpusDatasetRecord): string {
	const joined = stripControlChars(
		record.sourceEvidence.slice(0, MAX_EVIDENCE_PER_RECORD).join('; ').replace(DATA_URI_RE, '[blob-removed]')
	).trim();
	return joined.length > STRUCTURED_MAX_EVIDENCE_CHARS
		? `${joined.slice(0, STRUCTURED_MAX_EVIDENCE_CHARS)} …`
		: joined;
}

function countBy(values: string[]): Record<string, number> {
	const out: Record<string, number> = {};
	for (const v of values) {
		const k = (v ?? '').trim();
		if (k) out[k] = (out[k] ?? 0) + 1;
	}
	return out;
}

/**
 * Build the STRUCTURED context pack passed to ReduxScannerEngine via
 * `--context-pack`. Metadata + short evidence only; capped to the engine's
 * record limit; never embeds binary content or copyrighted asset bytes.
 */
export function buildStructuredContextPack(
	records: ReduxCorpusDatasetRecord[],
	userPrompt: string
): StructuredContextPack {
	const capped = records.slice(0, STRUCTURED_MAX_RECORDS);
	const cats = uniqueSorted(capped.map((r) => r.category));
	const allTargets = uniqueSorted(capped.flatMap((r) => r.targetPatterns));
	const packages = uniqueSorted(capped.map((r) => r.packageId));
	return {
		schemaVersion: STRUCTURED_SCHEMA_VERSION,
		source: 'homeops-redux-corpus',
		createdAt: new Date().toISOString(),
		query: { prompt: userPrompt.trim(), categories: cats, targetPatterns: allTargets },
		summary: {
			packageCount: packages.length,
			recordCount: capped.length,
			categoryCounts: countBy(capped.map((r) => r.category)),
			targetPatternCounts: countBy(capped.flatMap((r) => r.targetPatterns))
		},
		records: capped.map((r) => ({
			recordId: r.id,
			packageId: r.packageId,
			category: r.category,
			targetPatterns: r.targetPatterns.slice(0, MAX_TARGETS_PER_RECORD),
			fileHints: r.fileTypes.slice(0, MAX_TARGETS_PER_RECORD),
			operationHints: r.safePatchPlanTemplateCandidates.slice(0, MAX_TARGETS_PER_RECORD),
			safeTemplateHints: r.safePatchPlanTemplateCandidates.slice(0, MAX_TARGETS_PER_RECORD),
			blockedReasonHints: r.blockedReasons.slice(0, MAX_TARGETS_PER_RECORD),
			evidence: safeEvidence(r)
		})),
		constraints: {
			noOriginalGtaPaths: true,
			copiedRpfOnly: true,
			noNativeRpfWrite: true,
			tracerOrientation: 'vertical-top-to-bottom',
			minimapAirSprite: 305
		}
	};
}

/** Serialize a structured pack to the JSON string written into the run dir. */
export function serializeStructuredContextPack(pack: StructuredContextPack): string {
	return JSON.stringify(pack, null, 2);
}
