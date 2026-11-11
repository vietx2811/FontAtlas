import { describe, expect, it } from 'vitest';
import { ONBOARDING_I18N } from '../onboarding';
import { ONBOARDING_EN } from '../en/onboarding';

describe('onboarding English table', () => {
  it('has exactly the Vietnamese keys, none empty, and the same {placeholders}', () => {
    const vi = ONBOARDING_I18N as Record<string, string>;
    expect(Object.keys(ONBOARDING_EN).sort()).toEqual(Object.keys(vi).sort());
    const ph = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();
    for (const [k, en] of Object.entries(ONBOARDING_EN)) {
      expect(en.trim(), k).not.toBe('');
      expect(ph(en), k).toBe(ph(vi[k]));
    }
  });
  it('contains no Vietnamese diacritics (the art descriptions may quote the sample letters)', () => {
    for (const [k, en] of Object.entries(ONBOARDING_EN).filter(([k]) => !k.startsWith('onb.art.'))) expect(/[ăâđêôơưáàảãạấầẩẫậắằẳẵặéèẻẽẹếềểễệíìỉĩịóòỏõọốồổỗộớờởỡợúùủũụứừửữựýỳỷỹỵ]/i.test(en), k).toBe(false);
  });
});
