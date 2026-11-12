import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConflictDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useActivationQueue } from '../../../store/activationFlow';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { tipOf } from '../../../test/tip';
import { ActivationDialogHost } from '../ActivationDialogHost';
import { describeOutcome } from '../messages';
import { ConflictDialog } from '../ActivationDialogs';
import { confirmedOf, replaceAdvice, replaceTierOf } from '../conflictModel';
import { useActivationFlow } from '../useActivationFlow';

/** Stand-in for the backend's matrix (fontatlas-activate::replace_tier): the UI only echoes what the backend says. */
const tierOf = (owner: ConflictDto['owner'], path: string): ConflictDto['replaceTier'] =>
  /^(\/System\/Library|\/Library\/Fonts|\/Users\/[^/]+\/Library\/Fonts)\//.test(path) || owner === 'system' || owner === 'user_installed' ? 'protected' : owner === 'adobe_fonts' || owner === 'suitcase' ? 'guide' : 'replace';
const c = (owner: ConflictDto['owner'], existingPath: string, over: Partial<ConflictDto> = {}): ConflictDto => ({ postscriptName: 'X-Regular', existingPath, owner, sameFile: false, replaceTier: tierOf(owner, existingPath), variable: false, existingVariable: null, ...over });
const item = (conflicts: ConflictDto[]) => ({ family: 'Oswald', style: 'Regular', conflicts });
const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('replace matrix in the UI (the backend decides; the UI never guesses from paths)', () => {
  it('the tier comes from the conflict itself, whatever the path looks like', () => {
    expect(replaceTierOf(c('other', '/Users/u/Library/Fonts/A.ttf', { replaceTier: 'replace' }))).toBe('replace');
    expect(replaceTierOf(c('right_font', '/Volumes/X/lib/A.ttf', { replaceTier: 'protected' }))).toBe('protected');
    expect(replaceTierOf(c('right_font', '/x.ttf', { replaceTier: 'guide' }))).toBe('guide');
  });

  it('advice: the worst tier wins; app names are collected; "other" is flagged as unknown', () => {
    const a = replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('font_base', '/lib/b.ttf')])]);
    expect(a).toMatchObject({ tier: 'replace', apps: ['RightFont', 'FontBase'], unknownApp: false });
    expect(replaceAdvice([item([c('other', '/lib/a.ttf')])])).toMatchObject({ tier: 'replace', apps: [], unknownApp: true });
    expect(replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('adobe_fonts', '/x/a.otf')])]).tier).toBe('guide');
    expect(replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('system', '/Library/Fonts/a.ttf')])]).tier).toBe('protected');
  });

  it('same-file copies count too: the backend removes every conflict, so a protected same-file copy blocks replacing', () => {
    const a = replaceAdvice([item([c('right_font', '/lib/a.ttf'), c('system', '/Library/Fonts/a.ttf', { sameFile: true })])]);
    expect(a.tier).toBe('protected');
  });

  it('confirmedOf lists every (file, installed path) pair the user saw, once', () => {
    const it1 = { hashHex: 'aa', family: 'F', style: 'Regular', conflicts: [c('right_font', '/lib/a.ttf'), c('right_font', '/lib/a.ttf', { postscriptName: 'X-Bold' })] };
    expect(confirmedOf([it1, { ...it1, hashHex: 'bb' }])).toEqual([{ hashHex: 'aa', existingPath: '/lib/a.ttf' }, { hashHex: 'bb', existingPath: '/lib/a.ttf' }]);
  });
});

describe('ConflictDialog · replace option', () => {
  const mount = (items: ReturnType<typeof item>[], over: Partial<React.ComponentProps<typeof ConflictDialog>> = {}) => {
    const spies = { onConfirm: vi.fn(), onReplace: vi.fn(), onRetry: vi.fn(), onReveal: vi.fn(), onCancel: vi.fn() };
    render(<ConflictDialog items={items} total={items.length} {...spies} {...over} />);
    return spies;
  };

  it('RightFont: primary "Thay bằng bản này" with a short warning naming the app; Go calls onReplace', () => {
    const s = mount([item([c('right_font', '/Volumes/L/x.rightfontlibrary/a.ttf')])]);
    const radio = screen.getByRole('radio', { name: /Thay bằng bản này/ }) as HTMLInputElement;
    expect(radio.disabled).toBe(false);
    fireEvent.click(radio);
    expect(screen.getByTestId('replace-warning').textContent).toBe('RightFont có thể bật lại bản của nó; tắt trong RightFont để giữ lâu dài.');
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(s.onReplace).toHaveBeenCalledTimes(1);
    expect(s.onConfirm).not.toHaveBeenCalled();
  });

  it('default choice: "Thay bằng bản này" is preselected for a known app, "Bỏ qua" when an unknown app is involved or replacing is not allowed', () => {
    const radio = (re: RegExp) => screen.getByRole('radio', { name: re }) as HTMLInputElement;
    const m1 = mount([item([c('font_base', '/Volumes/L/a.ttf')])]);
    expect(radio(/Thay bằng bản này/).checked).toBe(true);
    expect(m1.onReplace).not.toHaveBeenCalled();
    cleanup();
    mount([item([c('right_font', '/Volumes/L/a.ttf'), c('other', '/Volumes/L/b.ttf')])]);
    expect(radio(/Bỏ qua/).checked).toBe(true);
    cleanup();
    mount([item([c('adobe_fonts', '/x/a.otf')])]);
    expect(radio(/Bỏ qua/).checked).toBe(true);
  });

  it('RightFont also shows "Hướng dẫn tắt trong RightFont" right below, without a toggle: instructions plus a retry that is not inside a paragraph', () => {
    const s = mount([item([c('right_font', '/Volumes/L/a.ttf')])]);
    expect(screen.getByTestId('guide-text').textContent).toMatch(/Hướng dẫn tắt trong RightFont.*Mở RightFont, tắt font này/);
    expect(screen.getByRole('button', { name: 'Thử lại' }).closest('p')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    expect(s.onRetry).toHaveBeenCalledTimes(1);
  });

  it('the scope words in the explanation are glossary terms with tooltips', async () => {
    mount([item([c('font_base', '/Volumes/L/a.ttf')])]);
    expect(await tipOf(document.querySelector('[data-term="scope_session"]')!)).toMatch(/đóng phiên đăng nhập|đăng xuất/);
    expect(await tipOf(document.querySelector('[data-term="scope_persistent"]')!)).toMatch(/sau khi khởi động lại|lâu dài/);
  });

  it('an unknown app needs an explicit "I understand" before Go works, and says it cannot name the app', () => {
    const s = mount([item([c('other', '/Volumes/L/a.ttf')])]);
    fireEvent.click(screen.getByRole('radio', { name: /Thay bằng bản này/ }));
    expect(screen.getByTestId('replace-warning').textContent).toMatch(/không biết đó là app nào/);
    const go = screen.getByRole('button', { name: 'Tiếp tục' }) as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.click(screen.getByRole('checkbox', { name: /Tôi hiểu/ }));
    expect(go.disabled).toBe(false);
    fireEvent.click(go);
    expect(s.onReplace).toHaveBeenCalled();
  });

  it('an unknown app next to a known one still needs "I understand", and the warning lists both', () => {
    const s = mount([item([c('right_font', '/Volumes/L/a.ttf'), c('other', '/Volumes/L/b.ttf')])]);
    fireEvent.click(screen.getByRole('radio', { name: /Thay bằng bản này/ }));
    expect(screen.getByTestId('replace-warning').textContent).toMatch(/RightFont \/ một ứng dụng khác/);
    expect((screen.getByRole('button', { name: 'Tiếp tục' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('checkbox', { name: /Tôi hiểu/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(s.onReplace).toHaveBeenCalled();
  });

  it('the warning is linked to the replace option for screen readers', () => {
    mount([item([c('font_base', '/Volumes/L/a.ttf')])]);
    const radio = screen.getByRole('radio', { name: /Thay bằng bản này/ });
    expect(document.getElementById(radio.getAttribute('aria-describedby')!)).toBe(screen.getByTestId('replace-warning'));
  });

  it('Adobe / Suitcase: no replace option, only guidance with a retry', () => {
    const s = mount([item([c('adobe_fonts', '/Users/u/Library/Application Support/Adobe/CoreSync/plugins/livetype/.r/.1.otf')])]);
    expect(screen.queryByRole('radio', { name: /Thay bằng bản này/ })).toBeNull();
    expect(screen.getByTestId('guide-text').textContent).toMatch(/Adobe/);
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    expect(s.onRetry).toHaveBeenCalled();
  });

  it('system and font folders: never replaceable; only "Hiện trong Finder" for the file', () => {
    const s = mount([item([c('user_installed', '/Users/u/Library/Fonts/Inter.ttf')])]);
    expect(screen.queryByRole('radio', { name: /Thay bằng bản này/ })).toBeNull();
    expect(screen.getByTestId('protected-note').textContent).toMatch(/không tắt hay di chuyển/);
    fireEvent.click(screen.getByRole('button', { name: 'Hiện trong Finder' }));
    expect(s.onReveal).toHaveBeenCalledWith('/Users/u/Library/Fonts/Inter.ttf');
  });

  it('a mix with one protected clash hides replace for the whole dialog', () => {
    mount([item([c('right_font', '/Volumes/L/a.ttf')]), item([c('system', '/Library/Fonts/A.ttf')])]);
    expect(screen.queryByRole('radio', { name: /Thay bằng bản này/ })).toBeNull();
  });
});

describe('activation flow · replace and restore', () => {
  it('Settings-style journal: a restored copy leaves the unrestored list', async () => {
    const api = createMockApi();
    const f = (await api.queryFaces({ text: 'Oswald', limit: 5 })).items[0];
    const pre = await api.activationPreflight([{ hashHex: f.hashHex, faceIndex: f.faceIndex }]);
    const seen = pre.results.flatMap(r => (r.outcome.kind === 'needs_confirmation' ? r.outcome.conflicts.map(x => ({ hashHex: r.hashHex, existingPath: x.existingPath })) : []));
    const rep = await api.activationReplace([{ hashHex: f.hashHex, faceIndex: f.faceIndex }], seen);
    expect(await api.activationListReplaced()).toHaveLength(1);
    await api.activationRestoreReplaced(rep.results[0].replaced![0].id);
    expect(await api.activationListReplaced()).toHaveLength(0);
  });

  beforeEach(() => { useActivationQueue.getState().clear(); useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
  function Harness({ api, faces }: { api: ReturnType<typeof createMockApi>; faces: { hashHex: string; faceIndex: number }[] }) {
    const flow = useActivationFlow(api);
    return <><ActivationDialogHost api={api} /><button type="button" onClick={() => void flow.toggle(faces, true)}>go</button></>;
  }
  const keysOf = async (api: ReturnType<typeof createMockApi>, family: string) => (await api.queryFaces({ text: family, limit: 100 })).items.filter(r => r.family === family).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));

  it('choosing replace turns on our copy, says what was removed, and the notice offers "Khôi phục bản cũ"', async () => {
    const api = createMockApi();
    const faces = (await keysOf(api, 'Oswald')).slice(0, 1);
    render(<Harness api={api} faces={faces} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    const dlg = screen.getByRole('dialog');
    fireEvent.click(within(dlg).getByRole('radio', { name: /Thay bằng bản này/ }));
    await act(async () => { fireEvent.click(within(dlg).getByRole('button', { name: 'Tiếp tục' })); }); await settle();
    expect((await api.activationState()).map(s => s.hashHex)).toEqual([faces[0].hashHex]);
    const n = useNoticeStore.getState().notices.find(x => /Đã tắt bản cũ của RightFont/.test(x.text))!;
    expect(n).toBeTruthy();
    expect(n.action?.label).toBe('Khôi phục bản cũ');
    await act(async () => { n.action!.run(); }); await settle();
    expect(await api.activationState()).toEqual([]);
    expect(useNoticeStore.getState().notices.some(x => /Đã khôi phục bản cũ/.test(x.text))).toBe(true);
  });

  it('"Thử lại" in the dialog runs the normal activation again (the user switched the other app off)', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'activationActivate');
    const faces = (await keysOf(api, 'Oswald')).slice(0, 1);
    render(<Harness api={api} faces={faces} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    spy.mockClear();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Thử lại' })); }); await settle();
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('a failed replace explains itself in plain words, not as a bare code', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'activationReplace').mockResolvedValue({ results: [{ hashHex: 'h', path: null, requestedFaces: [0], extraFaces: [], outcome: { kind: 'failed', code: 'replace_not_registered', message: 'x' } }], changed: 0, needsConfirmation: 0, failed: 1 });
    const faces = (await keysOf(api, 'Oswald')).slice(0, 1);
    render(<Harness api={api} faces={faces} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    fireEvent.click(screen.getByRole('radio', { name: /Thay bằng bản này/ }));
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' })); }); await settle();
    const text = useNoticeStore.getState().notices.map(n => n.text).join('\n');
    expect(text).toMatch(/không tắt được từ Font Atlas/);
    expect(text).not.toMatch(/replace_not_registered/);
  });

  it('a confirmation that went stale asks again with the new conflicts instead of replacing', async () => {
    const api = createMockApi();
    const real = api.activationReplace.bind(api);
    const spy = vi.spyOn(api, 'activationReplace').mockImplementationOnce(async (faces, _confirmed, o) => real(faces, [{ hashHex: faces[0].hashHex, existingPath: '/da/doi.ttf' }], o));
    const faces = (await keysOf(api, 'Oswald')).slice(0, 1);
    render(<Harness api={api} faces={faces} />);
    await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
    fireEvent.click(screen.getByRole('radio', { name: /Thay bằng bản này/ }));
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' })); }); await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect(await api.activationState()).toEqual([]);
    expect(screen.getByRole('dialog'), 'dialog reopens').toBeTruthy();
    expect(useNoticeStore.getState().notices.map(n => n.text).join('\n')).toMatch(/đã đổi so với lúc bạn xác nhận/);
  });

  it('"not effective" and "rollback failed" are told truthfully; not-effective still offers the restore action', async () => {
    const api = createMockApi();
    const f = (rc: string, replaced?: boolean) => ({ results: [{ hashHex: 'h', path: null, requestedFaces: [0], extraFaces: [], outcome: rc === 'replace_not_effective' ? { kind: 'activated' as const } : { kind: 'failed' as const, code: rc, message: 'x' }, ...(rc === 'replace_not_effective' ? { warning: rc } : {}), ...(replaced ? { replaced: [{ id: 1, path: '/x.ttf', owner: 'right_font' as const, scopes: ['persistent' as const] }] } : {}) }], changed: 0, needsConfirmation: 0, failed: 1 });
    vi.spyOn(api, 'activationReplace').mockResolvedValueOnce(f('replace_not_effective', true)).mockResolvedValueOnce(f('replace_rollback_failed', true));
    const faces = (await keysOf(api, 'Oswald')).slice(0, 1);
    render(<Harness api={api} faces={faces} />);
    for (let i = 0; i < 2; i++) {
      await act(async () => { fireEvent.click(screen.getByText('go')); }); await settle();
      fireEvent.click(screen.getByRole('radio', { name: /Thay bằng bản này/ }));
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' })); }); await settle();
    }
    const ns = useNoticeStore.getState().notices;
    expect(ns.some(n => /một bản khác vẫn đang cung cấp/.test(n.text) && n.action?.label === 'Khôi phục bản cũ')).toBe(true);
    expect(ns.some(n => /không trả lại được bản cũ/.test(n.text))).toBe(true);
  });
});

describe('messages for the new replace codes', () => {
  it('replace_own_held tells the user a collection keeps another copy on', () => {
    expect(describeOutcome({ kind: 'failed', code: 'replace_own_held', message: 'x' })?.text).toMatch(/collection/);
  });
});

describe('conflict dialog stays compact and "skip" does not replace', () => {
  const open = () => {
    const onConfirm = vi.fn(); const onReplace = vi.fn();
    render(<ConflictDialog items={[item([c('right_font', '/lib/a.ttf', { replaceTier: 'replace' })])]} total={25} onCancel={() => {}} onConfirm={onConfirm} onReplace={onReplace} />);
    return { onConfirm, onReplace };
  };
  it('one short warning stays visible; scope and guide live inside one collapsed <details>', () => {
    open();
    const det = document.querySelector('details.fa-conf-more') as HTMLDetailsElement;
    expect(det).toBeTruthy();
    expect(det.open).toBe(false);
    expect(det.contains(screen.getByTestId('replace-warning'))).toBe(false);
    expect(det.contains(screen.getByTestId('guide-text'))).toBe(true);
    cleanup();
  });
  it('choosing skip then continue only confirms (no replace); replace is the pre-selected default for a known app', () => {
    const { onConfirm, onReplace } = open();
    const radios = screen.getAllByRole('radio') as HTMLInputElement[];
    expect(radios.find(r => r.checked)?.closest('label')?.textContent).toMatch(/Thay bằng bản này/);
    fireEvent.click(radios[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' }));
    expect(onReplace).not.toHaveBeenCalled();
    expect(onConfirm).toHaveBeenCalledWith(false);
    cleanup();
  });
});
