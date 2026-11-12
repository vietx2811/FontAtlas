import { describe, expect, it } from 'vitest';
import { INTENTS, intentName, matchIntent, normalizeName } from '../intents';

describe('nhận ra ý đồ từ tên combo', () => {
  it('khớp tên chủ app hay dùng, không phân biệt hoa thường và dấu, vi + en', () => {
    const id = (s: string) => matchIntent(s)?.id ?? null;
    expect(id('Editorial')).toBe('editorial'); expect(id('TẠP CHÍ Bếp')).toBe('editorial'); expect(id('magazine 2')).toBe('editorial');
    expect(id('Tech startup')).toBe('ui'); expect(id('công nghệ')).toBe('ui'); expect(id('App')).toBe('ui');
    expect(id('Menu nhà hàng')).toBe('packaging'); expect(id('Thực đơn')).toBe('packaging'); expect(id('bao bì')).toBe('packaging');
    expect(id('Poster tết')).toBe('poster'); expect(id('Áp phích')).toBe('poster');
    expect(id('Brand book')).toBe('brand'); expect(id('Nhận diện')).toBe('brand');
  });
  it('gõ dở khớp tiền tố từ cuối (≥ 3 chữ); tên lạ không khớp', () => {
    expect(matchIntent('edit')?.id).toBe('editorial'); expect(matchIntent('Dự án 7')).toBeNull(); expect(matchIntent('')).toBeNull(); expect(matchIntent('ed')).toBeNull();
  });
  it('tên phụ dài hơn (cụ thể hơn) thắng khi nhiều ý đồ cùng khớp', () => {
    expect(matchIntent('brand book menu')?.id).toBe('brand'); // "brand book" (10) dài hơn "menu" (4)
  });
  it('mỗi ý đồ có tên (i18n), vai, phong cách; không có khoá "rule-breaking"', () => {
    expect(INTENTS).toHaveLength(5);
    for (const i of INTENTS) { expect(intentName(i)).not.toMatch(/^intent\./); expect(i.roles.length).toBeGreaterThan(0); expect(i.styles.length).toBeGreaterThan(0); expect(i.styles).not.toContain('rule_breaking'); }
    expect(normalizeName('  Đồ  ĂN ')).toBe('do an');
  });
});
