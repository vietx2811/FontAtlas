import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { setLocale } from '../../../i18n';
import { DupeLine, dupeHintOf, dupeHints } from '../dupeNames';

const row = (faceId: number, path: string | null, o: Partial<{ hashHex: string; faceIndex: number; subfamily: string | null }> = {}) =>
  ({ faceId, hashHex: o.hashHex ?? `h${faceId}`.padEnd(16, '0'), faceIndex: o.faceIndex ?? 0, family: 'Fira Sans', subfamily: o.subfamily === undefined ? 'Regular' : o.subfamily, path });

describe('dupeHints', () => {
  it('no hint when names differ, or when the same font appears twice', () => {
    expect(dupeHints([row(1, '/a/x.ttf'), row(2, '/a/y.ttf', { subfamily: 'Bold' })]).size).toBe(0);
    expect(dupeHints([row(1, '/a/x.ttf'), row(1, '/a/x.ttf')]).size).toBe(0);
    expect(dupeHints([null, undefined, row(1, '/a/x.ttf')]).size).toBe(0);
  });
  it('same name, different files → file name line + full path for the tooltip', () => {
    const a = row(1, '/Fonts/Google/FiraSans-Regular.ttf'); const b = row(2, '/Fonts/Old/FiraSans-Regular.otf');
    const h = dupeHints([a, b]);
    expect(dupeHintOf(h, a)).toEqual({ line: 'FiraSans-Regular.ttf', path: '/Fonts/Google/FiraSans-Regular.ttf' });
    expect(dupeHintOf(h, b)?.line).toBe('FiraSans-Regular.otf');
  });
  it('identical file names in different folders → parent folder is added; Windows paths work; no path → short hash; face index kept', () => {
    const a = row(1, 'C:\\F\\One\\Fira.ttc', { faceIndex: 0 }); const b = row(2, 'C:\\F\\Two\\Fira.ttc', { faceIndex: 1 });
    const h = dupeHints([a, b]);
    expect(dupeHintOf(h, a)?.line).toBe('One/Fira.ttc');
    expect(dupeHintOf(h, b)?.line).toBe('Two/Fira.ttc #1');
    const c = row(3, null); const d = row(4, '/x/y.ttf');
    expect(dupeHintOf(dupeHints([c, d]), c)).toEqual({ line: 'h3000000', path: null });
  });
});

describe('DupeLine', () => {
  it('renders nothing without hint; with hint shows the line and an accessible full path (vi + en)', () => {
    const { container, rerender } = render(<DupeLine hint={undefined} />);
    expect(container).toBeEmptyDOMElement();
    rerender(<DupeLine hint={{ line: 'a.ttf', path: '/x/a.ttf' }} />);
    expect(screen.getByTestId('dupe-line')).toHaveTextContent('a.ttf');
    expect(screen.getByTestId('dupe-line')).toHaveAttribute('aria-label', 'Tệp: /x/a.ttf');
    setLocale('en'); rerender(<DupeLine hint={{ line: 'a.ttf', path: '/x/a.ttf' }} />);
    expect(screen.getByTestId('dupe-line')).toHaveAttribute('aria-label', 'File: /x/a.ttf');
    setLocale('vi');
  });
});
