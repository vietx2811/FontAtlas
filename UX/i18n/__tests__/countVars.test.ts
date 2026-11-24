import { describe, expect, it } from 'vitest';
import { COUNT_VARS } from '..';
import * as viTables from '..';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/** Biến trong mẫu i18n mà tên gợi ý SỐ ĐẾM nhưng KHÔNG nằm trong COUNT_VARS phải được khai báo ở đây (người gọi tự định dạng: chuỗi, "<1", byte…). */
const PREFORMATTED = new Set(['size', 'chars', 'axes', 'pct', 'eta', 'filtered', 'mins', 'count', 'file']); // file = tên tệp (chuỗi)
const SUSPECT = /^(count|num|number|size|total|qty|amount|files?|faces|fonts|glyphs|chars|collections|sources|tags|favorites|mb|gb|kb|shown|done|missing|free|need|full|near|pairs|classes|axes|styles|items|entries|rows|cols)$/i;

const dirs = [join(__dirname, '..'), join(__dirname, '..', 'en')];
const varsOf = (): Map<string, string[]> => {
  const out = new Map<string, string[]>();
  for (const d of dirs) for (const f of readdirSync(d).filter(x => x.endsWith('.ts'))) {
    const src = readFileSync(join(d, f), 'utf8');
    for (const m of src.matchAll(/'([A-Za-z0-9_.]+)'\s*:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g)) {
      for (const v of (m[2] ?? m[3]).matchAll(/\{([A-Za-z0-9_]+)\}/g)) out.set(v[1], [...(out.get(v[1]) ?? []), m[1]]);
    }
  }
  return out;
};

describe('biến đếm trong mẫu i18n', () => {
  it('mọi {biến} có tên gợi ý số đếm đều nằm trong COUNT_VARS hoặc được khai báo "tự định dạng" (nhắc thêm khi có khoá mới)', () => {
    void viTables;
    const vars = varsOf();
    expect(vars.size).toBeGreaterThan(20); // bảo hiểm: bộ quét thật sự đọc được các bảng
    const missed = [...vars].filter(([v]) => SUSPECT.test(v) && !COUNT_VARS.has(v) && !PREFORMATTED.has(v)).map(([v, keys]) => `${v} (${keys.slice(0, 2).join(', ')})`);
    expect(missed, `Biến đếm chưa vào COUNT_VARS (i18n/index.ts) và chưa khai báo PREFORMATTED: ${missed.join('; ')}`).toEqual([]);
  });
  it('COUNT_VARS không chứa biến không phải số đếm (id, năm, px…)', () => {
    for (const v of ['id', 'y', 'x', 'px', 'ver', 'name', 'msg', 'code']) expect(COUNT_VARS.has(v)).toBe(false);
  });
});
