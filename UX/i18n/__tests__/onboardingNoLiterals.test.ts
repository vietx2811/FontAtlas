import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// Mọi chuỗi hiển thị của onboarding / splash / bảng phím tắt phải qua t(): không còn tiếng Việt cứng trong mã (trừ tên ngôn ngữ "Tiếng Việt").
const FILES = [
  'components/onboarding/Onboarding.tsx', 'components/onboarding/Illustrations.tsx', 'components/onboarding/blueprint.tsx',
  'components/onboarding/ScanBlocks.tsx', 'components/onboarding/Outro.tsx', 'components/splash/Splash.tsx',
  'components/help/ShortcutsOverlay.tsx', 'components/help/ShortcutsHelp.tsx',
];
const VI = /[ạảãàáâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđĐ]/;
const strip = (src: string) => src.replace(/\/\*[\s\S]*?\*\//g, '').split('\n').map(l => l.replace(/(^|[^:'"`])\/\/.*$/, '$1')).join('\n');

describe('no hard-coded Vietnamese UI text', () => {
  for (const f of FILES) it(f, () => {
    const code = strip(readFileSync(f, 'utf8')).replace(/'Tiếng Việt'/g, '');
    // Ký tự mẫu của hình minh hoạ (một chữ trong dấu nháy: 'ê', ch="Ặ") là dữ liệu glyph, không phải chữ hiển thị.
    const bad = code.replace(/(['"])[^'"\s]\1/g, '').split('\n').filter(l => VI.test(l));
    expect(bad).toEqual([]);
  });
});
