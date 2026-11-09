import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { TOOLTIP_DELAY_MS } from '../../ui/Tooltip';
import { notes } from '../../ui/__tests__/noteHelpers';
import { SAMPLE_VI, stackedIndexes } from '../CompareCells';
import { ComparePage } from '../ComparePage';
import * as cov from '../glyphCoverage';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
const initial = useUiStore.getState();
const rows = mockRows();
const faces = rows.slice(0, 2).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
beforeEach(() => { useUiStore.setState(initial, true); useUiStore.getState().openCompare(faces); cov.clearCoverageCache(); });
afterEach(() => vi.restoreAllMocks());
const mount = async () => { render(<ComparePage api={createMockApi()} faces={faces} />); await screen.findByRole('table'); };

describe('H7 · Vietnamese stacked diacritics', () => {
  it('stackedIndexes marks characters with two or more combining marks, not plain or single-mark ones', () => {
    expect([...stackedIndexes('ỮẶỆỖỞ ươ')]).toEqual([0, 1, 2, 3, 4]); // ư, ơ chỉ có một dấu (móc)
    expect([...stackedIndexes('ấ ở ặ a á')]).toEqual([0, 2, 4]);
  });
  it('the row is labelled Vietnamese with the stacked diacritics term and the Vietnamese gloss "dấu chồng"', async () => {
    await mount();
    const head = document.querySelector('[data-row="specimen.vi"]')!;
    expect(head.querySelector('b')!.textContent).toBe('Vietnamese');
    expect(head.querySelector('[data-term="vi_marks"]')!.textContent).toBe('stacked diacritics'.replace(/^./, c => c.toUpperCase()));
    expect(head.textContent).toContain('dấu chồng');
  });
  it('the term tooltip explains it with an SVG illustration', async () => {
    await mount();
    const el = document.querySelector('[data-row="specimen.vi"] [data-term="vi_marks"]') as HTMLElement;
    fireEvent.pointerEnter(el);
    await act(async () => { await new Promise(r => setTimeout(r, TOOLTIP_DELAY_MS + 60)); });
    const tip = screen.getByRole('tooltip');
    expect(tip.textContent).toMatch(/Dấu chồng/);
    expect(tip.querySelector('svg.fa-term-art')).toBeTruthy();
  });
  it('stacked characters in the sample are highlighted (data-stacked) without changing the text', async () => {
    await mount();
    const cell = document.querySelector('.cmp-vi')!;
    expect(cell.textContent).toBe(SAMPLE_VI);
    expect([...cell.querySelectorAll('[data-stacked="true"]')].map(e => e.textContent).join('')).toBe('ỮẶỆỖỞ');
  });
  it('a missing glyph in the Vietnamese sample is grey and reported through the note mark', async () => {
    vi.spyOn(cov.coverageProbe, 'detect').mockImplementation(async (_a, _f, text) => new Set([...text].filter(c => c === 'Ỗ')));
    await mount();
    await waitFor(() => expect(document.querySelectorAll('.cmp-vi .miss').length).toBeGreaterThan(0));
    expect(document.querySelector('.cmp-vi .miss')!.textContent).toBe('Ỗ');
    expect(notes('Thiếu: Ỗ').length).toBeGreaterThan(0);
  });
  it('typing in the specimen does not touch the Vietnamese row sample', async () => {
    await mount();
    fireEvent.change(screen.getAllByRole('textbox')[0], { target: { value: 'abc' } });
    expect(document.querySelector('.cmp-vi')!.textContent).toBe(SAMPLE_VI);
  });
});

describe('C · Vietnamese row styling', () => {
  it('stacked marks use a thin underline (no heavy fill) and the dashed red rule is gone', async () => {
    const { readFileSync } = await import('node:fs'); const { join } = await import('node:path');
    const css = readFileSync(join(process.cwd(), 'components/compare/compare.css'), 'utf8');
    const stk = css.split('\n').find(l => l.startsWith('.cmp-vi .stk {'))!;
    expect(stk).toMatch(/text-decoration: underline/); expect(stk).not.toMatch(/background/);
    const vi = css.split('\n').find(l => l.startsWith('.cmp-vi {'))!;
    expect(vi).not.toMatch(/dashed/);
    expect(css).not.toMatch(/\.cmp-vi\[data-overflow="true"\] \{[^}]*--warn/);
  });
});
