import { describe, expect, it } from 'vitest';
import { hasKey, t, type I18nKey } from '..';
import { classLabel, subclassLabel } from '../../components/main/classLabels';

describe('i18n', () => {
  it('missing key returns the key instead of throwing', () => {
    expect(t('class.no_such_value' as I18nKey)).toBe('class.no_such_value');
    expect(hasKey('class.serif')).toBe(true);
    expect(hasKey('class.usage_vote')).toBe(false);
  });
  it('class labels tolerate values the UI has not translated (real backend votes/traits)', () => {
    for (const v of ['display', 'usage', 'fat_face', 'inline_outline', 'brand_new']) expect(() => classLabel(v)).not.toThrow();
    expect(classLabel('usage')).toBe('usage');
    expect(() => subclassLabel('brand_new')).not.toThrow();
  });
});
