import { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Api, OnboardingDto, OrganizedStatusDto, ScanEvent, SourceDto } from '../../api';
import { OrganizedSetup } from '../library/OrganizedSetup';
import { OrganizedImportDialog } from '../library/OrganizedImportDialog';
import '../library/organized.css';
import { CmdError } from '../../api';
import { DONE, estimateAnalysisMinutes, nextStep, prevStep, startStep, visibleSteps, type Step } from '../../store/onboarding';
import { applyTheme, type ThemeMode } from '../../theme';
import { Illustration } from './Illustrations';
import { ScanBlocks } from './ScanBlocks';
import { Outro, OUTRO_MS } from './Outro';
import { usePrefersReducedMotion } from './blueprint';
import { chooseLocale, formatNumber, t, useLocale } from '../../i18n';
import { isSplashOpen } from '../splash/splashState';
import { pickFoldersWithDialog } from '../sidebar/useDropPaths';
import './onboarding.css';
import { useModelStore, useModelUi } from '../../store/model';
import { ModelDetails, ModelSpinner } from '../model/ModelFeedback';
import { Tooltip } from '../ui/Tooltip';
import { ShortcutsOverlay } from '../help/ShortcutsOverlay';

type Props = { api: Api; pickFolder?: () => Promise<string | null>; pickFolders?: () => Promise<string[]>; onDone(): void;
  /** false: không có outro (test, QA). Giảm chuyển động của hệ thống cũng tắt nó. Phần mở đầu là Splash của App, không nằm ở đây. */ animated?: boolean; outroMs?: number };
type Phase = 'steps' | 'outro';
const MB = 1024 * 1024;
const kindOf = (p: string | null) => t(!p ? 'onb.kind.local' : p.startsWith('/Volumes/') ? 'onb.kind.external' : /iCloud|Mobile Documents/.test(p) ? 'onb.kind.icloud' : 'onb.kind.local');

export function Onboarding({ api, pickFolder, pickFolders = pickFoldersWithDialog, onDone, animated = true, outroMs = OUTRO_MS }: Props): React.JSX.Element | null {
  const reduced = usePrefersReducedMotion();
  const motion = animated && !reduced;
  const [phase, setPhase] = useState<Phase>('steps');
  const [stage, setStage] = useState<HTMLDivElement | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const hold = import.meta.env.DEV && new URLSearchParams(location.hash.replace(/^#/, '')).get('hold') === '1'; // QA: giữ outro để chụp
  const motionRef = useRef(motion); motionRef.current = motion;
  const [state, setState] = useState<OnboardingDto | null>(null);
  const [step, setStep] = useState<Step>('o1');
  const [working, setWorking] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Kính thật của macOS (Liquid Glass) sau mảng trái trong suốt, chỉ trong lúc hướng dẫn; rời đi thì trả lại vibrancy sidebar.
  const [glass, setGlass] = useState(false);
  useEffect(() => {
    let alive = true;
    api.setWindowGlass(true).then(on => { if (alive) setGlass(on); }, () => undefined);
    return () => { alive = false; void api.setWindowGlass(false).catch(() => undefined); };
  }, [api]);
  const native = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const reload = useCallback(async () => { const s = await api.onboardingState(); setState(s); return s; }, [api]);

  useEffect(() => {
    let alive = true;
    api.onboardingState().then(s => {
      if (!alive) return;
      setState(s);
      const dev = import.meta.env.DEV ? new URLSearchParams(location.hash.replace(/^#/, '')).get('ob') : null; // chỉ cho QA
      setStep(dev && (['o1', 'o2', 'o3', 'o4', 'o5', 'o6', 'o7', 'end'] as string[]).includes(dev) ? (dev as Step) : startStep(s));
      if (dev === 'outro') setPhase('outro'); // chỉ cho QA/ảnh chụp (#ob=outro&hold=1)
    }).catch(e => { if (alive) setLoadError(String(e)); });
    return () => { alive = false; };
  }, [api]);

  const go = useCallback(async (to: Step | null) => {
    if (!to) return;
    setStep(to);
    await api.setOnboardingStep(to);
  }, [api]);
  /** `quick`: bỏ qua màn kết (nút "Bỏ qua hướng dẫn"). */
  const finish = useCallback(async (quick?: boolean) => { await api.setOnboardingStep(DONE); if (quick || !motionRef.current) onDone(); else setPhase('outro'); }, [api, onDone]);
  const stepRef = useRef(step); stepRef.current = step;
  // Phím: ← quay lại; → hoặc Enter (khi không focus gì) đi tiếp CHỈ ở nút an toàn (data-keynav: không xác nhận giấy phép hay đổi cài đặt); Esc ở O1 = Bỏ qua hướng dẫn.
  useEffect(() => {
    if (phase !== 'steps') return;
    const q = (sel: string) => root.current?.querySelector<HTMLButtonElement>(sel);
    const on = (e: KeyboardEvent) => {
      if (isSplashOpen() || e.defaultPrevented || e.isComposing || e.repeat || e.metaKey || e.ctrlKey || e.altKey) return; // splash còn hiện: phím thuộc về nó
      const el = e.target as HTMLElement | null;
      const typing = !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
      if (typing || document.querySelector('[role="dialog"], [role="menu"]')) return;
      if (e.key === 'ArrowLeft') q('.fa-onb-back:not(:disabled)')?.click();
      else if (e.key === 'ArrowRight' || (e.key === 'Enter' && (el === document.body || !el))) q('button[data-keynav]:not(:disabled)')?.click();
      else if (e.key === 'Escape' && stepRef.current === 'o1') q('button[data-skip]')?.click();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, [phase]);

  if (!state) return <div className="fa-onb" data-loading="true" data-native={native} data-glass={glass}><section className="fa-onb-left"><main className="fa-onb-body"><p role={loadError ? "alert" : "status"}>{loadError ? t('onb.loadFail', { msg: loadError }) : t('onb.loading')}</p>{loadError && <button type="button" onClick={() => location.reload()}>{t('onb.retry')}</button>}</main></section></div>;
  if (phase === 'outro') return <div className="fa-onb" ref={root} data-phase="outro" data-native={native} data-glass={glass}><Outro ms={hold ? 1e9 : outroMs} onDone={onDone} /></div>;
  const steps = visibleSteps(state);
  // Số chấm cố định suốt hướng dẫn: bước quét (o4) luôn có chấm, kể cả trước khi thêm nguồn,
  // để chấm không tăng giữa chừng. Màn "Xong" tô hết.
  const dots: Step[] = steps.filter(x => x !== 'end').concat(steps.includes('o4') ? [] : ['o4']).sort((a, b) => a.localeCompare(b));
  const idx = step === 'end' ? dots.length - 1 : Math.max(0, dots.indexOf(step));
  const next = () => go(nextStep(step, state));
  const back = prevStep(step, state);
  const ctx = { api, state, reload, next, finish, stage, pickFolders: pickFolder ? async () => { const p = await pickFolder(); return p ? [p] : []; } : pickFolders, setWorking };

  return (
    <div className="fa-onb" ref={root} data-step={step} data-native={native} data-glass={glass}>
      <div className="fa-onb-drag" data-tauri-drag-region aria-hidden />
      <section className="fa-onb-left">
        <header className="fa-onb-top">
          {back ? <button type="button" className="fa-onb-back" disabled={working} onClick={() => go(back)}>{t('onb.back')}</button> : <span />}
          <div className="fa-onb-dots" role="group" aria-label={t('onb.step', { n: Math.min(idx + 1, dots.length), total: dots.length })}>
            {dots.map((d, i) => <i key={d} data-on={i <= idx} data-now={i === idx} aria-hidden />)}
          </div>
          <span />
        </header>
        <span id="onb-keys-desc" className="sr-only">{t('onb.keys.aria')}</span>
        <main className="fa-onb-body" aria-describedby="onb-keys-desc">
          <div className="fa-onb-pane">
            {step === 'o1' && <O1 {...ctx} />}
            {step === 'o2' && <O2 {...ctx} />}
            {step === 'o3' && <O3 {...ctx} />}
            {step === 'o4' && <O4 {...ctx} />}
            {step === 'o5' && <O5 {...ctx} />}
            {step === 'o6' && <O6 {...ctx} />}
            {step === 'o7' && <O7 {...ctx} />}
            {step === 'end' && <End {...ctx} />}
          </div>
        </main>
      </section>
      {/* Mảng phải: minh hoạ phủ kín, đổi theo bước (key = bước → vào cảnh mới có chuyển cảnh nhẹ). */}
      <div className="fa-onb-art">
        <div className="fa-onb-art-bg" aria-hidden />
        <Illustration key={step} step={step} slotRef={setStage} />
      </div>
    </div>
  );
}

type Ctx = { api: Api; state: OnboardingDto; reload(): Promise<OnboardingDto>; next(): Promise<void>; finish(quick?: boolean): Promise<void>; stage: HTMLElement | null; pickFolders(): Promise<string[]>; setWorking(busy: boolean): void };
const Actions = ({ children }: { children: React.ReactNode }) => <div className="fa-onb-actions">{children}</div>;
/** `safe`: cho phép phím → / Enter kích hoạt (chỉ nút không có hệ quả pháp lý hay đổi cài đặt). */
const Primary = ({ safe, ...p }: React.ButtonHTMLAttributes<HTMLButtonElement> & { safe?: boolean }) => <button type="button" className="primary" data-keynav={safe ? '' : undefined} {...p} />;

function O1({ api, next, finish }: Ctx) {
  const lang = useLocale();
  const [theme, setTheme] = useState<ThemeMode>('system');
  return (
    <>
      <h1>{t('onb.o1.title')}</h1>
      <p>{t('onb.o1.intro')}</p>
      <fieldset className="fa-onb-field"><legend>{t('onb.o1.lang')}</legend>
        {([['vi', 'Tiếng Việt'], ['en', 'English']] as const).map(([v, label]) => (
          <label key={v}><input type="radio" name="lang" checked={lang === v} onChange={() => { chooseLocale(v); void api.setSettings({ uiLang: v }); }} />{label}</label>))}
      </fieldset>
      <fieldset className="fa-onb-field"><legend>{t('onb.o1.theme')}</legend>
        {([['system', t('onb.theme.system')], ['light', t('onb.theme.light')], ['dark', t('onb.theme.dark')]] as const).map(([v, label]) => (
          <label key={v}><input type="radio" name="theme" checked={theme === v} onChange={() => { setTheme(v); applyTheme(v); }} />{label}</label>))}
      </fieldset>
      <Actions><button type="button" data-skip onClick={() => void finish(true)}>{t('onb.skip')}</button><Primary safe onClick={() => void next()}>{t('onb.start')}</Primary></Actions>
    </>
  );
}

function O2({ api, state, reload, next }: Ctx) {
  const [busy, setBusy] = useState(false);
  const l = state.legacy;
  const p = l.preview;
  return (
    <>
      <h1>{t('onb.o2.title')}</h1>
      <p>{t('onb.o2.intro')}</p>
      {p && <p className="fa-onb-count">{t('onb.o2.counts', { sources: p.sources, favorites: p.favorites, collections: p.collections, tags: p.tags })}</p>}
      {l.imported && <p role="status">{t('onb.o2.imported', { fonts: l.imported.fonts, collections: l.imported.collections })}</p>}
      {l.error && <p role="alert">{t('onb.o2.failed', { msg: l.error.message })}</p>}
      <Actions>
        <button type="button" onClick={() => void next()}>{t('onb.skipStep')}</button>
        {l.imported ? <Primary onClick={() => void next()}>{t('onb.continue')}</Primary>
          : <Primary disabled={busy} onClick={async () => { setBusy(true); try { await api.legacyImport(); } finally { setBusy(false); await reload(); } }}>{t('onb.o2.import')}</Primary>}
      </Actions>
    </>
  );
}

function sourceError(error: unknown, path?: string): string {
  if (error instanceof CmdError) {
    if (error.code === 'cancelled') return '';
    if (error.code === 'nested_source') {
      const d = error.detail;
      const existing = typeof d === 'object' && d !== null && 'existingPath' in d && typeof d.existingPath === 'string' ? d.existingPath : null;
      if (existing === path) return t('onb.src.dup');
      if (existing && path && existing.startsWith(`${path.replace(/\/$/, '')}/`)) return t('onb.src.contains', { path: existing });
      return t('onb.src.nested');
    }
    if (error.code === 'busy') return t('onb.src.busy');
    if (error.code === 'io') return t('onb.src.io', { msg: error.message });
  }
  return error instanceof Error ? error.message : String(error);
}

function O3({ api, reload, next, pickFolders, setWorking }: Ctx) {
  const [sources, setSources] = useState<SourceDto[]>([]);
  const [messages, setMessages] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const alive = useRef(true);
  const [scanning, setScanning] = useState<number[]>([]);
  // B7: kiểu quản lý font. Mặc định "tự quản theo thư mục" (cách cũ, không đổi gì); "Organized Library" chép vào thư viện riêng.
  const [mode, setMode] = useState<'folders' | 'organized'>('folders');
  const [org, setOrg] = useState<OrganizedStatusDto | null>(null);
  const [importing, setImporting] = useState(false);
  useEffect(() => { let on = true; api.organizedStatus().then(x => { if (on) { setOrg(x); if (x.configured) setMode('organized'); } }, () => undefined); return () => { on = false; }; }, [api]);
  const refresh = useCallback(async () => {
    const list = await api.listSources();
    if (alive.current) { setSources(list); setLoadError(null); }
    await reload();
  }, [api, reload]);
  const load = useCallback(async () => {
    setLoading(true);
    try { await refresh(); }
    catch (e) { if (alive.current) setLoadError(sourceError(e)); }
    finally { if (alive.current) setLoading(false); }
  }, [refresh]);
  useEffect(() => {
    alive.current = true;
    void load();
    let off: (() => void) | undefined;
    let dead = false;
    let eventSeen = false;
    api.scanSubscribe(e => {
      if (dead) return;
      eventSeen = true;
      if (e.type === 'queued' || e.type === 'progress') setScanning(ids => ids.includes(e.sourceId) ? ids : [...ids, e.sourceId]);
      if (e.type === 'finished' || e.type === 'failed') setScanning(ids => ids.filter(id => id !== e.sourceId));
      if (e.type === 'idle') setScanning([]);
      if (e.type === 'finished' || e.type === 'idle') void refresh().catch(err => { if (!dead) setLoadError(sourceError(err)); });
    }).then(u => { if (dead) u(); else off = u; }).catch(e => { if (!dead) setMessages([t('onb.o3.trackFail', { msg: sourceError(e) })]); });
    api.scanStatus().then(s => { if (!dead && !eventSeen) setScanning([...s.queued, ...(s.currentSourceId === null ? [] : [s.currentSourceId])]); }).catch(e => { if (!dead) setMessages([t('onb.o3.statusFail', { msg: sourceError(e) })]); });
    return () => { dead = true; alive.current = false; off?.(); };
  }, [api, load, refresh]);
  const run = async (job: () => Promise<void>) => {
    if (lock.current) return;
    lock.current = true; setBusy(true); setWorking(true); setMessages([]);
    try { await job(); }
    catch (e) { if (alive.current) setMessages([sourceError(e)].filter(Boolean)); }
    finally { lock.current = false; if (alive.current) { setBusy(false); setWorking(false); } }
  };
  const add = (paths?: string[]) => run(async () => {
    const chosen = paths ?? await pickFolders();
    const errors: string[] = [];
    for (const path of chosen) {
      try { await api.addSource(path); }
      catch (e) { const msg = sourceError(e, path); if (msg) errors.push(`${path}: ${msg}`); }
    }
    if (alive.current) setMessages(errors);
    if (chosen.length) {
      try { await refresh(); }
      catch (e) { if (alive.current) setLoadError(sourceError(e)); }
    }
  });
  return (
    <>
      <h1>{t('onb.o3.title')}</h1>
      <fieldset className="fa-org-mode" data-testid="onb-mode">
        <legend>{t('onb.mode.title')}</legend>
        <label><input type="radio" name="fa-mode" checked={mode === 'organized'} disabled={busy} onChange={() => setMode('organized')} />
          <span><b>{t('onb.mode.organized')}</b><small>{t('onb.mode.organized.d')}</small></span></label>
        <label><input type="radio" name="fa-mode" checked={mode === 'folders'} disabled={busy || !!org?.configured} onChange={() => setMode('folders')} />
          <span><b>{t('onb.mode.folders')}</b><small>{t('onb.mode.folders.d')}</small></span></label>
      </fieldset>
      {mode === 'organized' && (org?.configured
        ? <div className="fa-onb-note" data-testid="onb-org-ready"><p>{t('onb.mode.ready', { root: org.root ?? '' })}</p>
            {sources.some(x => x.faceCount > 0) && <button type="button" disabled={busy || scanning.length > 0} data-testid="onb-org-import" onClick={() => setImporting(true)}>{t('onb.mode.import')}</button>}
            {scanning.length > 0 && <p>{t('onb.mode.waitScan')}</p>}</div>
        : <OrganizedSetup api={api} pick={async () => (await pickFolders())[0] ?? null} disabled={busy} onCreated={x => setOrg(x)} />)}
      <p className="fa-onb-note">{t('onb.mode.later')}</p>
      {importing && <OrganizedImportDialog api={api} onClose={() => { setImporting(false); void api.organizedStatus().then(setOrg, () => undefined); }} />}
      <p>{t('onb.o3.intro')}</p>
      <ul className="fa-onb-sources" aria-label={t('onb.o3.list')} aria-busy={loading || busy}>
        {sources.map(s => {
          const active = s.scanning || scanning.includes(s.id);
          return <li key={s.id}>
            <div className="fa-onb-source-info"><strong>{s.name}</strong><Tooltip label={s.path ?? s.name} onlyIfTruncated><span className="path">{s.path ?? s.name}</span></Tooltip>
              <span className="kind">{kindOf(s.path)} · {!s.available ? t('onb.o3.unavailable') : active ? t('onb.o3.scanning') : t('onb.o3.faces', { n: formatNumber(s.faceCount) })}</span>
            </div>
            <Tooltip label={active ? t('onb.o3.removeWait') : t('onb.o3.removeTip')}><button type="button" aria-disabled={busy || active || undefined} aria-label={t('onb.o3.removeAria', { name: s.name })} onClick={() => { if (!busy && !active) void run(async () => { await api.removeSource(s.id); await refresh(); }); }}>{t('onb.o3.remove')}</button></Tooltip>
          </li>;
        })}
        {sources.length === 0 && <li className="empty">{loading ? t('onb.o3.loadingList') : loadError ? t('onb.o3.listFail') : t('onb.o3.empty')}</li>}
      </ul>
      {loadError && <div className="fa-onb-feedback" role="alert">{t('onb.o3.loadFail', { msg: loadError })} <button type="button" disabled={busy || loading} onClick={() => void load()}>{t('onb.retry')}</button></div>}
      {messages.length > 0 && <ul className="fa-onb-feedback" role="alert">{messages.map((m, i) => <li key={i}>{m}</li>)}</ul>}
      {busy && <p role="status">{t('onb.o3.updating')}</p>}
      <div className="fa-onb-suggest">
        <button type="button" disabled={busy || loading || !!loadError} onClick={() => void add()}>{t('onb.o3.add')}</button>
        <button type="button" disabled={busy || loading || !!loadError} onClick={() => void add(['/Library/Fonts'])}>{t('onb.o3.system')}</button>
      </div>
      {sources.some(s => s.path?.startsWith('/Volumes/')) && <p className="fa-onb-note">{t('onb.o3.external')}</p>}
      <Actions><button type="button" disabled={busy} onClick={() => void next()}>{t('onb.later')}</button><Primary safe disabled={busy || loading || !!loadError || sources.length === 0} onClick={() => void next()}>{t('onb.continue')}</Primary></Actions>
    </>
  );
}

function O4({ api, next, stage }: Ctx) {
  const [seed] = useState(() => Math.floor(Math.random() * 2 ** 31));
  const [st, setSt] = useState<{ scanning: boolean; progress: number | null; done: boolean }>({ scanning: false, progress: null, done: false });
  const [seen, setSeen] = useState(0); const [written, setWritten] = useState(0);
  const [stats, setStats] = useState({ seen: 0, analyzed: 0, failed: 0, finished: false });
  useEffect(() => {
    let off: (() => void) | undefined; let dead = false;
    api.scanSubscribe((e: ScanEvent) => {
      if (e.type === 'queued') setSt(s => ({ ...s, scanning: true, done: false }));
      else if (e.type === 'progress') { setSeen(e.progress.total); setWritten(e.progress.facesWritten); setSt({ scanning: true, done: false, progress: e.progress.total > 0 ? e.progress.done / e.progress.total : null }); }
      else if (e.type === 'finished') setStats(d => ({ seen: d.seen + e.stats.seen, analyzed: d.analyzed + e.stats.analyzed, failed: d.failed + e.stats.failed, finished: true }));
      else if (e.type === 'idle') setSt({ scanning: false, progress: 1, done: true });
    }).then(u => { if (dead) u(); else off = u; });
    // Quét đã xong từ trước khi vào bước này (hoặc chưa chạy): hiện dấu tick luôn.
    void api.scanStatus().then(s => { if (!dead) setSt(p => (s.running ? { ...p, scanning: true } : p.scanning ? p : { scanning: false, progress: 1, done: true })); });
    return () => { dead = true; off?.(); };
  }, [api]);
  const text = stats.finished ? t('onb.o4.summaryDone', { seen: stats.seen, analyzed: stats.analyzed, failed: stats.failed }) : t('onb.o4.summary', { seen, written });
  return (
    <>
      {stage && createPortal(<ScanBlocks seed={seed} scanning={st.scanning} progress={st.progress} done={st.done} />, stage)}
      <h1>{st.done ? t('onb.o4.done') : t('onb.o4.running')}</h1>
      <p className="sr-only" data-testid="scan-summary" aria-live="polite">{text}</p>
      {!st.done && <p className="fa-onb-note">{t('onb.o4.note')}</p>}
      <Actions><Primary safe onClick={() => void next()}>{t('onb.continue')}</Primary></Actions>
    </>
  );
}

function O5({ api, reload, next }: Ctx) {
  return (
    <>
      <h1>{t('onb.o5.title')}</h1>
      <p>{t('onb.o5.body')}</p>
      <Actions><Primary onClick={async () => { await api.setSettings({ licenseNoticeAck: true }); await reload(); await next(); }}>{t('onb.o5.ack')}</Primary></Actions>
    </>
  );
}

function O6({ api, next }: Ctx) {
  const store = useModelStore;
  const ui = useModelUi(store);
  const model = store(s => s.model);
  const [started, setStarted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [initError, setInitError] = useState<string | null>(null);
  const [info, setInfo] = useState<{ free: number; required: number; faces: number } | null>(null);
  useEffect(() => {
    let alive = true; let off: (() => void) | undefined;
    store.getState().sync(api).then(u => { if (alive) off = u; else u(); }).catch(e => { if (alive) setInitError(String(e)); });
    void Promise.all([api.modelStatus(), api.listSources()]).then(([m, s]) => { if (alive) setInfo({ free: m.diskFree, required: m.requiredBytes, faces: s.reduce((n, x) => n + x.faceCount, 0) }); }).catch(e => { if (alive) setInitError(String(e)); });
    return () => { alive = false; off?.(); };
  }, [api]);
  const showDownload = started || !!model && model.state !== 'missing' || ui.severity === 'error';
  const short = info ? Math.max(0, info.required - info.free) : 0;
  const mins = info ? estimateAnalysisMinutes(info.faces) : 0;
  return (
    <>
      <h1>{t('onb.o6.title')}</h1>
      <p>{t('onb.o6.intro')}</p>
      <ul className="fa-onb-list">
        <li>{t('onb.o6.size')}</li>
        {info && <li>{info.faces > 0 ? t('onb.o6.faces', { n: formatNumber(info.faces), mins }) : ''}{t('onb.o6.free', { gb: Math.round(info.free / (1024 * MB)) })}</li>}
      </ul>
      {short > 0 && <p role="alert">{t('onb.o6.short', { mb: Math.ceil(short / MB) })}</p>}
      {initError && <p role="alert">{t('onb.o6.initFail')}</p>}
      {showDownload && <p role={ui.severity === 'error' ? 'alert' : 'status'}><ModelSpinner ui={ui} />{ui.text ?? (model?.state === 'ready' ? t('onb.o6.ready') : t('onb.o6.off'))}</p>}
      <ModelDetails detail={ui.detail ?? initError ?? undefined} />
      <Actions>
        {!showDownload && <><button type="button" onClick={async () => { await api.setSettings({ embedEnabled: false }); await next(); }}>{t('onb.o6.no')}</button>
        <button type="button" onClick={async () => { await api.setSettings({ embedEnabled: true }); await next(); }}>{t('onb.later')}</button>
        <Primary disabled={short > 0 || !info || busy || !!initError} onClick={async () => { setStarted(true); setBusy(true); try { await store.getState().enable(api); } finally { setBusy(false); } }}>{t('onb.o6.download')}</Primary></>}
        {showDownload && <>{ui.actions.map(a => <button key={a.label} type="button" disabled={busy} onClick={() => void store.getState().act(api, a.id)}>{a.label}</button>)}<Primary onClick={() => void next()}>{t('onb.continue')}</Primary></>}
      </Actions>
    </>
  );
}

function O7({ api, state, next }: Ctx) {
  const [keep, setKeep] = useState(state.keepActiveOnExit);
  const managers = state.fontManagers.filter(m => m.installed);
  if (!state.activationSupported) {
    return (<><h1>{t('onb.o7.title')}</h1><p>{t('onb.o7.soon')}</p><Actions><Primary onClick={() => void next()}>{t('onb.continue')}</Primary></Actions></>);
  }
  return (
    <>
      <h1>{t('onb.o7.title')}</h1>
      <p>{t('onb.o7.intro')}</p>
      <fieldset className="fa-onb-field"><legend>{t('onb.o7.exit')}</legend>
        <label><input type="radio" name="keep" checked={!keep} onChange={() => setKeep(false)} />{t('onb.o7.exitOff')} <small>{t('onb.o7.exitOffNote')}</small></label>
        <label><input type="radio" name="keep" checked={keep} onChange={() => setKeep(true)} />{t('onb.o7.exitKeep')} <small>{t('onb.o7.exitKeepNote')}</small></label>
      </fieldset>
      {managers.length > 0 && (
        <p className="fa-onb-warn">{t('onb.o7.warn', { names: managers.map(m => m.name).join(', ') })}</p>)}
      <Actions><button type="button" onClick={() => void next()}>{t('onb.later')}</button><Primary onClick={async () => { await api.activationSetKeep(keep); await next(); }}>{t('onb.o7.enable')}</Primary></Actions>
    </>
  );
}

function End({ finish }: Ctx) {
  // Trước màn kết: bảng phím tắt & tính năng chính. Esc, bấm nền hay nút đều vào thư viện (bỏ qua được).
  const [tips, setTips] = useState(() => import.meta.env.DEV && new URLSearchParams(location.hash.replace(/^#/, '')).get('tips') === '1'); // tips=1: chỉ cho ảnh chụp/QA dev
  return (
    <>
      {tips && <ShortcutsOverlay primary={t('tips.start')} onClose={() => void finish()} />}
      <h1>{t('onb.end.title')}</h1>
      <p>{t('onb.end.intro')}</p>
      <ul className="fa-onb-list"><li>{t('onb.end.tip1')}</li><li>{t('onb.end.tip2')}</li><li>{t('onb.end.tip3')}</li></ul>
      <Actions><Primary safe onClick={() => setTips(true)}>{t('onb.end.enter')}</Primary></Actions>
    </>
  );
}
