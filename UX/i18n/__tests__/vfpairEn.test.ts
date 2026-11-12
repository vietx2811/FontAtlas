import { describe, expect, it } from 'vitest';
import { VFPAIR_I18N, VFPAIR_I18N_EN } from '../vfpair';

const ph = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();

describe('VF/static pair strings exist in vi and en', () => {
  it('same keys, same placeholders, no empty text', () => {
    expect(Object.keys(VFPAIR_I18N_EN).sort()).toEqual(Object.keys(VFPAIR_I18N).sort());
    for (const [k, vi] of Object.entries(VFPAIR_I18N)) {
      expect(VFPAIR_I18N_EN[k as keyof typeof VFPAIR_I18N]).toBeTruthy();
      expect(ph(VFPAIR_I18N_EN[k as keyof typeof VFPAIR_I18N]), k).toBe(ph(vi));
    }
  });
});
