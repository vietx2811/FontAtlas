import { describe, expect, it } from 'vitest';
import { AUTO_I18N } from '../auto';
import { AUTO_EN } from '../en/auto';

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();

describe('auto-activation strings: the English table is complete and consistent', () => {
  it('has exactly the same keys as the Vietnamese table', () => {
    expect(Object.keys(AUTO_EN).sort()).toEqual(Object.keys(AUTO_I18N).sort());
  });
  it('uses the same {placeholders} in both languages', () => {
    for (const k of Object.keys(AUTO_I18N) as Array<keyof typeof AUTO_I18N>) expect(placeholders(AUTO_EN[k]), k).toEqual(placeholders(AUTO_I18N[k]));
  });
  it('has no Vietnamese-only diacritics in the English text', () => {
    for (const [k, v] of Object.entries(AUTO_EN)) expect(v, k).not.toMatch(/[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]/i);
  });
});
