import type { ONBOARDING_I18N } from '../onboarding';

export const EN_ONBOARDING: Record<keyof typeof ONBOARDING_I18N, string> = {
  'onb.step': 'Step {n}/{total}',
  'onb.keys.back': 'back',
  'onb.keys.next': 'next',
  'onb.keys.skip': 'skip',
  'onb.keys.aria': 'Shortcuts: ← goes back; → or Enter moves on in steps that need no confirmation; Esc on the first step skips the tour',
  'onb.legend': 'Figure caption, by number',
  'splash.label': 'Font Atlas',
  'splash.skip': 'Press any key to skip',
  'onb.outro.label': 'Opening your library',
  'onb.art.o1': 'Anatomy drawing of the letter Ặ: cap height, x-height, baseline, the breve and the dot below, with measurements',
  'onb.art.o2': 'Drawing of the letter ẫ moving from an old version’s frame to a new version’s frame',
  'onb.art.o3': 'A specimen tray with six cells, each receiving a letter as a folder is added',
  'onb.art.o5': 'A specimen label with a round stamp around the letter ự and a signature line',
  'onb.art.o6': 'The letter ê is scanned into a grid of sample points, then into a feature vector',
  'onb.art.o7': 'A toggle switch moves the letter ă from a source cell into the app window, first as a wireframe and then solid',
  'onb.art.end': 'The letters Aa on a baseline with a completion mark',
  'onb.art.o4': 'A nine-cell grid of letters being scanned by a scan line, with measurements',
  'onb.scan.done': 'Scan complete',
  'onb.scan.progress': 'Scan progress',
};
