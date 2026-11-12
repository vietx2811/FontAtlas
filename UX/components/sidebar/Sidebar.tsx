import { useCallback, useEffect, useRef, useState } from 'react';
import { CmdError, type Api, type CollectionDto, type SourceDto, type TagDto } from '../../api';
import type { CaseSummaryDto } from '../../bindings';
import type { ActivationReportDto } from '../../bindings';
import { applyTheme, savedTheme, syncNativeTheme } from '../../theme';
import { useScanStore } from '../../store/scan';
import { CloudDownloadControl } from './CloudDownload';
import { cancelSourceDownload, downloadSourceToMac, useCloudDownloadStore } from '../../store/cloudDownload';
import { cloudHint, notDownloadedOf } from '../zero/zeroStates';
import { dropTags, singleTagId, useUiStore } from '../../store/ui';
import { useTagsStore } from '../../store/tags';
import { useNoticeStore } from '../../store/notices';
import { ModelProgress } from './ModelProgress';
import { NestedSourceDialog, type Nesting } from './NestedSourceDialog';
import { NavItem } from './NavItem';
import { t, tn } from '../../i18n';
import { NavSection } from './NavSection';
import { Icon } from './icons';
import { isFontFile, parentDir, pickFolderWithDialog, useDropPaths } from './useDropPaths';
import './sidebar.css';
import { Tooltip } from '../ui/Tooltip';
import { Menu, type MenuAnchor } from '../ui/Menu';
import { Modal } from '../Modal';
import { CONFIRM_ACTIVATE_FROM, CONFIRM_DEACTIVATE_FROM, buildSidebarMenu, type SidebarMenuKind } from './sidebarMenu';
import { NoteMark } from '../ui/noteMark';
import { RemoveSourceDialog } from '../sources/RemoveSourceDialog';
import { focusLibraryGrid } from '../sources/focusLibrary';
import { useActivationFlow } from '../activation/useActivationFlow';

type Props = { api: Api; pickFolder?: () => Promise<string | null>; onOpenSettings?: () => void };
type Counts = { all: number | null; star: number | null; dups: number | null };
type Nested = { newPath: string; existing: SourceDto; nesting: Nesting };
type Ctx = { kind: SidebarMenuKind; id: number | null; name: string; anchor: MenuAnchor; el: HTMLElement; keyboard: boolean; colActive?: number };
type Confirm = { title: string; body: string; ok: string; run(): Promise<void> | void };
type Notice = { text: string; action?: { label: string; run(): void } };

/** Tag hệ thống (style:/mood:/use:): do danh mục sở hữu, không đổi tên/xoá từ thanh bên. */
const isSystemTag = (name: string): boolean => /^(style|mood|use):/.test(name);
const tl = t; // `t` bị che bởi tham số Thẻ trong vòng lặp
const prefersDark = () => typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches;

export const cloudTip = (): string => t('side.cloudTip');

export function SidebarToggle(): React.JSX.Element {
  const collapsed = useUiStore(s => s.sidebarCollapsed);
  const toggle = useUiStore(s => s.toggleSidebar);
  return (
    <Tooltip label={collapsed ? t('side.expand') : t('side.collapse')}>
      <button type="button" className="fa-iconbtn" aria-pressed={collapsed} aria-label={collapsed ? t('side.expand') : t('side.collapse')} onClick={toggle}>
        <Icon name="panel" />
      </button>
    </Tooltip>
  );
}

export function Sidebar({ api, pickFolder = pickFolderWithDialog, onOpenSettings }: Props): React.JSX.Element {
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => { const id = singleTagId(s.filters); return id === null ? null : String(id); });
  const open = useUiStore(s => s.openSections);
  const collapsed = useUiStore(s => s.sidebarCollapsed);
  const scanVersion = useScanStore(s => s.version);
  const scanning = useScanStore(s => s.scanning);
  const scanned = useScanStore(s => s.written);
  const access = useScanStore(s => s.access);
  const cloud = useScanStore(s => s.cloud);
  const dlRuns = useCloudDownloadStore(s => s.runs);
  const cloudNow = Object.values(cloud).reduce((a, b) => a + b, 0);
  const addRequest = useUiStore(s => s.addFolderRequest);
  const cmpSession = useUiStore(s => s.compareSession);
  const cmpOpen = useUiStore(s => s.compare !== null);
  const libraryRev = useUiStore(s => s.libraryRev);
  // "Đang bật" đếm theo store (nạp lại sau mỗi lần bật/tắt), không theo lần nạp thanh bên: bật xong con số phải đổi ngay.
  const activeCount = useUiStore(s => s.active.size);
  const { setView, setTag, toggleSection } = useUiStore.getState();

  const [sources, setSources] = useState<SourceDto[]>([]);
  const [cols, setCols] = useState<CollectionDto[]>([]);
  const [tags, setTags] = useState<TagDto[]>([]);
  const [cases, setCases] = useState<CaseSummaryDto[]>([]);
  const [counts, setCounts] = useState<Counts>({ all: null, star: null, dups: null });
  const [notice, setNotice] = useState<Notice | null>(null);
  const [nested, setNested] = useState<Nested | null>(null);
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<number | null>(null);
  const [renamingTag, setRenamingTag] = useState<number | null>(null);
  const [ctx, setCtx] = useState<Ctx | null>(null);
  const [confirm, setConfirm] = useState<Confirm | null>(null);
  const activation = useActivationFlow(api);
  const [removingSrc, setRemovingSrc] = useState<SourceDto | null>(null);
  const recentCount = useUiStore(s => s.recentViews.length);
  const scanningNow = useScanStore(s => s.scanning);

  const refresh = useCallback(async () => {
    const [s, c, t, all, star, dup, cs] = await Promise.all([
      api.listSources(), api.listCollections(), api.listTags(),
      api.queryFaces({ limit: 1 }), api.queryFaces({ favoritesOnly: true, limit: 1 }),
      api.duplicateSummary({}).catch(() => null), // lỗi đọc trùng không làm hỏng thanh bên
      api.listCases().catch(() => [] as CaseSummaryDto[]), // Case lỗi không làm hỏng thanh bên
    ]);
    setSources(s); setCols(c); setTags(t); setCases([...cs].sort((a, b) => b.openedAt - a.openedAt)); useTagsStore.getState().set(t);
    // Thẻ đã xoá nơi khác mà bộ lọc còn giữ id: dọn, để không lọc ra rỗng với chip vô danh.
    const have = new Set(t.map(x => x.id)); const cur = useUiStore.getState().filters;
    if (cur.tagIds?.some(id => !have.has(id))) useUiStore.getState().setFilters(cur.tagIds.every(id => !have.has(id)) ? dropTags(cur) : { ...cur, tagIds: cur.tagIds.filter(id => have.has(id)) });
    setCounts({ all: all.total, star: star.total, dups: dup ? dup.total : null });
  }, [api]);
  useEffect(() => { refresh().catch(e => setNotice({ text: t('side.loadFail', { msg: e?.message ?? e }) })); }, [refresh, scanVersion, libraryRev]);

  const addPath = useCallback(async (path: string) => {
    try {
      const s = await api.addSource(path);
      await refresh();
      setView(`src:${s.id}`);
      setNotice(null);
    } catch (e) {
      if (e instanceof CmdError && e.code === 'nested_source') {
        const d = (e.detail ?? {}) as { existingId?: number; existingPath?: string };
        const existing = (await api.listSources()).find(x => x.id === d.existingId);
        const norm = (p: string) => p.replace(/[\\/]+$/, '');
        if (!existing?.path) setNotice({ text: t('side.nested') });
        else if (norm(path) === norm(existing.path)) {
          setNotice({ text: t('side.exists', { name: existing.name }), action: { label: t('side.viewSource'), run: () => { setView(`src:${existing.id}`); setNotice(null); } } });
        } else setNested({ newPath: path, existing, nesting: norm(path).startsWith(`${norm(existing.path)}/`) ? 'inside' : 'contains' });
      } else setNotice({ text: t('side.addFail', { msg: (e as Error).message }) });
    }
  }, [api, refresh, sources, setView]);

  const replaceSource = async (n: Nested) => {
    setNested(null);
    try { await api.removeSource(n.existing.id); await addPath(n.newPath); }
    catch (e) { setNotice({ text: t('side.replaceFail', { msg: (e as Error).message }) }); }
  };
  const onAddFont = async () => { const p = await pickFolder(); if (p) await addPath(p); };
  const lastAddRequest = useRef(addRequest);
  useEffect(() => { if (addRequest !== lastAddRequest.current) { lastAddRequest.current = addRequest; void onAddFont(); } }); // eslint-disable-line react-hooks/exhaustive-deps
  const onDrop = useCallback((paths: string[]) => {
    const [first] = paths;
    if (!first) return;
    if (!isFontFile(first)) { void addPath(first); return; }
    const dir = parentDir(first);
    setNotice({ text: t('side.addFolder', { dir: String(dir.split(/[\\/]/).pop()) }), action: { label: t('side.add'), run: () => { setNotice(null); void addPath(dir); } } });
  }, [addPath]);
  useDropPaths(onDrop);

  /** Bấm Case: mở trong Pairing Desk ở Combo chính (không có Combo chính thì Combo đầu). */
  const openCaseInDesk = async (id: number) => {
    try {
      const c = await api.openCase(id);
      const combo = c.combos.find(x => x.isMain) ?? c.combos[0];
      if (!combo) { setNotice({ text: t('side.caseEmpty') }); return; }
      useUiStore.getState().openDesk({ caseId: c.id, comboId: combo.id, role: combo.slots[0]?.role ?? 'display', trial: null });
    } catch (e) { warn(e); }
  };
  const createCollection = async (name: string) => {
    setAdding(false);
    if (!name.trim()) return;
    const c = await api.createCollection(name.trim(), null);
    await refresh();
    setView(`col:${c.id}`);
  };
  const focusNav = (navId: string) => requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-nav-id="${navId}"]`)?.focus());
  const renameBusy = useRef(false);
  const endRename = (id: number, refocus: boolean) => { setRenaming(null); if (refocus) focusNav(`col:${id}`); };
  /** Enter / blur: rỗng → báo và giữ ô; lỗi → báo và giữ ô (không mất tên đã gõ); xong → trả focus về mục. */
  const commitRename = async (c: CollectionDto, raw: string, input: HTMLInputElement, via: 'enter' | 'blur') => {
    if (renameBusy.current) return;
    const name = raw.trim();
    if (name === c.name) { endRename(c.id, via === 'enter'); return; }
    if (!name) {
      if (via === 'blur') { endRename(c.id, false); return; }
      setNotice({ text: t('smenu.rename.empty') }); input.focus(); return;
    }
    renameBusy.current = true;
    try { await api.renameCollection(c.id, name); await refresh(); endRename(c.id, via === 'enter'); }
    catch (e) { warn(e); if (via === 'enter') input.focus(); }
    finally { renameBusy.current = false; }
  };
  const removeCollection = async (id: number) => {
    await api.deleteCollection(id);
    if (useUiStore.getState().view === `col:${id}`) setView('all');
    await refresh();
  };
  const focusTag = (id: number) => requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-tag-id="${id}"]`)?.focus());
  const tagRenameBusy = useRef(false);
  const endTagRename = (id: number, refocus: boolean) => { setRenamingTag(null); if (refocus) focusTag(id); };
  /** Enter / blur: rỗng hoặc lỗi → báo và giữ ô (không mất tên đã gõ); trùng tên → báo rõ, không gộp ngầm. */
  const commitTagRename = async (tg: TagDto, raw: string, input: HTMLInputElement, via: 'enter' | 'blur') => {
    if (tagRenameBusy.current) return;
    const name = raw.normalize('NFC').trim(); // cùng dạng với tên lưu (NFC): gõ "Phở" ở dạng tổ hợp không bị coi là đổi tên
    if (name === tg.name.normalize('NFC')) { endTagRename(tg.id, via === 'enter'); return; }
    if (!name) {
      if (via === 'blur') { endTagRename(tg.id, false); return; }
      setNotice({ text: t('smenu.tag.empty') }); input.focus(); return;
    }
    tagRenameBusy.current = true;
    try { await api.renameTag(tg.id, name); await refresh(); endTagRename(tg.id, via === 'enter'); }
    catch (e) {
      const code = e instanceof CmdError ? e.code : '';
      setNotice({ text: code === 'conflict' ? t('smenu.tag.conflict', { name }) : code === 'invalid_arg' ? t('smenu.tag.invalid') : code === 'not_found' ? t('smenu.tag.gone') : t('smenu.tag.fail', { msg: (e as Error)?.message ?? String(e) }) });
      if (code === 'not_found') { endTagRename(tg.id, false); await refresh().catch(() => undefined); } else if (via === 'enter') input.focus(); else endTagRename(tg.id, false);
    } finally { tagRenameBusy.current = false; }
  };
  const removeTag = async (tg: TagDto) => {
    const n = await api.deleteTag(tg.id);
    useNoticeStore.getState().push({ kind: 'info', text: t('smenu.delTag.done', { name: tg.name, n }) });
    await refresh();
  };
  const at = (e: { x: number; y: number; keyboard: boolean; el: HTMLElement }): MenuAnchor => (e.keyboard ? { rect: e.el.getBoundingClientRect() } : { x: e.x, y: e.y });
  const openCtx = (kind: SidebarMenuKind, id: number | null, name: string) => (e: { x: number; y: number; keyboard: boolean; el: HTMLElement }) => {
    setCtx({ kind, id, name, anchor: at(e), el: e.el, keyboard: e.keyboard });
    // Dự án: đếm font ĐANG BẬT (không phải số font trong dự án) cho nhãn, ngưỡng xác nhận và trạng thái của "Tắt tất cả".
    if (kind === 'collection' && id !== null) {
      void api.queryFaces({ collectionId: id, limit: 20000 }).then(p => {
        const on = useUiStore.getState().active;
        const n = new Set(p.items.filter(r => on.has(r.hashHex)).map(r => r.hashHex)).size;
        setCtx(cur => (cur && cur.kind === 'collection' && cur.id === id ? { ...cur, colActive: n } : cur));
      }, () => undefined);
    }
  };
  const warn = (e: unknown) => setNotice({ text: t('side.opFail', { msg: (e as Error)?.message ?? String(e) }) });
  /** Báo kết quả tắt hàng loạt: đã tắt / lỗi / còn bật vì còn được giữ / cần xác nhận. Không im lặng khi có phần không tắt được. */
  const reportOff = (rep: ActivationReportDto) => {
    const failed = rep.failed; const kept = rep.results.filter(r => r.outcome.kind === 'kept_active' || r.outcome.kind === 'held_by_collections').length;
    const parts = [failed ? tn('smenu.off.doneErr', { n: rep.changed, m: failed }) : tn('smenu.off.done', { n: rep.changed })];
    if (kept) parts.push(t('smenu.off.kept', { k: kept }));
    if (rep.needsConfirmation) parts.push(t('smenu.off.confirm', { c: rep.needsConfirmation }));
    useNoticeStore.getState().push({ kind: failed || rep.needsConfirmation ? 'error' : 'info', text: parts.join('. ') });
  };
  const deactivate = (n: number, run: () => Promise<ActivationReportDto>) => {
    const go = async () => { try { reportOff(await run()); } catch (e) { warn(e); } await useUiStore.getState().refreshActive(api).catch(() => undefined); };
    if (n >= CONFIRM_DEACTIVATE_FROM) setConfirm({ title: tn('smenu.off.title', { n }), body: t('smenu.off.body'), ok: t('smenu.off.ok'), run: go }); else void go();
  };
  const menuEntries = (c: Ctx) => {
    const col = c.kind === 'collection' ? cols.find(x => x.id === c.id) : undefined;
    const src = c.kind === 'source' ? sources.find(x => x.id === c.id) : undefined;
    const tg = c.kind === 'tag' ? tags.find(x => x.id === c.id) : undefined;
    const goView = (v: string) => () => setView(v as never);
    return buildSidebarMenu(c.kind, {
      open: c.kind === 'compare' ? () => useUiStore.getState().resumeCompare() : goView(c.kind === 'source' ? `src:${c.id}` : c.kind === 'collection' ? `col:${c.id}` : c.kind),
      sourceAvailable: src?.available, hasPath: !!src?.path, scanning: scanningNow,
      reveal: () => { if (src) api.revealSource(src.id).catch(warn); },
      rescan: () => { if (src) api.rescanSources([src.id]).catch(warn); },
      cloudSource: !!src && (!!src.cloudProvider || notDownloadedOf(src, cloud) > 0), cloudLeft: src ? notDownloadedOf(src, cloud) : 0, downloading: !!src && src.id in dlRuns,
      downloadCloud: () => { if (src) void downloadSourceToMac(api, src); },
      cancelDownload: () => { if (src) void cancelSourceDownload(api, src.id); },
      removeSource: () => { if (src) setRemovingSrc(src); },
      rename: () => { if (col) setRenaming(col.id); },
      colActive: c.colActive, colSize: col?.itemCount,
      activateAll: () => { if (col) { const n = col.itemCount; if (n > CONFIRM_ACTIVATE_FROM) setConfirm({ title: t('act.all.title', { n }), body: t('act.all.body'), ok: t('act.all.ok'), run: () => activation.activateCollection(col.id) }); else void activation.activateCollection(col.id); } },
      deactivateAll: c.kind === 'active' ? () => deactivate(activeCount, () => api.activationDeactivateAll()) : () => { if (col) deactivate(c.colActive ?? 0, () => api.activationDeactivateCollection(col.id)); },
      remove: () => { if (col) setConfirm({ title: t('smenu.delCol.title', { name: col.name }), body: t('smenu.delCol.body'), ok: t('smenu.delCol.ok'), run: () => removeCollection(col.id).catch(warn) }); },
      filterByTag: () => setTag(String(c.id)),
      renameTag: tg && !isSystemTag(tg.name) ? () => setRenamingTag(tg.id) : undefined,
      deleteTag: tg && isSystemTag(tg.name) ? undefined : () => { if (tg) setConfirm({ title: t('smenu.delTag.title', { name: tg.name }), body: t('smenu.delTag.body', { n: tg.faceCount }), ok: t('smenu.delTag.ok'), run: () => removeTag(tg).catch(warn) }); },
      recentCount, clearRecent: () => setConfirm({ title: t('smenu.clearRecent.title'), body: t('smenu.clearRecent.body'), ok: t('smenu.clearRecent.ok'), run: () => useUiStore.getState().clearRecent() }),
      activeCount, closeSession: () => useUiStore.getState().closeCompareSession(),
    });
  };

  const [dark, setDark] = useState(false);
  useEffect(() => {
    const saved = savedTheme();
    if (saved !== 'system') document.documentElement.dataset.theme = saved;
    syncNativeTheme(saved);
    setDark(saved === 'dark' || (saved === 'system' && prefersDark()));
  }, []);
  const toggleTheme = () => {
    applyTheme(dark ? 'light' : 'dark');
    setDark(!dark);
  };

  const cur = (v: string) => view === v;
  return (
    <>
    {nested && <NestedSourceDialog {...nested} onCancel={() => setNested(null)} onReplace={() => void replaceSource(nested)}
      onCreateCollection={name => { setNested(null); void createCollection(name); }} />}
    <nav className="fa-side" data-testid="sidebar" data-collapsed={collapsed} aria-label={t('side.nav')}>
      <div className="fa-side-scroll">
        <NavItem icon="plus" label={t('side.addFont')} onClick={onAddFont} />
        <NavItem icon="all" label={t('side.all')} count={counts.all} current={cur('all')} onClick={() => setView('all')} />
        <NavItem icon="star" label={t('side.favorites')} count={counts.star} current={cur('star')} onClick={() => setView('star')} />
        <NavItem icon="clock" label={t('side.recent')} current={cur('recent')} onClick={() => setView('recent')} onContextMenu={openCtx('recent', null, t('side.recent'))} context={ctx?.kind === 'recent'} />
        <NavItem icon="power" label={t('side.active')} count={activeCount} current={cur('active')} onClick={() => setView('active')} onContextMenu={openCtx('active', null, t('side.active'))} context={ctx?.kind === 'active'} />
        {((counts.dups ?? 0) > 0 || cur('dups')) && <NavItem icon="copy" label={t('dup.title')} count={counts.dups} current={cur('dups')} onClick={() => setView('dups')} />}
        {cmpSession && <NavItem icon="compare" label={t('compare.session', { n: cmpSession.faces.length })} title={t('compare.session.tip', { n: cmpSession.faces.length })} current={cmpOpen} onClick={() => useUiStore.getState().resumeCompare()} onContextMenu={openCtx('compare', null, t('compare.session', { n: cmpSession.faces.length }))} context={ctx?.kind === 'compare'} />}

        {notice && (
          <div className="fa-notice" role="status">
            <span>{notice.text}</span>
            {notice.action && <button type="button" onClick={notice.action.run}>{notice.action.label}</button>}
            <button type="button" aria-label={t('side.dismiss')} onClick={() => setNotice(null)}>×</button>
          </div>
        )}

        <NavSection title={t('side.projects')} open={open.col} onToggle={() => toggleSection('col')} addLabel={t('side.addProject')} onAdd={() => { if (!open.col) toggleSection('col'); setAdding(true); }}>
          {adding && <input className="fa-inline-input" aria-label={t('side.projectName')} autoFocus placeholder={t('side.projectName')}
            onKeyDown={e => { if (e.key === 'Enter') void createCollection(e.currentTarget.value); if (e.key === 'Escape') setAdding(false); }} onBlur={() => setAdding(false)} />}
          {cols.map(c => renaming === c.id
            ? <input key={c.id} className="fa-inline-input" aria-label={t('side.renameProject')} autoFocus defaultValue={c.name}
                onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); void commitRename(c, e.currentTarget.value, e.currentTarget, 'enter'); } else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); endRename(c.id, true); } }} onBlur={e => void commitRename(c, e.currentTarget.value, e.currentTarget, 'blur')} />
            : <NavItem key={c.id} swatch={c.color} label={c.name} count={c.itemCount} drop={{ id: c.id, name: c.name, here: cur(`col:${c.id}`) }} current={cur(`col:${c.id}`)} onClick={() => setView(`col:${c.id}`)}
navId={`col:${c.id}`} onContextMenu={openCtx('collection', c.id, c.name)} onRename={() => setRenaming(c.id)} context={ctx?.kind === 'collection' && ctx.id === c.id}
                more={<Tooltip label={t('side.options', { name: c.name })}><button type="button" className="fa-more" aria-label={t('side.options', { name: c.name })} aria-haspopup="menu" aria-expanded={ctx?.kind === 'collection' && ctx.id === c.id}
                  onClick={e => { const el = e.currentTarget; openCtx('collection', c.id, c.name)({ x: 0, y: 0, keyboard: true, el }); }}><Icon name="more" size={14} /></button></Tooltip>} />)}
        </NavSection>

        {cases.length > 0 && (
          <NavSection title={t('side.cases')} open={open.case} onToggle={() => toggleSection('case')}>
            {cases.map(c => (
              <NavItem key={c.id} icon="pair" label={c.name} count={c.faceCount} title={t('side.caseTip', { name: c.name })} navId={`case:${c.id}`}
                drop={{ kind: 'case', id: c.id, name: c.name, here: useUiStore.getState().desk?.caseId === c.id }} onClick={() => void openCaseInDesk(c.id)} />
            ))}
          </NavSection>
        )}

        <NavSection title={t('side.sources')} open={open.src} onToggle={() => toggleSection('src')}>
          {sources.map(s => (
            <NavItem key={s.id} icon={s.id in access ? 'lock' : s.available ? 'folder' : 'eject'} label={s.name} count={s.faceCount} unavailable={!s.available}
              title={s.id in access ? t('side.needsAccess', { name: s.name }) : s.available ? s.path ?? s.name : t('side.notConnected', { name: s.name })} current={cur(`src:${s.id}`)} onClick={() => setView(`src:${s.id}`)} onContextMenu={openCtx('source', s.id, s.name)} context={ctx?.kind === 'source' && ctx.id === s.id}
              sub={notDownloadedOf(s, cloud) > 0 || cloudHint(s) || s.id in dlRuns
                ? <>
                    {notDownloadedOf(s, cloud) > 0 && <Tooltip label={cloudTip()}><div className="fa-cloud" data-testid="not-downloaded"><Icon name="cloud" size={12} />{tn('side.notDownloaded', { n: notDownloadedOf(s, cloud) })}</div></Tooltip>}
                    {(notDownloadedOf(s, cloud) > 0 || s.id in dlRuns) && <div className="fa-cloud fa-cloud-dl"><CloudDownloadControl api={api} source={s} notDownloaded={notDownloadedOf(s, cloud)} /></div>}
                    {cloudHint(s) && <div className="fa-cloud"><NoteMark kind="info" inline testId="cloud-hint" text={cloudHint(s)!} /></div>}
                  </>
                : undefined} />
          ))}
        </NavSection>

        <NavSection title={t('side.tags')} open={open.tag} onToggle={() => toggleSection('tag')}>
          <div className="fa-tags">
            {tags.map(t => renamingTag === t.id
              ? <input key={t.id} className="fa-inline-input fa-tag-input" data-testid="tag-rename" aria-label={tl('smenu.tag.renameLabel')} autoFocus defaultValue={t.name}
                  onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); void commitTagRename(t, e.currentTarget.value, e.currentTarget, 'enter'); } else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); endTagRename(t.id, true); } }}
                  onBlur={e => void commitTagRename(t, e.currentTarget.value, e.currentTarget, 'blur')} />
              : (
              <button key={t.id} data-tag-id={t.id} type="button" className="fa-tag" aria-pressed={tag === String(t.id)} data-context={ctx?.kind === 'tag' && ctx.id === t.id ? 'true' : undefined}
                onContextMenu={e => { e.preventDefault(); openCtx('tag', t.id, t.name)({ x: e.clientX, y: e.clientY, keyboard: false, el: e.currentTarget }); }}
                onKeyDown={e => { if (e.key === 'F2' && !isSystemTag(t.name)) { e.preventDefault(); setRenamingTag(t.id); } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) { e.preventDefault(); openCtx('tag', t.id, t.name)({ x: 0, y: 0, keyboard: true, el: e.currentTarget }); } }} onClick={() => setTag(tag === String(t.id) ? null : String(t.id))}>{t.name}</button>
            ))}
          </div>
        </NavSection>
      </div>
      <div className="fa-side-foot">
        {scanning && <div className="fa-scan" role="status" aria-label={t('side.scanning')}><span className="fa-scan-dot" aria-hidden />{t('side.scanning')}{scanned > 0 ? tn('side.scanningN', { n: scanned }) : ''}{cloudNow > 0 && <Tooltip label={cloudTip()}><span className="fa-scan-cloud" data-testid="scan-not-downloaded"><Icon name="cloud" size={12} />{tn('side.notDownloaded', { n: cloudNow })}</span></Tooltip>}</div>}
        <ModelProgress api={api} />
        <div className="fa-foot-actions">
          <Tooltip label={dark ? t('side.themeLight') : t('side.themeDark')}><button type="button" className="fa-iconbtn" aria-label={t('side.themeToggle')} aria-pressed={dark} onClick={toggleTheme}><Icon name="moon" /></button></Tooltip>
          <Tooltip label={t('side.settings')} shortcut="⌘,"><button type="button" className="fa-iconbtn" aria-label={t('side.settings')} onClick={() => (onOpenSettings ?? useUiStore.getState().toggleSettings)()}><Icon name="gear" /></button></Tooltip>
        </div>
      </div>
    </nav>
    {removingSrc && <RemoveSourceDialog api={api} source={removingSrc} onClose={() => { setRemovingSrc(null); focusLibraryGrid(); }}
      onRemoved={rep => {
        const s = removingSrc; setRemovingSrc(null); focusLibraryGrid();
        if (useUiStore.getState().view === `src:${s.id}`) setView('all');
        useNoticeStore.getState().push({ kind: 'info', text: t('rmsrc.done', { name: s.name, n: rep.removedLocations }) });
        useUiStore.getState().bumpLibrary(true);
        void refresh();
      }} />}
    {ctx && <Menu entries={menuEntries(ctx)} anchor={ctx.anchor} label={t('smenu.menuOf', { name: ctx.name })} focusFirst={ctx.keyboard} returnFocus={ctx.el} onClose={() => setCtx(null)} />}
    {confirm && (
      <Modal title={confirm.title} onClose={() => setConfirm(null)}>
        <p>{confirm.body}</p>
        <div className="fa-modal-actions">
          <button type="button" data-testid="smenu-cancel" onClick={() => setConfirm(null)}>{t('smenu.cancel')}</button>
          <button type="button" className="primary" data-testid="smenu-confirm" onClick={() => { const c = confirm; setConfirm(null); void Promise.resolve(c.run()); }}>{confirm.ok}</button>
        </div>
      </Modal>
    )}
    </>
  );
}
