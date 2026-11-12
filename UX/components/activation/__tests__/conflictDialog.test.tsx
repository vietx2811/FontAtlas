import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ConflictDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useActivationQueue } from '../../../store/activationFlow';
import { useUiStore } from '../../../store/ui';
import { ActivationDialogHost } from '../ActivationDialogHost';
import { ConflictDialog } from '../ActivationDialogs';
import { groupByFamily, groupNote, isSameFile, ownerNote, toItems } from '../conflictModel';
import { useActivationFlow } from '../useActivationFlow';

const c = (over: Partial<ConflictDto> = {}): ConflictDto => ({ postscriptName: 'X-Regular', existingPath: '/Users/u/Library/Fonts/FontBase/BT/x.ttf', owner: 'user_installed', sameFile: false, replaceTier: 'protected', variable: false, existingVariable: null, ...over });
const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('ownerNote: who installed it, in plain words', () => {
  it('a folder under ~/Library/Fonts/<App>/ names the app; plain owners fall back to the owner kind', () => {
    expect(ownerNote(c())).toBe('do FontBase cài');
    expect(ownerNote(c({ existingPath: '~/Library/Fonts/Adobe/a.otf' }))).toBe('do Adobe cài');
    expect(ownerNote(c({ existingPath: '/Users/u/Library/Fonts/Inter.ttf' }))).toBe('bạn đã cài');
    expect(ownerNote(c({ existingPath: '/Library/Fonts/Arial.ttf', owner: 'system' }))).toBe('của hệ thống'); // /Library, không phải ~/Library
    expect(ownerNote(c({ existingPath: '/x/y.ttf', owner: 'right_font' }))).toBe('do RightFont cài');
    expect(ownerNote(c({ existingPath: '/x/y.ttf', owner: 'other' }))).toBe('một ứng dụng khác');
  });
});

describe('grouping by family', () => {
  it('12 faces of 2 families become 2 rows with style lists and de-duplicated conflicts', () => {
    const items = [
      ...Array.from({ length: 8 }, (_, i) => ({ family: 'BT Annam Neue', style: `S${i}`, conflicts: [c({ postscriptName: `A-${i}` })] })),
      ...Array.from({ length: 4 }, (_, i) => ({ family: 'BT Absinotte', style: `T${i}`, conflicts: [c({ postscriptName: `B-${i}`, existingPath: '/Users/u/Library/Fonts/FontBase/BT/same.ttf' })] })),
    ];
    const g = groupByFamily(items);
    expect(g.map(x => [x.family, x.styles.length])).toEqual([['BT Absinotte', 4], ['BT Annam Neue', 8]]);
    expect(groupNote(g[0]).split('\n').length).toBeGreaterThanOrEqual(1);
  });
  it('the note lists at most 3 places then "+N"', () => {
    const conflicts = Array.from({ length: 5 }, (_, i) => c({ existingPath: `/Users/u/Library/Fonts/App${i}/x.ttf`, postscriptName: `P${i}` }));
    const note = groupNote({ family: 'F', styles: ['R'], conflicts });
    expect(note.split('\n')).toHaveLength(4);
    expect(note).toContain('+2 nơi khác');
  });
  it('toItems names faces from the library, falling back to the PostScript name', () => {
    const rows = [{ hashHex: 'aa', faceIndex: 0, family: 'Fam', subfamily: 'Bold' }] as never;
    const items = toItems(rows, [{ hashHex: 'aa', faceIndex: 0 }, { hashHex: 'bb', faceIndex: 0 }], [[c()], [c({ postscriptName: 'Other-Italic' })]]);
    expect(items.map(i => [i.family, i.style])).toEqual([['Fam', 'Bold'], ['Other', 'Italic']]);
  });
  it('isSameFile needs every conflict to be the same file', () => {
    expect(isSameFile([])).toBe(false);
    expect(isSameFile([c({ sameFile: true })])).toBe(true);
    expect(isSameFile([c({ sameFile: true }), c()])).toBe(false);
  });
});

describe('ConflictDialog', () => {
  const items = [
    ...Array.from({ length: 8 }, (_, i) => ({ family: 'BT Annam Neue', style: `Style ${i}`, conflicts: [c({ postscriptName: `A-${i}` })] })),
    ...Array.from({ length: 4 }, (_, i) => ({ family: 'BT Absinotte', style: `Kiểu ${i}`, conflicts: [c({ postscriptName: `B-${i}` })] })),
  ];
  it('short title, one-line explanation, one row per family; paths only in tooltips; styles expand on demand', () => {
    render(<ConflictDialog items={items} total={12} onCancel={() => {}} onConfirm={() => {}} />);
    const dlg = screen.getByRole('dialog', { name: '12 font đã có trên máy' });
    expect(within(dlg).getAllByRole('button', { name: /Xem các kiểu của/ })).toHaveLength(2);
    expect(dlg.textContent).toContain('BT Annam Neue · 8 kiểu');
    expect(dlg.textContent).toContain('BT Absinotte · 4 kiểu');
    expect(dlg.textContent).not.toMatch(/\/Users\/|FontBase/); // không in đường dẫn/owner inline
    expect(within(dlg).getAllByRole('img', { name: /do FontBase cài · \/Users\/u\/Library\/Fonts\/FontBase\/BT\/x\.ttf/ })).toHaveLength(2);
    expect(within(dlg).queryByText('Style 3')).toBeNull();
    const row = within(dlg).getByRole('button', { name: 'Xem các kiểu của BT Annam Neue' });
    expect(row.getAttribute('aria-expanded')).toBe('false');
    fireEvent.click(row);
    expect(within(dlg).getByText('Style 3')).toBeTruthy();
    expect(within(dlg).getByRole('button', { name: 'Thu gọn BT Annam Neue' }).getAttribute('aria-expanded')).toBe('true');
  });
  it('defaults to "Bỏ qua các font này (khuyên dùng)" and confirms with include=false/true', () => {
    const calls: boolean[] = [];
    render(<ConflictDialog items={items} total={12} onCancel={() => {}} onConfirm={i => calls.push(i)} />);
    expect((screen.getByRole('radio', { name: 'Bỏ qua các font này (khuyên dùng)' }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Vẫn kích hoạt' }));
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(calls).toEqual([false, true]);
  });
  it('is height-capped at 70vh with the list scrolling inside (CSS contract)', async () => {
    const { readFileSync } = await import('node:fs');
    const css = readFileSync('components/activation/conflictDialog.css', 'utf8');
    expect(css).toMatch(/\.fa-modal\.fa-conf\s*\{[^}]*max-height:\s*70vh/);
    expect(css).toMatch(/\.fa-conf \.fa-conf-list\s*\{[^}]*overflow-y:\s*auto/);
  });
  it('radios are a ring with a solid inner dot drawn from tokens (the native control showed a tiny dot in light and dark)', async () => {
    const { readFileSync } = await import('node:fs');
    const css = readFileSync('components/activation/conflictDialog.css', 'utf8');
    expect(css).toMatch(/\.fa-radio input\[type="radio"\]\s*\{[^}]*appearance:\s*none[^}]*border:\s*1\.5px solid var\(--muted\)[^}]*border-radius:\s*50%/);
    expect(css).toMatch(/input\[type="radio"\]::after\s*\{[^}]*width:\s*8px[^}]*background:\s*var\(--accent-ink\)/);
    expect(css).toMatch(/input\[type="radio"\]:checked\s*\{[^}]*border-color:\s*var\(--accent-ink\)/);
    expect(css).toMatch(/input\[type="radio"\]:checked::after\s*\{[^}]*scale\(1\)/);
  });
  it('renders in a portal on <body>, so a parent stacking context (Top Bar, grid) cannot bury it', () => {
    const { container } = render(<div style={{ position: 'relative', zIndex: 1 }}><ConflictDialog items={items} total={12} onCancel={() => {}} onConfirm={() => {}} /></div>);
    const dlg = screen.getByRole('dialog');
    expect(container.contains(dlg)).toBe(false);
    expect(dlg.parentElement?.parentElement).toBe(document.body);
  });
  it('a system-owned conflict blocks "Vẫn kích hoạt"', () => {
    render(<ConflictDialog items={[{ family: 'Arial', style: 'Regular', conflicts: [c({ owner: 'system', existingPath: '/Library/Fonts/Arial.ttf' })] }]} total={1} onCancel={() => {}} onConfirm={() => {}} />);
    expect((screen.getByRole('radio', { name: /Vẫn kích hoạt/ }) as HTMLInputElement).disabled).toBe(true);
  });
});

describe('activation flow with same-file duplicates', () => {
  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Harness({ api, faces }: { api: ReturnType<typeof createMockApi>; faces: { hashHex: string; faceIndex: number }[] }) {
    const flow = useActivationFlow(api);
    return <><ActivationDialogHost api={api} /><button type="button" onClick={() => void flow.toggle(faces, true)}>go</button></>;
  }
  const keysOf = async (api: ReturnType<typeof createMockApi>, family: string) => (await api.queryFaces({ text: family, limit: 100 })).items.filter(r => r.family === family).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));

  it('all fonts already installed as the same file: no dialog, one toast, nothing registered', async () => {
    const api = createMockApi();
    const faces = await keysOf(api, 'Pacifico');
    render(<Harness api={api} faces={faces} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useNoticeStore.getState().notices.map(n => n.text)).toEqual([`${faces.length} font đã có sẵn trên máy`]);
    expect(await api.activationState()).toEqual([]);
  });

  it('mixed: same-file fonts are not asked about; the dialog lists only the real clash, grouped by family', async () => {
    const api = createMockApi();
    const same = await keysOf(api, 'Pacifico');
    const clash = await keysOf(api, 'Roboto');
    const ok = await keysOf(api, 'Inter');
    render(<Harness api={api} faces={[...same, ...clash, ...ok.slice(0, 1)]} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    const dlg = screen.getByRole('dialog');
    expect(dlg.textContent).toContain(`Roboto · ${clash.length} kiểu`);
    expect(dlg.textContent).not.toContain('Pacifico');
    expect(useNoticeStore.getState().notices.some(n => /đã có sẵn trên máy/.test(n.text))).toBe(true);
    expect((await api.activationState()).length).toBe(1); // Inter đã bật ngay
  });
});
