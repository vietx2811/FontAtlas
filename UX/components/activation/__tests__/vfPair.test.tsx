import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ConflictDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useActivationQueue } from '../../../store/activationFlow';
import { useUiStore } from '../../../store/ui';
import { ActivationDialogHost } from '../ActivationDialogHost';
import { ConflictDialog } from '../ActivationDialogs';
import { vfActionOf, vfChoiceOf, vfPairOf, type ConflictItem } from '../conflictModel';
import { useActivationFlow } from '../useActivationFlow';

const c = (over: Partial<ConflictDto> = {}): ConflictDto => ({
  postscriptName: 'Oswald-Regular', existingPath: '~/Library/RightFont/Oswald.ttf', owner: 'right_font', sameFile: false, replaceTier: 'replace', variable: false, existingVariable: true, ...over,
});
const item = (over: Partial<ConflictDto> = {}): ConflictItem => ({ hashHex: 'aa', family: 'Oswald', style: 'Regular', conflicts: [c(over)] });
const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('vfPairOf: only a pure VF↔static clash is a pair', () => {
  it('different kinds on both sides → pair, and says which one is new', () => {
    expect(vfPairOf([item()])).toEqual({ newIsVf: false });
    expect(vfPairOf([item({ variable: true, existingVariable: false })])).toEqual({ newIsVf: true });
  });
  it('same kind, unknown kind, mixed new kinds, or only same-file clashes → not a pair (plain dialog)', () => {
    expect(vfPairOf([item({ existingVariable: false })])).toBeNull();
    expect(vfPairOf([item({ existingVariable: null })])).toBeNull();
    expect(vfPairOf([item(), item({ variable: true, existingVariable: false })])).toBeNull();
    expect(vfPairOf([item({ sameFile: true })])).toBeNull();
    expect(vfPairOf([])).toBeNull();
  });
  it('a same-file clash next to a real pair does not spoil the pair', () => {
    expect(vfPairOf([{ ...item(), conflicts: [c(), c({ sameFile: true, existingPath: '/x/same.ttf' })] }])).toEqual({ newIsVf: false });
  });
});

describe('remembered choice is stored by KIND and maps back to an action for either direction', () => {
  it('table', () => {
    // nhớ "vf": khi bản mới là VF → dùng nó; khi bản mới là static → giữ bản VF đang có
    expect(vfActionOf('vf', true)).toBe('use');
    expect(vfActionOf('vf', false)).toBe('keep');
    expect(vfActionOf('static', true)).toBe('keep');
    expect(vfActionOf('static', false)).toBe('use');
    expect(vfActionOf('both', true)).toBe('both');
    expect(vfActionOf('both', false)).toBe('both');
    for (const nv of [true, false]) for (const a of ['keep', 'use', 'both'] as const) expect(vfActionOf(vfChoiceOf(a, nv), nv)).toBe(a);
  });
});

describe('ConflictDialog in pair mode', () => {
  it('asks with three choices named by kind (with glossed terms), keeps the existing copy by default — never swaps on its own', () => {
    render(<ConflictDialog items={[item()]} total={1} onCancel={() => {}} onConfirm={() => {}} onReplace={() => {}} onRemember={() => {}} />);
    const dlg = screen.getByRole('dialog');
    expect(dlg.textContent).toContain('variable font (VF)');
    expect(dlg.textContent).toContain('static font');
    const keep = screen.getByRole('radio', { name: /Dùng bản variable font \(VF\) đang có/ }) as HTMLInputElement;
    expect(keep.checked).toBe(true);
    expect((screen.getByRole('radio', { name: /Dùng bản static font này/ }) as HTMLInputElement).checked).toBe(false);
    expect(screen.getByRole('radio', { name: /Giữ cả hai/ })).toBeTruthy();
    expect(dlg.querySelectorAll('[data-term="variable_font"]').length).toBeGreaterThan(0);
  });
  it('"Giữ cả hai" shows the warning; "Dùng bản này" calls onReplace; remember saves the KIND', () => {
    const calls: string[] = [];
    render(<ConflictDialog items={[item()]} total={1} onCancel={() => {}} onConfirm={i => calls.push(`confirm:${i}`)} onReplace={() => calls.push('replace')} onRemember={ch => calls.push(`remember:${ch}`)} />);
    fireEvent.click(screen.getByRole('radio', { name: /Giữ cả hai/ }));
    expect(screen.getByTestId('vf-both-warning')).toBeTruthy();
    fireEvent.click(screen.getByRole('radio', { name: /Dùng bản static font này/ }));
    expect(screen.queryByTestId('vf-both-warning')).toBeNull();
    fireEvent.click(screen.getByRole('checkbox', { name: /Nhớ lựa chọn này/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(calls).toEqual(['remember:static', 'replace']); // bản mới là static, chọn dùng nó → nhớ "static"
  });
  it('without the tick nothing is remembered; "keep" remembered when the new one is static means "vf"', () => {
    const calls: string[] = [];
    const { unmount } = render(<ConflictDialog items={[item()]} total={1} onCancel={() => {}} onConfirm={i => calls.push(`confirm:${i}`)} onRemember={ch => calls.push(`remember:${ch}`)} />);
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(calls).toEqual(['confirm:false']);
    unmount();
    calls.length = 0;
    render(<ConflictDialog items={[item()]} total={1} onCancel={() => {}} onConfirm={i => calls.push(`confirm:${i}`)} onRemember={ch => calls.push(`remember:${ch}`)} />);
    fireEvent.click(screen.getByRole('checkbox', { name: /Nhớ lựa chọn này/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(calls).toEqual(['remember:vf', 'confirm:false']);
  });
  it('a non-pair clash keeps the ordinary dialog: no pair options, replace preselected when allowed', () => {
    render(<ConflictDialog items={[item({ existingVariable: false })]} total={1} onCancel={() => {}} onConfirm={() => {}} onReplace={() => {}} />);
    expect(screen.queryByTestId('vf-pair-options')).toBeNull();
    expect((screen.getByRole('radio', { name: /Thay bằng bản này/ }) as HTMLInputElement).checked).toBe(true);
  });
  it('a system font blocks "Giữ cả hai" in pair mode too', () => {
    render(<ConflictDialog items={[item({ owner: 'system', replaceTier: 'protected' })]} total={1} onCancel={() => {}} onConfirm={() => {}} />);
    expect((screen.getByRole('radio', { name: /Giữ cả hai/ }) as HTMLInputElement).disabled).toBe(true);
  });
});

describe('activation flow with a remembered VF/static choice (mock: library has the static, RightFont has the VF)', () => {
  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Harness({ api, faces }: { api: ReturnType<typeof createMockApi>; faces: { hashHex: string; faceIndex: number }[] }) {
    const flow = useActivationFlow(api);
    return <><ActivationDialogHost api={api} /><button type="button" onClick={() => void flow.toggle(faces, true)}>go</button></>;
  }
  const faces = async (api: ReturnType<typeof createMockApi>) => {
    const r = (await api.queryFaces({ text: 'Bebas Neue', limit: 100 })).items.filter(x => x.family === 'Bebas Neue').slice(0, 1);
    return r.map(x => ({ hashHex: x.hashHex, faceIndex: x.faceIndex }));
  };
  const run = async (policy: 'ask' | 'vf' | 'static' | 'both') => {
    const api = createMockApi();
    await api.activationSetVfPolicy(policy);
    const f = await faces(api);
    render(<Harness api={api} faces={f} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    return { api, f, notices: () => useNoticeStore.getState().notices.map(n => n.text) };
  };

  it('ask (default): the pair dialog opens and nothing is turned on', async () => {
    const { api } = await run('ask');
    expect(screen.getByTestId('vf-pair-options')).toBeTruthy();
    expect(await api.activationState()).toEqual([]);
  });
  it('remembered "vf" while the new one is static → keeps the VF: no dialog, nothing turned on, says so', async () => {
    const { api, notices } = await run('vf');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(await api.activationState()).toEqual([]);
    expect(notices().some(t => /Giữ bản variable font \(VF\)/.test(t))).toBe(true);
  });
  it('remembered "static" → uses the static (replace): no dialog, turned on, with an undo notice', async () => {
    const { api, notices } = await run('static');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect((await api.activationState()).length).toBe(1);
    expect(notices().some(t => /Đã bật bản static font và tắt bản variable font \(VF\)/.test(t))).toBe(true);
    expect(useNoticeStore.getState().notices.some(n => n.action?.label === 'Khôi phục bản cũ')).toBe(true);
  });
  it('remembered "both" → turns on with proceed, warns, no dialog', async () => {
    const { api, notices } = await run('both');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect((await api.activationState()).length).toBe(1);
    expect(notices().some(t => /cả hai bản trùng tên/.test(t))).toBe(true);
  });
  it('the dialog\'s remember box writes the policy through the API', async () => {
    const { api } = await run('ask');
    fireEvent.click(screen.getByRole('checkbox', { name: /Nhớ lựa chọn này/ }));
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' })); }); await settle();
    expect(await api.activationVfPolicy()).toBe('vf'); // chọn mặc định "dùng bản VF đang có" khi bản mới là static
  });
});
