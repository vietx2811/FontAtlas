import { describe, expect, it } from 'vitest';
import { chooseApiKind } from '../select';

describe('chooseApiKind', () => {
  it('real commands inside Tauri, mock outside (browser, Vitest)', () => {
    expect(chooseApiKind({}, true)).toBe('tauri');
    expect(chooseApiKind({}, false)).toBe('mock');
  });
  it('VITE_FA_API overrides in both directions', () => {
    expect(chooseApiKind({ VITE_FA_API: 'mock' }, true)).toBe('mock');
    expect(chooseApiKind({ VITE_FA_API: 'tauri' }, false)).toBe('tauri');
    expect(chooseApiKind({ VITE_FA_API: 'other' }, true)).toBe('tauri');
  });
});
