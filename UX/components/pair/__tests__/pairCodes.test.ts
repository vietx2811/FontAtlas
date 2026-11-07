import { describe, expect, it } from 'vitest';
import { blockLabel, clashLabel, gateLabel, humanize, warningLabel } from '../pairCodes';

describe('humanize', () => {
  it('drops underscores and capitalises the first letter', () => {
    expect(humanize('brand_new_warning')).toBe('Brand new warning');
    expect(humanize('x')).toBe('X');
    expect(humanize('')).toBe('');
  });
});

describe('backend warning codes are all translated, never raw', () => {
  it.each([
    ['high_contrast', 'Tương phản nét cao'],
    ['weight_out_of_range', 'Độ đậm ngoài khoảng đọc tốt'],
    ['missing_regular_or_bold', 'Thiếu Regular hoặc Bold'],
    ['low_x_height', 'Nội dung có x-height thấp'],
  ])('%s → %s (with a gloss)', (code, text) => {
    const w = warningLabel(code);
    expect(w.text).toBe(text);
    expect(w.gloss).toBeTruthy();
    expect(w.text).not.toMatch(/_/);
  });
  it('an unknown code is humanized and never throws', () => {
    expect(warningLabel('some_future_code')).toEqual({ text: 'Some future code', gloss: null });
  });
});

describe('gate', () => {
  it('translates safe / controlled / experimental / blocked and humanizes strangers', () => {
    expect(gateLabel('safe')).toBe('An toàn');
    expect(gateLabel('blocked')).toBe('Bị chặn');
    expect(gateLabel('mystery_gate')).toBe('Mystery gate');
  });
});

describe('clashes', () => {
  it('axis:<axis> uses the Vietnamese axis name; global and skeleton have their own sentences', () => {
    expect(clashLabel('axis:contrast')).toBe('Tương phản nét');
    expect(clashLabel('axis:x_height')).toBe('Chiều cao x');
    expect(clashLabel('global')).toBe('Quá giống nhau');
    expect(clashLabel('skeleton')).toBe('Cùng nét, khác khung');
  });
  it('an unknown axis or kind falls back to a humanized name', () => {
    expect(clashLabel('axis:new_axis')).toBe('New axis');
    expect(clashLabel('weird_kind')).toBe('Weird kind');
  });
});

describe('block', () => {
  it('missing_vietnamese:<side>:<n> says which role lacks how many characters', () => {
    expect(blockLabel('missing_vietnamese:a:12')).toBe('Tiêu đề thiếu 12 ký tự tiếng Việt');
    expect(blockLabel('missing_vietnamese:b:3')).toBe('Nội dung thiếu 3 ký tự tiếng Việt');
  });
  it('an unknown block is humanized (colons become spaces)', () => {
    expect(blockLabel('other_block:x:1')).toBe('Other block x 1');
  });
});
