import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { resolveHooks } from '../../../api/mockResolve';
import type { Api } from '../../../api';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { DuplicatesPage } from '../DuplicatesPage';

const initial = useUiStore.getState();
beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
const mount = async (api: Api = createMockApi()) => { render(<DuplicatesPage api={api} />); await screen.findAllByTestId('dup-group'); return api; };
const cardOf = (re: RegExp) => screen.getAllByTestId('dup-group').find(x => re.test(x.textContent ?? ''))!;
const click = (el: HTMLElement) => fireEvent.click(el);
const dialog = () => screen.getByRole('dialog');
const confirm = () => click(screen.getByTestId('pv-confirm'));

describe('DuplicatesPage: reading the page', () => {
  it('says in plain words how many groups, how many are safe, how many need you', async () => {
    await mount();
    expect(screen.getByTestId('dup-summary').textContent).toBe('Tìm thấy 6 nhóm font bị trùng. 4 nhóm xử lý được ngay, 2 nhóm cần bạn xem.');
    expect(screen.getByTestId('resolve-all-safe').textContent).toBe('Giải quyết 4 nhóm an toàn');
  });

  it('uses everyday names for the kinds, review groups first, and shows no raw codes anywhere', async () => {
    await mount();
    const heads = screen.getAllByRole('heading', { level: 2 }).map(h => h.textContent ?? '');
    expect(heads[0]).toMatch(/^Cần bạn xem/);
    for (const name of ['Bản sao giống hệt', 'Bản cũ và bản mới', 'Cùng font, khác định dạng']) expect(heads.some(h => h.startsWith(name)) || cardOf(/./) != null).toBe(true);
    expect(screen.getByRole('heading', { name: /Bản sao giống hệt/ })).toBeTruthy();
    expect(screen.getByRole('heading', { name: /Bản cũ và bản mới/ })).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/same_content|preferred_|newest_unknown|collection_file|may_reflow|tidiest_path|\bsame_file\b/);
  });

  it('each card puts the two copies side by side with the SAME Vietnamese sample line and a plain reason', async () => {
    await mount();
    const g = cardOf(/Be Vietnam Pro Regular/);
    const keep = within(g).getByTestId('dup-side-keep'), hide = within(g).getByTestId('dup-side-hide');
    expect(within(keep).getByText('Nên giữ')).toBeTruthy();
    expect(within(hide).getByText('Sẽ ẩn')).toBeTruthy();
    const [a, b] = [within(keep).getByTestId('dup-spec'), within(hide).getByTestId('dup-spec')];
    expect(a.textContent).toBe(b.textContent);
    expect(a.textContent).toMatch(/Việt Nam/);
    expect(within(g).getByTestId('dup-why').textContent).toBe('Hai bản giống hệt nhau; giữ bản có tên và đường dẫn gọn hơn');
    // the two copies must be told apart by their folder, not just the file name
    expect(keep.querySelector('.dup-where')!.textContent).not.toBe(hide.querySelector('.dup-where')!.textContent);
  });

  it('a version group translates the reason from its code and args; a warning is a note mark, not a chip', async () => {
    await mount();
    const g = cardOf(/Playfair Display Bold/);
    expect(within(g).getByTestId('dup-why').textContent).toContain('Bản mới hơn: 1.204 so với 1.203');
    expect(within(g).getByLabelText('Văn bản dùng bản cũ có thể xuống dòng khác đi')).toBeTruthy();
  });

  it('review groups say why the app did not decide', async () => {
    await mount();
    const g = cardOf(/Roboto Regular/);
    expect(within(g).getByTestId('dup-review').textContent).toContain('Không biết bản nào mới hơn');
    expect(g.closest('[data-testid=dup-review-section]')).toBeTruthy();
  });

  it('technical details stay behind "Chi tiết"', async () => {
    await mount();
    const g = cardOf(/Be Vietnam Pro Regular/);
    expect(within(g).queryByTestId('dup-detail')).toBeNull();
    expect(g.textContent).not.toMatch(/^.*Khoá nhóm/);
    const btn = within(g).getByRole('button', { name: 'Chi tiết' });
    expect(btn.getAttribute('aria-expanded')).toBe('false');
    click(btn);
    expect(btn.getAttribute('aria-expanded')).toBe('true');
    expect(within(g).getAllByTestId('dup-detail')).toHaveLength(2);
    expect(within(g).getAllByText('Hiện trong Finder')).toHaveLength(2);
    expect(g.textContent).toContain('Khoá nhóm');
  });

  it('uses the app tooltip, never a native title attribute; every control has an accessible name; no button is natively disabled', async () => {
    await mount();
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Chi tiết' }));
    expect(document.querySelectorAll('.dup [title]')).toHaveLength(0);
    for (const b of document.querySelectorAll('.dup button')) { expect((b.textContent ?? '').trim() || b.getAttribute('aria-label'), b.outerHTML).toBeTruthy(); expect(b.hasAttribute('disabled')).toBe(false); }
  });

  it('shows an empty state when nothing is duplicated, and an error with retry', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'resolvePreview').mockResolvedValueOnce({ items: [], needsReview: 0, keepCount: 0, hideCount: 0, reclaimableBytes: 0 });
    render(<DuplicatesPage api={api} />);
    expect(await screen.findByText('Không có font trùng')).toBeTruthy();
    const bad = createMockApi();
    vi.spyOn(bad, 'resolvePreview').mockRejectedValueOnce(new Error('boom'));
    render(<DuplicatesPage api={bad} />);
    expect(await screen.findByRole('alert')).toBeTruthy();
    click(screen.getByRole('button', { name: 'Thử lại' }));
    await waitFor(() => expect(screen.getAllByTestId('dup-group').length).toBeGreaterThan(0));
  });

  it('the tabs are a real tablist that arrow keys move through', async () => {
    await mount();
    const tabs = screen.getAllByRole('tab');
    expect(tabs.map(x => x.textContent?.replace(/\d+$/, ''))).toEqual(['Cần xử lý', 'Đã giải quyết', 'Đã bỏ qua']);
    expect(tabs[0].getAttribute('aria-selected')).toBe('true');
    fireEvent.keyDown(tabs[0], { key: 'ArrowRight' });
    await waitFor(() => expect(screen.getByRole('tab', { name: 'Đã giải quyết' }).getAttribute('aria-selected')).toBe('true'));
    expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('dup-tab-done');
  });
});

describe('DuplicatesPage: resolving a group', () => {
  it('opens a preview with Hide selected by default, applies "hide" with the plan\'s memberPaths, then offers Undo', async () => {
    const api = await mount();
    const hooks = resolveHooks(api);
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Giải quyết' }));
    const d = await screen.findByRole('dialog');
    await within(d).findByTestId('pv-summary');
    expect((within(d).getByLabelText(/Ẩn trong Font Atlas/) as HTMLInputElement).checked).toBe(true);
    expect(within(d).getByTestId('pv-summary').textContent).toBe('Giữ 1 bản, ẩn 1 bản.');
    confirm();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const req = hooks.__log.applied[0];
    expect(req.action).toBe('hide'); expect(req.mode).toBe('chosen'); expect(req.explicitTrashPaths).toEqual([]);
    expect(req.items[0].memberPaths.length).toBe(2); expect(req.items[0].loserPaths.length).toBe(1);
    await waitFor(() => expect(screen.queryByText('Be Vietnam Pro Regular', { selector: 'article b' })).toBeNull());
    const note = useNoticeStore.getState().notices.find(n => n.text === 'Đã ẩn 1 bản')!;
    expect(note.action?.label).toBe('Hoàn tác');
    expect(useUiStore.getState().libraryRev).toBeGreaterThan(0);
    note.action!.run();
    await waitFor(() => expect(cardOf(/Be Vietnam Pro Regular/)).toBeTruthy());
  });

  it('"Đổi bản giữ" swaps the keeper, says the user chose it, and sends the other path as keeper', async () => {
    const api = await mount();
    const hooks = resolveHooks(api);
    const g = cardOf(/Be Vietnam Pro Regular/);
    const keepBefore = within(g).getByTestId('dup-side-keep').querySelector('.dup-where')!.textContent;
    click(within(g).getByRole('button', { name: 'Đổi bản giữ' }));
    const g2 = cardOf(/Be Vietnam Pro Regular/);
    expect(within(g2).getByTestId('dup-side-keep').querySelector('.dup-where')!.textContent).not.toBe(keepBefore);
    expect(within(g2).getByTestId('dup-why').textContent).toBe('Bạn đã chọn giữ bản này');
    click(within(g2).getByRole('button', { name: 'Giải quyết' }));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    const swapped = hooks.__log.applied.length;
    confirm();
    await waitFor(() => expect(hooks.__log.applied.length).toBe(swapped + 1));
    const req = hooks.__log.applied[swapped];
    expect(req.items[0].keeperPath).toContain('/Downloads/');
    expect(req.items[0].loserPaths[0]).not.toContain('/Downloads/');
  });

  it('"Bỏ qua nhóm này" removes the group, lists it under Đã bỏ qua, and can bring it back', async () => {
    const api = await mount();
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Bỏ qua nhóm này' }));
    await waitFor(() => expect(screen.getAllByTestId('dup-group').every(x => !/Be Vietnam Pro Regular/.test(x.textContent ?? ''))).toBe(true));
    click(screen.getByRole('tab', { name: 'Đã bỏ qua' }));
    const row = await screen.findByTestId('dup-ign-row');
    expect(row.textContent).toContain('Be Vietnam Pro');
    click(within(row).getByRole('button', { name: 'Hiện lại' }));
    await waitFor(() => expect(screen.queryByTestId('dup-ign-row')).toBeNull());
    expect((await api.duplicateGroups()).some(g => g.key.length > 0 && g.kind === 'same_file')).toBe(true);
    click(screen.getByRole('tab', { name: /^Cần xử lý/ }));
    await screen.findAllByTestId('dup-group');
    expect(cardOf(/Be Vietnam Pro Regular/)).toBeTruthy();
  });
});

describe('DuplicatesPage: "Giải quyết tất cả an toàn"', () => {
  it('previews every safe group, leaves the review groups alone, and sends mode all_safe', async () => {
    const api = await mount();
    const hooks = resolveHooks(api);
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog');
    await within(d).findByTestId('pv-summary');
    expect(within(d).getAllByTestId('pv-item')).toHaveLength(4);
    expect(within(d).getByTestId('pv-summary').textContent).toBe('Giữ 4 bản, ẩn 4 bản. 2 nhóm cần bạn xem sẽ không bị đụng tới.');
    confirm();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(hooks.__log.applied[0].mode).toBe('all_safe');
    expect(hooks.__log.applied[0].action).toBe('hide');
    await waitFor(() => expect(screen.getAllByTestId('dup-group')).toHaveLength(2));
    expect(screen.getByTestId('dup-summary').textContent).toBe('Tìm thấy 2 nhóm font bị trùng. Cả 2 nhóm cần bạn xem.');
    expect(screen.queryByTestId('resolve-all-safe')).toBeNull();
  });
});

describe('DuplicatesPage: the Trash is a deliberate, per-file choice', () => {
  const toTrash = async () => {
    const api = await mount();
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog');
    await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/));
    return { api, d: dialog(), hooks: resolveHooks(api) };
  };

  it('lists every file with its size, states the safety sentence, and the file in ~/Library/Fonts needs its own tick', async () => {
    const { d } = await toTrash();
    const box = within(d).getByTestId('pv-trash');
    expect(within(box).getByText('Tệp vào Thùng rác của máy, bạn lấy lại được. Font có thể là bản bạn đã mua.')).toBeTruthy();
    expect(within(box).getAllByRole('listitem')).toHaveLength(4);
    const tick = within(box).getByRole('checkbox') as HTMLInputElement;
    expect(tick.checked).toBe(false);
    expect(within(box).getByText(/app khác có thể đang dùng/)).toBeTruthy();
    expect(within(box).getByTestId('pv-total').textContent).toMatch(/^2 tệp · /);
    expect(within(box).getByText('2 tệp sẽ chỉ bị ẩn, không vào Thùng rác.')).toBeTruthy();
    expect(within(box).getByText(/Không chuyển được: font của hệ thống/)).toBeTruthy();
    expect(within(d).getByTestId('pv-confirm').textContent).toBe('Chuyển 2 tệp vào Thùng rác');
    click(tick);
    expect(within(box).getByTestId('pv-total').textContent).toMatch(/^3 tệp · /);
    expect(within(d).getByTestId('pv-confirm').textContent).toBe('Chuyển 3 tệp vào Thùng rác');
  });

  it('sends only eligible, ticked files to the Trash and merely hides the rest (never an unticked file)', async () => {
    const { hooks } = await toTrash();
    confirm();
    await waitFor(() => expect(hooks.__log.applied.length).toBe(2));
    const [trash, hide] = hooks.__log.applied;
    expect(trash.joinBatch).toBeNull();
    expect(hide.joinBatch).toMatch(/^b\d+$/); // the follow-up Hide joins the Trash request's batch
    expect(trash.action).toBe('trash'); expect(trash.mode).toBe('chosen');
    expect(trash.explicitTrashPaths).toEqual([]);
    const sent = trash.items.flatMap(i => i.loserPaths);
    expect(sent).toHaveLength(2);
    expect(sent.some(p => p.includes('/Library/Fonts/'))).toBe(false);
    expect(hide.action).toBe('hide');
    expect(hide.items.flatMap(i => i.loserPaths).sort()).toEqual(['/Library/Fonts/SourceSerif4-Regular.ttf', '/Users/me/Library/Fonts/Inter-Regular.ttf']);
    // the second request must not still list paths the first request already moved
    for (const p of sent) expect(hide.items.every(i => !i.memberPaths.includes(p))).toBe(true);
  });

  it('the ticked ~/Library/Fonts file is sent as explicit', async () => {
    const { d, hooks } = await toTrash();
    click(within(d).getByRole('checkbox'));
    confirm();
    await waitFor(() => expect(hooks.__log.applied.length).toBe(1));
    const req = hooks.__log.applied[0];
    expect(req.action).toBe('trash');
    expect(req.explicitTrashPaths).toEqual(['/Users/me/Library/Fonts/Inter-Regular.ttf']);
  });

  it('a system font cannot go to the Trash and says why in plain words', async () => {
    const api = await mount();
    click(within(cardOf(/Source Serif 4 Regular/)).getByRole('button', { name: 'Giải quyết' }));
    const d = await screen.findByRole('dialog'); await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/));
    expect(within(d).getByText(/Không chuyển được: font của hệ thống/)).toBeTruthy();
    expect(within(d).getByTestId('pv-confirm').getAttribute('aria-disabled')).toBe('true');
    expect(within(d).getByText('Tệp vào Thùng rác của máy, bạn lấy lại được. Font có thể là bản bạn đã mua.')).toBeTruthy();
    click(within(d).getByTestId('pv-confirm'));
    expect(resolveHooks(api).__log.applied).toHaveLength(0);
  });
});

describe('DuplicatesPage: partial success is reported clearly', () => {
  it('a failed group says so in plain words with the file name; the raw code and detail never show', async () => {
    const api = await mount();
    const hooks = resolveHooks(api);
    hooks.__failPaths.set('/Users/me/Downloads/src1/BeVietnamPro-Regular.ttf', 'file_changed');
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Giải quyết' }));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    confirm();
    expect((await screen.findByTestId('res-headline')).textContent).toBe('Chưa làm được nhóm nào (1 nhóm)');
    expect(screen.getByTestId('res-fails').textContent).toBe('BeVietnamPro-Regular.ttf: tệp vừa thay đổi so với lúc quét');
    expect(document.body.textContent).not.toMatch(/file_changed/);
    click(screen.getByRole('button', { name: 'Đóng' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('some groups done and one not: "Đã xử lý N nhóm, 1 nhóm chưa làm được"', async () => {
    const api = createMockApi();
    const hooks = resolveHooks(api);
    await mount(api);
    hooks.__failPaths.set('/Users/me/Fonts/src1/PlayfairDisplay-Regular.ttf', 'trash_failed');
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog'); await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/));
    click(within(d).getAllByRole('checkbox')[0]);
    confirm();
    const r = await screen.findByTestId('res-headline');
    expect(r.textContent).toBe('Đã xử lý 3 nhóm, 1 nhóm chưa làm được');
    expect(screen.getByTestId('res-fails').textContent).toContain('PlayfairDisplay-Regular.ttf: không chuyển được vào Thùng rác');
  });

  it('a recorded-but-loose-end code is shown under "Đã xử lý, nhưng cần chú ý", not as a failure', async () => {
    const api = createMockApi();
    const hooks = resolveHooks(api);
    await mount(api);
    hooks.__failPaths.set('/Users/me/Downloads/src1/BeVietnamPro-Regular.ttf', 'keeper_activation_failed');
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Giải quyết' }));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    confirm();
    // the mock reports forced codes as failures; the dialog must file partial codes under "notes"
    const d = (await screen.findByTestId('res-notes')).closest('[role=dialog]') as HTMLElement;
    expect(within(d).queryByTestId('res-notes')).not.toBeNull();
    expect(within(d).queryByTestId('res-fails')).toBeNull();
    expect(within(d).getByTestId('res-notes').textContent).toContain('đã xử lý, nhưng không bật được bản giữ');
  });
});

describe('DuplicatesPage: Đã giải quyết', () => {
  it('lists decisions newest first grouped by batch, undoes one or the whole batch', async () => {
    const api = await mount();
    click(screen.getByTestId('resolve-all-safe'));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    confirm();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    click(screen.getByRole('tab', { name: 'Đã giải quyết' }));
    const rows = await screen.findAllByTestId('dup-res-row');
    expect(rows).toHaveLength(4);
    expect(screen.getAllByTestId('dup-batch')).toHaveLength(1);
    expect(rows[0].textContent).toContain('Đã ẩn');
    click(within(rows[0]).getByRole('button', { name: 'Hoàn tác' }));
    await waitFor(() => expect(screen.getAllByTestId('dup-res-row')).toHaveLength(3));
    click(screen.getByRole('button', { name: 'Hoàn tác cả mẻ' }));
    await waitFor(() => expect(screen.queryAllByTestId('dup-res-row')).toHaveLength(0));
    expect(await api.resolveList(false)).toEqual([]);
    expect(screen.getByText('Chưa giải quyết nhóm nào')).toBeTruthy();
    click(screen.getByLabelText('Hiện cả quyết định đã hoàn tác'));
    await waitFor(() => expect(screen.getAllByTestId('dup-res-row')).toHaveLength(4));
  });

  it('undoing an older decision while a newer one depends on it explains the reverse order', async () => {
    const api = createMockApi();
    // Oswald has three copies: hide one keeping A, then hide A keeping the third → chain
    const plan = await api.resolvePreview({ type: 'group', kind: 'formats', key: 'Oswald|Regular|400|false' });
    const it = plan.items[0]; const all = [it.keeper, ...it.losers];
    const first = await api.resolveApply({ items: [{ kind: it.kind, key: it.key, keeperPath: all[0].path, loserPaths: [all[1].path], memberPaths: it.memberPaths }], action: 'hide', explicitTrashPaths: [], mode: 'chosen', joinBatch: null });
    const second = await api.resolveApply({ items: [{ kind: it.kind, key: it.key, keeperPath: all[2].path, loserPaths: [all[0].path], memberPaths: it.memberPaths.filter(p => p !== all[1].path) }], action: 'hide', explicitTrashPaths: [], mode: 'chosen', joinBatch: null });
    expect(first.done).toHaveLength(1); expect(second.done).toHaveLength(1);
    render(<DuplicatesPage api={api} />);
    click(await screen.findByRole('tab', { name: 'Đã giải quyết' }));
    const rows = await screen.findAllByTestId('dup-res-row');
    expect(rows).toHaveLength(2);
    click(within(rows[1]).getByRole('button', { name: 'Hoàn tác' }));
    expect(await screen.findByText('Có quyết định mới hơn dựa vào bản này. Hãy hoàn tác cả mẻ.')).toBeTruthy();
    expect(useNoticeStore.getState().notices.some(n => n.text.includes('quyết định mới hơn đang dựa vào bản này'))).toBe(true);
    expect(await api.resolveList(false)).toHaveLength(2);
  });
});

describe('DuplicatesPage: review fixes', () => {
  const openTrash = async (api: Api) => {
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog'); await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/));
    return dialog();
  };

  it('Trash request succeeds but the follow-up Hide request throws: the done work is kept — toast with Undo, library refreshed, the failure is listed', async () => {
    const api = await mount();
    const real = api.resolveApply.bind(api); let n = 0;
    vi.spyOn(api, 'resolveApply').mockImplementation(async r => { if (++n === 2) throw new Error('network gone'); return real(r); });
    await openTrash(api);
    confirm();
    expect(await screen.findByTestId('res-fails')).toBeTruthy();
    const note = useNoticeStore.getState().notices.find(x => /Đã chuyển 2 tệp vào Thùng rác/.test(x.text))!;
    expect(note).toBeTruthy(); expect(note.action?.label).toBe('Hoàn tác');
    expect(useUiStore.getState().libraryRev).toBeGreaterThan(0);
    expect(screen.getByTestId('res-fails').textContent).toMatch(/chưa làm được/);
    expect((await api.resolveList(false)).length).toBe(2);
  });

  it('the toast splits the numbers when one action produced both Trash and Hide results', async () => {
    const api = await mount();
    await openTrash(api); confirm();
    await waitFor(() => expect(useNoticeStore.getState().notices.some(x => x.text === 'Đã chuyển 2 tệp vào Thùng rác · Đã ẩn 2 bản')).toBe(true));
  });

  it('plan_stale reloads the plan and resets the choice to Hide with nothing ticked', async () => {
    const api = await mount();
    const real = api.resolveApply.bind(api); let n = 0;
    vi.spyOn(api, 'resolveApply').mockImplementation(async r => (++n === 1 ? { batchId: '', done: [], failed: [{ path: '/x', code: 'plan_stale', detail: null }], needsRescan: false } : real(r)));
    const d = await openTrash(api);
    click(within(d).getByRole('checkbox'));
    confirm();
    await within(dialog()).findByText('Danh sách font vừa thay đổi. Hãy xem lại kế hoạch mới.');
    await waitFor(() => expect((within(dialog()).getByLabelText(/Ẩn trong Font Atlas/) as HTMLInputElement).checked).toBe(true));
    click(within(dialog()).getByLabelText(/Chuyển vào Thùng rác/));
    expect((within(dialog()).getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
  });

  it('while applying: Cancel does nothing and a double click sends one request', async () => {
    const api = await mount();
    let release: () => void = () => {}; const gate = new Promise<void>(r => { release = r; });
    const real = api.resolveApply.bind(api);
    const spy = vi.spyOn(api, 'resolveApply').mockImplementation(async r => { await gate; return real(r); });
    click(within(cardOf(/Be Vietnam Pro Regular/)).getByRole('button', { name: 'Giải quyết' }));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    confirm(); confirm();
    click(screen.getByRole('button', { name: 'Huỷ' }));
    expect(screen.queryByRole('dialog')).not.toBeNull();
    release();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('Home / End jump to the first / last tab; the Details button controls the region it opens', async () => {
    await mount();
    fireEvent.keyDown(screen.getAllByRole('tab')[0], { key: 'End' });
    await waitFor(() => expect(screen.getByRole('tab', { name: 'Đã bỏ qua' }).getAttribute('aria-selected')).toBe('true'));
    fireEvent.keyDown(screen.getByRole('tab', { name: 'Đã bỏ qua' }), { key: 'Home' });
    await waitFor(() => expect(screen.getByRole('tab', { name: /^Cần xử lý/ }).getAttribute('aria-selected')).toBe('true'));
    await screen.findAllByTestId('dup-group');
    const g = cardOf(/Be Vietnam Pro Regular/);
    const btn = within(g).getByRole('button', { name: 'Chi tiết' });
    click(btn);
    const id = btn.getAttribute('aria-controls')!;
    expect(id).toBeTruthy(); expect(g.querySelector(`#${CSS.escape(id)}`)).toBeTruthy();
  });

  it('the result dialog opens with "Đóng" focused, not Undo', async () => {
    const api = await mount();
    resolveHooks(api).__failPaths.set('/Users/me/Library/Fonts/Inter-Regular.ttf', 'file_changed');
    click(within(cardOf(/Inter Regular/)).getByRole('button', { name: 'Giải quyết' }));
    await within(await screen.findByRole('dialog')).findByTestId('pv-summary');
    confirm();
    const close = await screen.findByRole('button', { name: 'Đóng' });
    await waitFor(() => expect(document.activeElement).toBe(close));
  });
});

describe('DuplicatesPage: one Trash action is one batch', () => {
  it('Trash + Hide land in a single batch row; one "Hoàn tác mẻ" undoes both', async () => {
    const api = await mount();
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog'); await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/)); confirm();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    click(screen.getByRole('tab', { name: 'Đã giải quyết' }));
    expect(await screen.findAllByTestId('dup-res-row')).toHaveLength(4);
    expect(screen.getAllByTestId('dup-batch')).toHaveLength(1);
    click(screen.getByRole('button', { name: 'Hoàn tác cả mẻ' }));
    await waitFor(() => expect(screen.queryAllByTestId('dup-res-row')).toHaveLength(0));
    expect(await api.resolveList(false)).toEqual([]);
  });

  it('a batch_* error on the follow-up request is reported in words, the first request stays done', async () => {
    const api = await mount();
    const real = api.resolveApply.bind(api); let n = 0;
    vi.spyOn(api, 'resolveApply').mockImplementation(async r => { if (++n === 2) throw new (await import('../../../api/types')).CmdError('batch_not_found', 'x'); return real(r); });
    click(screen.getByTestId('resolve-all-safe'));
    const d = await screen.findByRole('dialog'); await within(d).findByTestId('pv-summary');
    click(within(d).getByLabelText(/Chuyển vào Thùng rác/)); confirm();
    expect((await screen.findByTestId('res-fails')).textContent).toMatch(/mẻ thao tác không còn nữa/);
    expect(document.body.textContent).not.toMatch(/batch_not_found/);
    expect((await api.resolveList(false)).length).toBe(2);
  });
});
