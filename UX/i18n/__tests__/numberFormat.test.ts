import { afterEach, describe, expect, it } from 'vitest';
import { formatDecimal, formatNumber, setLocale, t, tn } from '..';

afterEach(() => setLocale('vi'));

describe('định dạng số theo ngôn ngữ UI', () => {
  it('biến đếm trong t()/tn() tự có dấu ngăn nghìn: vi "1.234", en "1,234"', () => {
    setLocale('vi');
    expect(t('topbar.selected', { n: 1234 })).toBe('1.234 chọn');
    expect(t('topbar.selected', { n: 12 })).toBe('12 chọn');
    expect(t('group.selectAll.capSkip', { max: 2000, total: 12345, skipped: 1000 })).toMatch(/2\.000 font đầu tiên trong 12\.345 font.*1\.000 font/);
    setLocale('en');
    expect(t('topbar.selected', { n: 1234 })).toBe('1,234 selected');
    expect(tn('faces.nFonts', { n: 1234 })).toBe('1,234 fonts');
    expect(tn('faces.nFonts', { n: 1 })).toBe('1 font');
  });

  it('chuỗi đã định dạng, tên và biến không phải đếm (id, px…) giữ nguyên', () => {
    setLocale('vi');
    expect(t('topbar.selected', { n: formatNumber(1234) })).toBe('1.234 chọn');
    expect(t('act.why.case', { name: '2.000 Fonts' })).toBe('Bật cả Case «2.000 Fonts»');
    expect(t('meta.created', { y: 2024 })).toBe('Tạo 2024'); // năm không có dấu ngăn nghìn
  });

  it('formatDecimal: số chữ số cố định, không ngăn nghìn, dấu thập phân theo ngôn ngữ (khớp backend 0,85 / 0.85)', () => {
    setLocale('vi');
    expect(formatDecimal(0.85, 2)).toBe('0,85');
    expect(formatDecimal(1234.5, 2)).toBe('1234,50');
    expect(formatDecimal(0.5, 3)).toBe('0,500');
    setLocale('en');
    expect(formatDecimal(0.85, 2)).toBe('0.85');
    expect(formatDecimal(1234.5, 2)).toBe('1234.50');
    expect(formatDecimal(-0.25, 1)).toBe('-0.3');
  });

  it('mb/gb thập phân không bị làm tròn thành số nguyên: 1,5 / 1.5; số lớn vẫn có ngăn nghìn', () => {
    setLocale('vi');
    expect(formatNumber(1.5)).toBe('1,5');
    expect(t('set.vis.once', { mb: 1.5 })).toMatch(/1,5 MB/);
    expect(t('set.vis.once', { mb: 1500 })).toMatch(/1\.500 MB/);
    setLocale('en');
    expect(formatNumber(1.5)).toBe('1.5');
    expect(t('set.vis.once', { mb: 1.5 })).toMatch(/1\.5 MB/);
    expect(t('set.vis.once', { mb: 1500 })).toMatch(/1,500 MB/);
  });
});
