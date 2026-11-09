import { useCallback, useEffect, useRef, useState } from 'react';
import { CmdError, type Api, type CollectionDto, type SourceDto, type TagDto } from '../../api';
import { applyTheme, savedTheme, syncNativeTheme } from '../../theme';
import { useScanStore } from '../../store/scan';
import { notDownloadedOf } from '../zero/zeroStates';
import { useUiStore } from '../../store/ui';
import { ModelProgress } from './ModelProgress';
import { NestedSourceDialog, type Nesting } from './NestedSourceDialog';
import { NavItem } from './NavItem';
import { t } from '../../i18n';
import { NavSection } from './NavSection';
import { Icon } from './icons';
import { isFontFile, parentDir, pickFolderWithDialog, useDropPaths } from './useDropPaths';
import './sidebar.css';
import { Tooltip } from '../ui/Tooltip';

type Props = { api: Api; pickFolder?: () => Promise<string | null>; onOpenSettings?: () => void };
type Counts = { all: number | null; star: number | null; dups: number | null };
type Nested = { newPath: string; existing: SourceDto; nesting: Nesting };
type Notice = { text: string; action?: { label: string; run(): void } };

const prefersDark = () => typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches;

export const CLOUD_TIP = 'File đang ở iCloud/OneDrive, chưa tải về máy nên chưa quét được. Trong Finder: bấm chuột phải › Tải về, rồi quét lại.';

export function SidebarToggle(): React.JSX.Element {
  const collapsed = useUiStore(s => s.sidebarCollapsed);
  const toggle = useUiStore(s => s.toggleSidebar);
  return (
    <Tooltip label={collapsed ? 'Mở rộng thanh bên' : 'Thu gọn thanh bên'}>
      <button type="button" className="fa-iconbtn" aria-pressed={collapsed} aria-label={collapsed ? 'Mở rộng thanh bên' : 'Thu gọn thanh bên'} onClick={toggle}>
        <Icon name="panel" />
      </button>
    </Tooltip>
  );
}

export function Sidebar({ api, pickFolder = pickFolderWithDialog, onOpenSettings }: Props): React.JSX.Element {
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => s.tag);
  const open = useUiStore(s => s.openSections);
  const collapsed = useUiStore(s => s.sidebarCollapsed);
  const scanVersion = useScanStore(s => s.version);
  const scanning = useScanStore(s => s.scanning);
  const scanned = useScanStore(s => s.written);
  const access = useScanStore(s => s.access);
  const cloud = useScanStore(s => s.cloud);
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
  const [counts, setCounts] = useState<Counts>({ all: null, star: null, dups: null });
  const [notice, setNotice] = useState<Notice | null>(null);
  const [nested, setNested] = useState<Nested | null>(null);
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<number | null>(null);
  const [menuFor, setMenuFor] = useState<number | null>(null);

  const refresh = useCallback(async () => {
    const [s, c, t, all, star, dup] = await Promise.all([
      api.listSources(), api.listCollections(), api.listTags(),
      api.queryFaces({ limit: 1 }), api.queryFaces({ favoritesOnly: true, limit: 1 }),
      api.duplicateSummary({}).catch(() => null), // lỗi đọc trùng không làm hỏng thanh bên
    ]);
    setSources(s); setCols(c); setTags(t);
    setCounts({ all: all.total, star: star.total, dups: dup ? dup.total : null });
  }, [api]);
  useEffect(() => { refresh().catch(e => setNotice({ text: `Không tải được thanh bên: ${e?.message ?? e}` })); }, [refresh, scanVersion, libraryRev]);

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
        if (!existing?.path) setNotice({ text: 'Thư mục này đã nằm trong một nguồn có sẵn.' });
        else if (norm(path) === norm(existing.path)) {
          setNotice({ text: `Đã có trong thư viện (nguồn «${existing.name}»).`, action: { label: 'Xem nguồn', run: () => { setView(`src:${existing.id}`); setNotice(null); } } });
        } else setNested({ newPath: path, existing, nesting: norm(path).startsWith(`${norm(existing.path)}/`) ? 'inside' : 'contains' });
      } else setNotice({ text: `Không thêm được: ${(e as Error).message}` });
    }
  }, [api, refresh, sources, setView]);

  const replaceSource = async (n: Nested) => {
    setNested(null);
    try { await api.removeSource(n.existing.id); await addPath(n.newPath); }
    catch (e) { setNotice({ text: `Không thay được nguồn: ${(e as Error).message}` }); }
  };
  const onAddFont = async () => { const p = await pickFolder(); if (p) await addPath(p); };
  const lastAddRequest = useRef(addRequest);
  useEffect(() => { if (addRequest !== lastAddRequest.current) { lastAddRequest.current = addRequest; void onAddFont(); } }); // eslint-disable-line react-hooks/exhaustive-deps
  const onDrop = useCallback((paths: string[]) => {
    const [first] = paths;
    if (!first) return;
    if (!isFontFile(first)) { void addPath(first); return; }
    const dir = parentDir(first);
    setNotice({ text: `Thêm cả thư mục «${dir.split(/[\\/]/).pop()}» chứa font này?`, action: { label: 'Thêm', run: () => { setNotice(null); void addPath(dir); } } });
  }, [addPath]);
  useDropPaths(onDrop);

  const createCollection = async (name: string) => {
    setAdding(false);
    if (!name.trim()) return;
    const c = await api.createCollection(name.trim(), null);
    await refresh();
    setView(`col:${c.id}`);
  };
  const rename = async (id: number, name: string) => { setRenaming(null); if (name.trim()) { await api.renameCollection(id, name.trim()); await refresh(); } };
  const remove = async (id: number) => {
    setMenuFor(null);
    await api.deleteCollection(id);
    if (view === `col:${id}`) setView('all');
    await refresh();
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
    <nav className="fa-side" data-testid="sidebar" data-collapsed={collapsed} aria-label="Thanh bên">
      <div className="fa-side-scroll">
        <NavItem icon="plus" label="Thêm font" onClick={onAddFont} />
        <NavItem icon="all" label="Tất cả" count={counts.all} current={cur('all')} onClick={() => setView('all')} />
        <NavItem icon="star" label="Yêu thích" count={counts.star} current={cur('star')} onClick={() => setView('star')} />
        <NavItem icon="clock" label="Gần đây" current={cur('recent')} onClick={() => setView('recent')} />
        <NavItem icon="power" label="Đang bật" count={activeCount} current={cur('active')} onClick={() => setView('active')} />
        {((counts.dups ?? 0) > 0 || cur('dups')) && <NavItem icon="copy" label={t('dup.title')} count={counts.dups} current={cur('dups')} onClick={() => setView('dups')} />}
        {cmpSession && <NavItem icon="compare" label={t('compare.session', { n: cmpSession.faces.length })} title={t('compare.session.tip', { n: cmpSession.faces.length })} current={cmpOpen} onClick={() => useUiStore.getState().resumeCompare()} />}

        {notice && (
          <div className="fa-notice" role="status">
            <span>{notice.text}</span>
            {notice.action && <button type="button" onClick={notice.action.run}>{notice.action.label}</button>}
            <button type="button" aria-label="Đóng thông báo" onClick={() => setNotice(null)}>×</button>
          </div>
        )}

        <NavSection title="Dự án" open={open.col} onToggle={() => toggleSection('col')} addLabel="Thêm dự án" onAdd={() => { if (!open.col) toggleSection('col'); setAdding(true); }}>
          {adding && <input className="fa-inline-input" aria-label="Tên dự án" autoFocus placeholder="Tên dự án"
            onKeyDown={e => { if (e.key === 'Enter') void createCollection(e.currentTarget.value); if (e.key === 'Escape') setAdding(false); }} onBlur={() => setAdding(false)} />}
          {cols.map(c => renaming === c.id
            ? <input key={c.id} className="fa-inline-input" aria-label="Đổi tên dự án" autoFocus defaultValue={c.name}
                onKeyDown={e => { if (e.key === 'Enter') void rename(c.id, e.currentTarget.value); if (e.key === 'Escape') setRenaming(null); }} onBlur={() => setRenaming(null)} />
            : <NavItem key={c.id} swatch={c.color} label={c.name} count={c.itemCount} current={cur(`col:${c.id}`)} onClick={() => setView(`col:${c.id}`)}
                more={<>
                  <Tooltip label={`Tuỳ chọn ${c.name}`}><button type="button" className="fa-more" aria-label={`Tuỳ chọn ${c.name}`} aria-haspopup="menu" aria-expanded={menuFor === c.id} onClick={() => setMenuFor(menuFor === c.id ? null : c.id)}><Icon name="more" size={16} /></button></Tooltip>
                  {menuFor === c.id && <div role="menu" className="fa-menu">
                    <button role="menuitem" type="button" onClick={() => { setMenuFor(null); setRenaming(c.id); }}>Đổi tên</button>
                    <button role="menuitem" type="button" onClick={() => void remove(c.id)}>Xoá dự án</button>
                  </div>}
                </>} />)}
        </NavSection>

        <NavSection title="Nguồn" open={open.src} onToggle={() => toggleSection('src')}>
          {sources.map(s => (
            <NavItem key={s.id} icon={s.id in access ? 'lock' : s.available ? 'folder' : 'eject'} label={s.name} count={s.faceCount} unavailable={!s.available}
              title={s.id in access ? `${s.name}: cần quyền truy cập` : s.available ? s.path ?? s.name : `${s.name}: chưa kết nối`} current={cur(`src:${s.id}`)} onClick={() => setView(`src:${s.id}`)}
              sub={notDownloadedOf(s, cloud) > 0 ? <Tooltip label={CLOUD_TIP}><div className="fa-cloud" data-testid="not-downloaded"><Icon name="cloud" size={12} />{notDownloadedOf(s, cloud)} font chưa tải về</div></Tooltip> : undefined} />
          ))}
        </NavSection>

        <NavSection title="Thẻ" open={open.tag} onToggle={() => toggleSection('tag')}>
          <div className="fa-tags">
            {tags.map(t => (
              <button key={t.id} type="button" className="fa-tag" aria-pressed={tag === String(t.id)} onClick={() => setTag(tag === String(t.id) ? null : String(t.id))}>{t.name}</button>
            ))}
          </div>
        </NavSection>
      </div>
      <div className="fa-side-foot">
        {scanning && <div className="fa-scan" role="status" aria-label="Đang quét"><span className="fa-scan-dot" aria-hidden />Đang quét{scanned > 0 ? ` · ${scanned} font` : ''}{cloudNow > 0 && <Tooltip label={CLOUD_TIP}><span className="fa-scan-cloud" data-testid="scan-not-downloaded"><Icon name="cloud" size={12} />{cloudNow} font chưa tải về</span></Tooltip>}</div>}
        <ModelProgress api={api} />
        <div className="fa-foot-actions">
          <Tooltip label={dark ? 'Chuyển sang giao diện sáng' : 'Chuyển sang giao diện tối'}><button type="button" className="fa-iconbtn" aria-label="Giao diện sáng/tối" aria-pressed={dark} onClick={toggleTheme}><Icon name="moon" /></button></Tooltip>
          <Tooltip label="Cài đặt" shortcut="⌘,"><button type="button" className="fa-iconbtn" aria-label="Cài đặt" onClick={() => (onOpenSettings ?? useUiStore.getState().toggleSettings)()}><Icon name="gear" /></button></Tooltip>
        </div>
      </div>
    </nav>
    </>
  );
}
