import { afterEach, describe, expect, it } from 'vitest';
import { setLocale } from '../locale';
import { SAMPLE_DEFAULT, SAMPLE_LEGACY, defaultSample, isDefaultSample } from '../sample';
import { useUiStore } from '../../store/ui';

afterEach(() => { setLocale('vi'); useUiStore.setState({ sample: defaultSample('vi') }); });

describe('default sample text', () => {
  it('is neutral (no F&B), keeps stacked Vietnamese diacritics in both languages', () => {
    for (const s of Object.values(SAMPLE_DEFAULT)) { expect(s).not.toMatch(/phở|bò/i); expect(s).toMatch(/[ềếệểễ]|ệ|ấ|Ấ|ữ/); }
    expect(defaultSample('vi')).toBe('Chữ Việt đẹp — Ấn tượng');
    expect(defaultSample('en')).toBe('Typeface · Chữ Việt');
  });
  it('the old default and either new default count as "not changed by the user"; typed text does not', () => {
    for (const s of [SAMPLE_LEGACY, SAMPLE_DEFAULT.vi, SAMPLE_DEFAULT.en]) expect(isDefaultSample(s)).toBe(true);
    for (const s of ['Bún chả', '', 'Typeface']) expect(isDefaultSample(s)).toBe(false);
  });
  it('the store follows the UI language while the sample is still a default, and never overwrites typed text', () => {
    useUiStore.setState({ sample: defaultSample('vi') });
    setLocale('en'); expect(useUiStore.getState().sample).toBe(SAMPLE_DEFAULT.en);
    setLocale('vi'); expect(useUiStore.getState().sample).toBe(SAMPLE_DEFAULT.vi);
    useUiStore.setState({ sample: SAMPLE_LEGACY });
    setLocale('en'); expect(useUiStore.getState().sample).toBe(SAMPLE_DEFAULT.en);
    useUiStore.setState({ sample: 'Bún chả' });
    setLocale('vi'); expect(useUiStore.getState().sample).toBe('Bún chả');
  });
});
