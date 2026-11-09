import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { notes } from '../../ui/__tests__/noteHelpers';
import { ComparePage } from '../ComparePage';
import * as cov from '../glyphCoverage';
import { missingFrom, missingList } from '../glyphCoverage';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
const initial = useUiStore.getState();
const rows = mockRows();
const faces = rows.slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
beforeEach(() => { useUiStore.setState(initial, true); useUiStore.getState().openCompare(faces); cov.clearCoverageCache(); });
afterEach(() => vi.restoreAllMocks());
const mount = async () => { render(<ComparePage api={createMockApi()} faces={faces} />); await screen.findByRole('table'); };
const inputs = () => screen.getAllByRole('textbox') as HTMLInputElement[];

describe('H6 · specimen edited in place', () => {
  it('one editable field per column, showing the shared sample', async () => {
    await mount();
    expect(inputs()).toHaveLength(3);
    for (const i of inputs()) expect(i.value).toBe(useUiStore.getState().sample);
  });
  it('typing in one column changes the shared sample and every column follows', async () => {
    await mount();
    fireEvent.change(inputs()[1], { target: { value: 'Quạ đen' } });
    expect(useUiStore.getState().sample).toBe('Quạ đen');
    await waitFor(() => { for (const i of inputs()) expect(i.value).toBe('Quạ đen'); });
    expect([...document.querySelectorAll('.cmp-spec-view')].map(e => e.textContent)).toEqual(['Quạ đen', 'Quạ đen', 'Quạ đen']);
  });
  it('the field is transparent over a view layer in the same font (style unchanged)', async () => {
    await mount();
    const view = document.querySelector<HTMLElement>('.cmp-spec-view')!;
    expect(inputs()[0].style.fontSize).toBe(view.style.fontSize);
    expect(inputs()[0].style.fontFamily).toBe(view.style.fontFamily);
  });
  it('characters the font lacks are grey and the cell gets a note listing them', async () => {
    vi.spyOn(cov.coverageProbe, 'detect').mockImplementation(async (_f, text) => new Set([...text].filter(c => 'ấợ'.includes(c))));
    useUiStore.getState().setSample('Phấn ợ x');
    await mount();
    await waitFor(() => expect(document.querySelectorAll('.cmp-spec .miss').length).toBeGreaterThan(0));
    const miss = [...document.querySelectorAll('.cmp-spec-view')[0].querySelectorAll('.miss')].map(e => e.textContent);
    expect(miss).toEqual(['ấ', 'ợ']);
    expect(notes('Thiếu: ấ ợ').length).toBeGreaterThan(0);
  });
  it('no note and no grey when nothing is missing or coverage cannot be detected', async () => {
    vi.spyOn(cov.coverageProbe, 'detect').mockResolvedValue(null);
    await mount();
    await act(async () => { await new Promise(r => setTimeout(r, 20)); });
    expect(document.querySelectorAll('.cmp-spec .miss')).toHaveLength(0);
    expect(notes('Thiếu:')).toHaveLength(0);
  });
});

describe('glyph coverage', () => {
  it('a char is missing when monospace and serif fallbacks measure differently, present when they agree', () => {
    const ctx = { font: '', measureText(c: string) { const fb = /monospace/.test(this.font) ? 1 : 2; return { width: c === 'ấ' ? 10 * fb : 10 } as TextMetrics; } };
    expect([...missingFrom(ctx, 'F', ['a', 'ấ', ' '])!]).toEqual(['ấ']);
  });
  it('a throwing canvas yields null, not an exception', () => {
    expect(missingFrom({ font: '', measureText() { throw new Error('x'); } }, 'F', ['a'])).toBeNull();
  });
  it('missingList caps the list', () => {
    expect(missingList(new Set(['a', 'b', 'c']), 2)).toBe('a b …');
    expect(missingList(new Set(['a', 'b']))).toBe('a b');
  });
});
