import { browser } from '$app/environment';

// H2.0 — Theme system foundation.
//
// Only `command-dark-current` is selectable today; the architecture is ready
// for additional themes (oled-dark, graphite, nord-dark, amber-terminal, …)
// which are listed as disabled placeholders and cannot be selected yet.

export type ThemeId =
	| 'command-dark-current'
	| 'oled-dark'
	| 'graphite'
	| 'nord-dark'
	| 'amber-terminal';

export type ThemeSwatch = {
	name: string;
	/** CSS color (or var reference) used for the preview chip. */
	value: string;
};

export type ThemeDef = {
	id: ThemeId;
	name: string;
	description: string;
	/** Only selectable themes can be activated; others render as disabled. */
	selectable: boolean;
	swatches: ThemeSwatch[];
};

export const THEME_STORAGE_KEY = 'homeops.theme';
export const DEFAULT_THEME_ID: ThemeId = 'command-dark-current';

export const THEMES: ThemeDef[] = [
	{
		id: 'command-dark-current',
		name: 'HomeOps Command Dark',
		description:
			'Warm dark command-center theme — warm grays, thin borders, compact density, and an orange accent. The HomeOps default.',
		selectable: true,
		swatches: [
			{ name: 'app', value: '#1a1917' },
			{ name: 'surface', value: '#1f1e1c' },
			{ name: 'border', value: '#3d3c39' },
			{ name: 'accent', value: '#da7756' },
			{ name: 'success', value: '#5a9e6f' },
			{ name: 'warning', value: '#b8943a' }
		]
	},
	// Placeholders only — not selectable yet (kept honest: clearly disabled).
	{
		id: 'oled-dark',
		name: 'OLED Dark',
		description: 'True-black variant for OLED panels. Planned.',
		selectable: false,
		swatches: [
			{ name: 'app', value: '#000000' },
			{ name: 'surface', value: '#0a0a0a' },
			{ name: 'accent', value: '#da7756' }
		]
	},
	{
		id: 'amber-terminal',
		name: 'Amber Terminal',
		description: 'Retro amber phosphor accent over warm dark. Planned.',
		selectable: false,
		swatches: [
			{ name: 'app', value: '#161310' },
			{ name: 'surface', value: '#1c1813' },
			{ name: 'accent', value: '#d8a23a' }
		]
	}
];

export function getTheme(id: ThemeId): ThemeDef | undefined {
	return THEMES.find((theme) => theme.id === id);
}

export function getStoredThemeId(): ThemeId {
	if (!browser) return DEFAULT_THEME_ID;
	const stored = localStorage.getItem(THEME_STORAGE_KEY);
	const theme = stored ? getTheme(stored as ThemeId) : undefined;
	// Only honor stored ids that map to a real, selectable theme.
	return theme && theme.selectable ? theme.id : DEFAULT_THEME_ID;
}

export function saveThemeId(id: ThemeId): void {
	if (!browser) return;
	const theme = getTheme(id);
	if (!theme || !theme.selectable) {
		throw new Error(`Theme '${id}' is not selectable.`);
	}
	localStorage.setItem(THEME_STORAGE_KEY, id);
	applyTheme(id);
}

/**
 * Apply a theme by tagging the document root. The current theme's tokens live
 * on `:root` in layout.css, so the default applies with no flash; future themes
 * override tokens via `[data-theme="…"]` selectors.
 */
export function applyTheme(id: ThemeId): void {
	if (!browser) return;
	document.documentElement.dataset.theme = id;
}
