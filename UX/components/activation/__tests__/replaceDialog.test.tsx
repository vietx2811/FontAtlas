import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConflictDto } from '../../../api';
import { createMockApi } from '../../../api/mock';
import { useActivationQueue } from '../../../store/activationFlow';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { tipOf } from '../../../test/tip';
import { ActivationDialogHost } from '../ActivationDialogHost';
import { ConflictDialog } from '../ActivationDialogs';
import { replaceAdvice, replaceTierOf } from '../conflictModel';
import { useActivationFlow } from '../useActivationFlow';

const c = (owner: ConflictDto['owner'], existingPath: string, over: Partial<ConflictDto> = {}): ConflictDto => ({ postscriptName: 'X-Regular', existingPath, owner, sameFile: false, ...over });
const item = (conflicts: ConflictDto[]) => ({ family: 'Oswald', style: 'Regular', conflicts });
const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

describe('replace matrix in the UI (mirrors fontatlas-activate::replace_tier)', () => {
  it('protected: system and font folders by owner or by path; guide: Adobe and Suitcase; replace: everything else', () => {
    expect(replaceTierOf(c('system', '/System/Library/Fonts/A.ttf'))).toBe('protected');
    expect(replaceTierOf(c('system', '/Library/Fonts/A.ttf'))).toBe('protected');
    expect(replaceTierOf(c('user_installed', '/Users/u/Library/Fonts/A.ttf'))).toBe('protected');
    expect(replaceTierOf(c('other', '/Users/u/Library/Fonts/A.ttf'))).toBe('protected');
    expect(replaceTierOf(c('right_font', '~/Library/Fonts/A.ttf'))).toBe('protected');
    expect(replaceTierOf(c('adobe_fonts', '/x/Adobe/CoreSync/plugins/livetype/.r/.1.otf'))).toBe('guide');
    expect(replaceTierOf(c('suitcase', '/x/Suitcase Fusion/a.ttf'))).toBe('guide');
    for (const o of ['font_atlas', 'right_font', 'font_base', 'typeface', 'other'] as const) expect(replaceTierOf(c(o, '/Volumes/X/lib/A.ttf'))).toBe('replace');
    expect(replaceTierOf(c('right_font', '/Users/u/Library/FontsOld/A.ttf'))).toBe('replace');
  });

  it('advice: the worst tier wins; app names are collected; "other" is flagged as unknown', () => {
    const a = replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('font_base', '/lib/b.ttf')])]);
    expect(a).toMatchObject({ tier: 'replace', apps: ['RightFont', 'FontBase'], unknownApp: false });
    expect(replaceAdvice([item([c('other', '/lib/a.ttf')])])).toMatchObject({ tier: 'replace', apps: [], unknownApp: true });
    expect(replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('adobe_fonts', '/x/a.otf')])]).tier).toBe('guide');
    expect(replaceAdvice([item([c('right_font', '/lib/a.ttf')]), item([c('system', '/Library/Fonts/a.ttf')])]).tier).toBe('protected');
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

  it('RightFont also gets the secondary "Hướng dẫn tắt trong RightFont": instructions plus a retry', () => {
    const s = mount([item([c('right_font', '/Volumes/L/a.ttf')])]);
    expect(screen.queryByTestId('guide-text')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn tắt trong RightFont' }));
    expect(screen.getByTestId('guide-text').textContent).toMatch(/Mở RightFont, tắt font này/);
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
    fireEvent.click(screen.getByRole('button', { name: 'Hướng dẫn tắt trong RightFont' }));
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
});
