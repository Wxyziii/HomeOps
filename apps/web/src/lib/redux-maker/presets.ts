// H2.1 — Redux Maker presets. Selecting a preset only fills the prompt composer
// and recommends a run mode; it never auto-runs and never bypasses bridge gates.

export type RunMode = 'planOnly' | 'applyReadyProof';

export interface MakerPreset {
  id: string;
  label: string;
  pill: string;
  prompt: string;
  mode: RunMode;
}

export const PRESETS: MakerPreset[] = [
  {
    id: 'cinematic_combat',
    label: 'Cinematic Combat',
    pill: '◐ Cinematic Combat',
    prompt:
      'make a cinematic combat Redux with darker nights, cleaner weather, performance opti, red vertical tracers, minimap Air bar, and hit glow — plan only, do not apply',
    mode: 'planOnly'
  },
  {
    id: 'cyberpunk_nights',
    label: 'Cyberpunk Nights',
    pill: '☾ Cyberpunk Nights',
    prompt:
      'make darker nights with stronger neon contrast and cleaner wet-road reflections using safe text config PatchPlans only — do not apply',
    mode: 'planOnly'
  },
  {
    id: 'weather_perf',
    label: 'Weather + Perf',
    pill: '☀ Weather + Perf',
    prompt:
      'make weather clearer, reduce fog, improve visual clarity, and add safe performance-oriented config adjustments — plan only',
    mode: 'planOnly'
  },
  {
    id: 'red_tracers',
    label: 'Red Tracers',
    pill: '▌ Red Tracers',
    prompt:
      'create red vertical top-to-bottom tracer assets and safe replacement planning for copied RPF review — plan only, do not apply',
    mode: 'planOnly'
  },
  {
    id: 'apply_ready_red_tracer',
    label: 'Apply-ready Red Tracer',
    pill: '▌ Apply-ready Red Tracer',
    prompt:
      'create apply-ready red vertical top-to-bottom tracer generated PNG source asset, build a real YTD, and create a copied-RPF replacement plan only; readyToApply must stay false unless generated asset, replacement plan, copied RPF SHA gate, loopback CodeWalker URL, and forbidden endpoint gates all pass',
    mode: 'applyReadyProof'
  }
];

export function presetById(id: string | undefined): MakerPreset | undefined {
  return PRESETS.find((p) => p.id === id);
}
