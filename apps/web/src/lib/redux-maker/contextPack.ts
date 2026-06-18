// H2.2 — Redux Corpus context pack builder.
//
// Turns selected read-only dataset records into a compact, human-visible context
// block that the user can attach to a prompt before running the local
// ReduxScannerEngine. Contains ONLY metadata + safe evidence (categories, target
// path patterns, safe PatchPlan template candidates, blocked reasons, evidence
// summaries) — never raw binary content and never raw copyrighted asset bytes.

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
