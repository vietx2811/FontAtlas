import { describe, expect, it } from 'vitest';
import { REPLACE_I18N, REPLACE_I18N_EN } from '../replace';

const ph = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();

describe('replace-activation strings exist in vi and en', () => {
  it('same keys, same placeholders, no empty text', () => {
    expect(Object.keys(REPLACE_I18N_EN).sort()).toEqual(Object.keys(REPLACE_I18N).sort());
    for (const [k, vi] of Object.entries(REPLACE_I18N)) {
      expect(REPLACE_I18N_EN[k as keyof typeof REPLACE_I18N]).toBeTruthy();
      expect(ph(REPLACE_I18N_EN[k as keyof typeof REPLACE_I18N]), k).toBe(ph(vi));
    }
  });
});
