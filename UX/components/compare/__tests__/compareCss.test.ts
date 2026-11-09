import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const css = readFileSync(join(process.cwd(), 'components/compare/compare.css'), 'utf8');
const rule = (sel: string): string => css.split('\n').find(l => l.startsWith(`${sel} {`)) ?? '';

describe('H1 · cột property neo ngang', () => {
  it('row header is sticky at left:0 with a solid background and a divider', () => {
    const r = rule('.cmp-tbl .rh');
    expect(r).toMatch(/position: sticky/);
    expect(r).toMatch(/left: 0/);
    expect(r).toMatch(/background: var\(--card\)/);
    expect(r).toMatch(/border-right: 1px solid var\(--border\)/);
  });
  it('the table does not clip with overflow:hidden (that would turn it into the scroll container and break sticky)', () => {
    const r = rule('.cmp-tbl');
    expect(r).not.toMatch(/overflow: hidden/);
    expect(r).toMatch(/overflow: clip/);
  });
});
