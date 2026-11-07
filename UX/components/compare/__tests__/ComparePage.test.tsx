import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { buildCompare, type CompareDto, type CompareMetric } from '../../../api/compare';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { ComparePage } from '../ComparePage';

const initial = useUiStore.getState();
const rows = mockRows();
const faces = (n: number) => rows.slice(0, n).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

const m = (value: number | null, o: Partial<CompareMetric> = {}): CompareMetric => ({ key: 'k', value, unit: 'em', quality: 'measured', reason: null, percentile: 50, ...o });
async function crafted(patch: (d: CompareDto) => void) {
  const api = createMockApi();
  const d = buildCompare(await Promise.all(faces(3).map(f => api.getFace(f, 'vi'))));
  patch(d);
  vi.spyOn(api, 'compareFaces').mockResolvedValue(d);
  return api;
}
const mount = async (api = createMockApi(), n = 3) => { render(<ComparePage api={api} faces={faces(n)} />); await screen.findByRole('table'); return api; };

describe('ComparePage', () => {
  it('shows one column per face with the first as the base', async () => {
    await mount();
    const heads = [...document.querySelectorAll('.cmp-tbl .ch')];
    expect(heads).toHaveLength(3);
    expect(heads[0].className).toContain('base');
    expect(heads[0].textContent).toContain('mốc');
    expect(screen.getByRole('heading', { name: /Compare · 3/ })).toBeTruthy();
  });

  it('shows loading, then an error with retry that loads again', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'compareFaces').mockRejectedValueOnce(new Error('boom'));
    render(<ComparePage api={api} faces={faces(2)} />);
    expect(screen.getByRole('status')).toBeTruthy();
    expect(await screen.findByRole('alert')).toHaveTextContent('boom');
    spy.mockRestore();
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    await screen.findByRole('table');
  });

  it('Same size / Match x-height: only Match scales the other columns to the base x-height', async () => {
    const api = await crafted(d => {
      d.faces[0].metrics.x_height_em = m(0.5); d.faces[1].metrics.x_height_em = m(0.25); d.faces[2].metrics.x_height_em = m(0.5);
    });
    await mount(api);
    const line = (i: number) => document.querySelectorAll<HTMLElement>('.cmp-line')[i].style.fontSize;
    expect(line(1)).toBe('30px');
    fireEvent.click(screen.getByRole('button', { name: 'Match x-height' }));
    expect(line(0)).toBe('30px'); expect(line(1)).toBe('60px'); expect(line(2)).toBe('30px');
    fireEvent.click(screen.getByRole('button', { name: 'Same size' }));
    expect(line(1)).toBe('30px');
  });

  it('metric cells show real values with Δ% against the base, a percentile and the Ước lượng flag', async () => {
    const api = await crafted(d => {
      d.faces[0].metrics.contrast = m(1.31, { unit: 'ratio', percentile: 8 });
      d.faces[1].metrics.contrast = m(4.89, { unit: 'ratio', percentile: 97 });
      d.faces[1].metrics.stress_deg = m(89, { unit: 'deg', quality: 'estimated', percentile: 96 });
      d.faces[0].metrics.stress_deg = m(null, { quality: 'failed', reason: 'không có nét đứng' });
    });
    await mount(api);
    const row = screen.getByText('contrast ⊥').closest('.row') as HTMLElement;
    expect(within(row).getByText('Δ +273%')).toBeTruthy();
    expect(within(row).getByRole('img', { name: 'Percentile 97' })).toBeTruthy();
    const stress = screen.getByText('stress').closest('.row') as HTMLElement;
    expect(within(stress).getAllByText('Ước lượng').length).toBeGreaterThan(0);
    expect(within(stress).getByText('Không đo được')).toBeTruthy();
    expect(within(stress).getByText('không có nét đứng')).toBeTruthy();
  });

  it('warns when stacked marks overflow and reports missing Vietnamese characters', async () => {
    const api = await crafted(d => {
      d.faces[1].vi = { ...d.faces[1].vi, stackedMarksOverflow: true, stackedMarksTopEm: 1.12, supports: true };
      d.faces[2].vi = { ...d.faces[2].vi, stackedMarksOverflow: false, supports: false, missing: 4 };
    });
    await mount(api);
    expect(screen.getByText(/Dấu tràn khung · 1,120 em/)).toBeTruthy();
    expect(screen.getByText('Thiếu 4 ký tự')).toBeTruthy();
  });

  it('Classification row: Uncertain shows the candidate, override shows Đã sửa, traits and usage are listed; license is a note only', async () => {
    const api = await crafted(d => {
      d.faces[1].classification = { ...d.faces[1].classification, class: 'uncertain', label: 'Uncertain', subclass: null, candidate: 'script', confidence: 0.5 };
      d.faces[2].classification = { ...d.faces[2].classification, source: 'override', autoClass: 'sans', traits: ['Rounded'], usage: ['display'] };
    });
    await mount(api);
    expect(screen.getByText('có lẽ là Script')).toBeTruthy();
    expect(screen.getByText('Đã sửa')).toBeTruthy();
    expect(screen.getByText('Rounded')).toBeTruthy();
    expect(screen.getAllByText('Chỉ ghi chú, không chặn')).toHaveLength(3);
  });

  it('the Overlay toggle hides and shows the overlay row', async () => {
    await mount();
    expect(document.querySelectorAll('.cmp-overlay')).toHaveLength(3);
    fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(document.querySelectorAll('.cmp-overlay')).toHaveLength(0);
  });

  it('Esc and the back button close the screen; Esc is left to dialogs', async () => {
    useUiStore.getState().openCompare(faces(3));
    await mount();
    const dlg = document.createElement('div'); dlg.setAttribute('role', 'dialog'); document.body.append(dlg);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().compare).not.toBeNull(); dlg.remove();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().compare).toBeNull();
    useUiStore.getState().openCompare(faces(3));
    fireEvent.click(screen.getByRole('button', { name: 'Quay lại thư viện' }));
    expect(useUiStore.getState().compare).toBeNull();
  });

  it('PDF is a disabled TODO; PNG fails loudly when canvas is unavailable (no fake "exported")', async () => {
    const api = await mount();
    const save = vi.spyOn(api, 'savePng');
    expect(screen.getByRole('button', { name: 'PDF' }).getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'PNG' }));
    await waitFor(() => expect(useNoticeStore.getState().notices[0]?.kind).toBe('error'));
    expect(save).not.toHaveBeenCalled();
  });

  it('PNG: a saved path is announced; a cancelled dialog (null) stays silent', async () => {
    const mod = await import('../exportPng');
    vi.spyOn(mod, 'renderCompareBlob').mockResolvedValue({ blob: new Blob(['x']), name: 'compare-a.png' });
    const api = await mount();
    vi.spyOn(api, 'savePng').mockResolvedValueOnce('/Users/me/Desktop/compare-a.png').mockResolvedValueOnce(null);
    fireEvent.click(screen.getByRole('button', { name: 'PNG' }));
    await waitFor(() => expect(useNoticeStore.getState().notices[0]?.text).toContain('compare-a.png'));
    useNoticeStore.setState({ notices: [] });
    fireEvent.click(screen.getByRole('button', { name: 'PNG' }));
    await waitFor(() => expect((screen.getByRole('button', { name: 'PNG' }) as HTMLButtonElement).disabled).toBe(false));
    expect(useNoticeStore.getState().notices).toHaveLength(0);
  });

  it('moves focus to the title on open and back to the previous element on close', async () => {
    const outside = document.createElement('button'); document.body.append(outside); outside.focus();
    const { unmount } = render(<ComparePage api={createMockApi()} faces={faces(2)} />);
    expect(document.activeElement).toBe(screen.getByRole('heading', { name: /Compare/ }));
    unmount();
    expect(document.activeElement).toBe(outside); outside.remove();
  });

  it('a font without Vietnamese is never shown as fitting; its sample is dimmed', async () => {
    const api = await crafted(d => { d.faces.forEach(f => { f.vi = { ...f.vi, supports: true, missing: 0, stackedMarksOverflow: false }; }); d.faces[1].vi = { ...d.faces[1].vi, supports: false }; });
    await mount(api);
    expect(screen.getByText('Không hỗ trợ tiếng Việt')).toBeTruthy();
    expect(document.querySelectorAll('.cmp-vi[data-unsupported="true"]')).toHaveLength(1);
    expect(screen.getAllByText('Dấu vừa khung')).toHaveLength(2);
  });

  it('Match x-height: a column without x-height keeps its size and says why; a missing base draws no x-height line', async () => {
    const api = await crafted(d => { d.faces[1].metrics.x_height_em = m(null, { quality: 'failed' }); d.faces[0].metrics.x_height_em = m(null, { quality: 'failed' }); });
    await mount(api);
    fireEvent.click(screen.getByRole('button', { name: 'Match x-height' }));
    expect(screen.getAllByText('Không đo được x-height').length).toBeGreaterThan(0);
    expect(document.querySelectorAll('.cmp-overlay line.xh')).toHaveLength(0);
    expect([...document.querySelectorAll<HTMLElement>('.cmp-line')].every(l => l.style.fontSize === '30px')).toBe(true);
  });

  it('clamps Match x-height scale to 0.5–2', async () => {
    const api = await crafted(d => { d.faces[0].metrics.x_height_em = m(0.9); d.faces[1].metrics.x_height_em = m(0.2); d.faces[2].metrics.x_height_em = m(0.9); });
    await mount(api);
    fireEvent.click(screen.getByRole('button', { name: 'Match x-height' }));
    expect(document.querySelectorAll<HTMLElement>('.cmp-line')[1].style.fontSize).toBe('60px');
  });

  it('a missing base measure shows "—" instead of a Δ, with an explanation', async () => {
    const api = await crafted(d => { d.faces[0].metrics.contrast = m(null, { quality: 'failed' }); d.faces[1].metrics.contrast = m(2, { unit: 'ratio' }); });
    await mount(api);
    const row = screen.getByText('contrast ⊥').closest('.row') as HTMLElement;
    expect(within(row).getAllByTitle('Mốc không đo được, không có Δ').length).toBeGreaterThan(0);
  });

  it('an unreadable face is reported with its reason while the others still compare', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'getFace').mockRejectedValueOnce(new Error('file gone'));
    render(<ComparePage api={api} faces={faces(3)} />);
    await screen.findByRole('table');
    expect(screen.getAllByRole('alert')[0]).toHaveTextContent('file gone');
    expect(document.querySelectorAll('.cmp-tbl .ch')).toHaveLength(2);
  });

  it('the table has real rows: every row has cells, row headers and column headers', async () => {
    await mount();
    const rowsEls = screen.getAllByRole('row');
    expect(rowsEls.length).toBeGreaterThan(10);
    expect(screen.getAllByRole('rowheader').length).toBeGreaterThan(8);
    expect(screen.getAllByRole('columnheader').length).toBeGreaterThan(3);
  });

  it('does not crash on values the UI has no translation for (unknown vote/source/trait)', async () => {
    const abril = rows.find(r => /Abril/.test(r.family))!;
    const api = createMockApi();
    render(<ComparePage api={api} faces={[...faces(1), { hashHex: abril.hashHex, faceIndex: abril.faceIndex }]} />);
    await screen.findByRole('table');
    expect(screen.getByText(/Quirky new trait/)).toBeTruthy();
  });
});
