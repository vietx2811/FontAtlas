import { describe, expect, it } from 'vitest';
import { dateText, groupTitle, midEllipsis, revisionText, splitPath, tailDepth } from '../duplicatesFormat';

describe('midEllipsis', () => {
  it('keeps short paths and cuts the middle of long ones, keeping the file name end', () => {
    expect(midEllipsis('/a/b.ttf', 40)).toBe('/a/b.ttf');
    const out = midEllipsis('/Users/someone/Library/Fonts/Foundry/Family/Family-BoldItalic.ttf', 40);
    expect(out.length).toBeLessThanOrEqual(40);
    expect(out).toContain('…');
    expect(out.endsWith('Family-BoldItalic.ttf')).toBe(true);
    expect(out.startsWith('/Users')).toBe(true);
  });
  it('never splits a combining mark from its base letter', () => {
    const out = midEllipsis('/Ổ ngoài/' + 'Phở'.repeat(30) + '.ttf', 24);
    expect(out).not.toMatch(/̉$|^̉/);
  });
});

describe('dateText / revisionText', () => {
  it('formats unix seconds as a Vietnamese date and null as a dash', () => {
    expect(dateText(1_600_000_000)).toMatch(/2020/);
    expect(dateText(null)).toBe('—');
  });
  it('shows revision with up to 3 decimals and a comma, dash when unknown', () => {
    expect(revisionText(1.2029999494552612)).toBe('1,203');
    expect(revisionText(2)).toBe('2');
    expect(revisionText(null)).toBe('—');
  });
});

describe('groupTitle', () => {
  const face = (family: string, sub: string) => ({ row: { family, subfamily: sub } }) as never;
  it('same_file and formats name the font; versions use the PostScript name key', () => {
    expect(groupTitle({ kind: 'same_file', key: 'abc', faces: [face('Roboto', 'Bold')] } as never)).toBe('Roboto Bold');
    expect(groupTitle({ kind: 'versions', key: 'Roboto-Bold', faces: [face('Roboto', 'Bold')] } as never)).toBe('Roboto-Bold');
    expect(groupTitle({ kind: 'formats', key: 'Oswald|Regular|400|false', faces: [face('Oswald', 'Regular')] } as never)).toBe('Oswald Regular');
  });
  it('a strange kind or empty group still yields a readable title', () => {
    expect(groupTitle({ kind: 'weird_kind', key: 'some_key', faces: [] } as never)).toBe('some_key');
  });
});

describe('splitPath', () => {
  it('keeps "parent/file" as the always-visible tail and the rest as a shrinkable head', () => {
    expect(splitPath('/Users/me/QA/A/roboto.ttf')).toEqual({ head: '/Users/me/QA/', tail: 'A/roboto.ttf' });
    expect(splitPath('/Users/me/QA/C/roboto_copy.ttf')).toEqual({ head: '/Users/me/QA/', tail: 'C/roboto_copy.ttf' });
  });
  it('short or root-level paths have no head', () => {
    expect(splitPath('/roboto.ttf')).toEqual({ head: '', tail: '/roboto.ttf' });
    expect(splitPath('roboto.ttf')).toEqual({ head: '', tail: 'roboto.ttf' });
    expect(splitPath('/A/roboto.ttf')).toEqual({ head: '', tail: '/A/roboto.ttf' });
    expect(splitPath('/Users/me/QA/A/roboto.ttf', 3)).toEqual({ head: '/Users/me/', tail: 'QA/A/roboto.ttf' });
  });
  it('two same-file copies stay distinguishable by their tails', () => {
    expect(splitPath('/x/A/roboto.ttf').tail).not.toBe(splitPath('/x/C/roboto_copy.ttf').tail);
  });
});

describe('tailDepth', () => {
  it('is 2 when the copies already differ in "parent/file"', () => {
    expect(tailDepth(['/q/A/roboto.ttf', '/q/C/roboto_copy.ttf'])).toBe(2);
  });
  it('grows until same-named copies in same-named folders become distinguishable', () => {
    expect(tailDepth(['/Fonts/src1/X.ttf', '/Downloads/src1/X.ttf'])).toBe(3);
    expect(tailDepth(['/a/b/c/X.ttf', '/d/b/c/X.ttf'])).toBe(4);
  });
  it('ignores missing paths and stops at the longest path', () => {
    expect(tailDepth([null, '/a/X.ttf'])).toBe(2);
    expect(tailDepth(['/a/X.ttf', '/a/X.ttf'])).toBe(2);
  });
});
