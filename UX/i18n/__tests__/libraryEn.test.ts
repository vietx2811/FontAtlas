import { describe, expect, it } from 'vitest';
import { LIBRARY_I18N } from '../library';
import { LIBRARY_EN } from '../en/library.en';
import { hasKey } from '../index';

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();
const VI_ONLY = /[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]/i;

describe('library view options strings: English table is complete and consistent', () => {
  it('has exactly the same keys as the Vietnamese table, all registered', () => {
    expect(Object.keys(LIBRARY_EN).sort()).toEqual(Object.keys(LIBRARY_I18N).sort());
    for (const k of Object.keys(LIBRARY_I18N)) expect(hasKey(k), k).toBe(true);
  });
  it('uses the same {placeholders} in both languages', () => {
    for (const k of Object.keys(LIBRARY_I18N) as Array<keyof typeof LIBRARY_I18N>) expect(placeholders(LIBRARY_EN[k]), k).toEqual(placeholders(LIBRARY_I18N[k]));
  });
  it('has no Vietnamese diacritics in the English text', () => {
    for (const [k, v] of Object.entries(LIBRARY_EN)) expect(v, k).not.toMatch(VI_ONLY);
  });
});
