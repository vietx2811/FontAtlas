import { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Api, OnboardingDto, ScanEvent, SourceDto } from '../../api';
import { CmdError } from '../../api';
import { DONE, estimateAnalysisMinutes, nextStep, prevStep, startStep, visibleSteps, type Step } from '../../store/onboarding';
import { applyTheme, type ThemeMode } from '../../theme';
import { Illustration } from './Illustrations';
import { ScanBlocks } from './ScanBlocks';
import { Intro, Outro, INTRO_MS, OUTRO_MS } from './Intro';
import { usePrefersReducedMotion } from './blueprint';
import { t } from '../../i18n';
import { pickFoldersWithDialog } from '../sidebar/useDropPaths';
import './onboarding.css';
import { useModelStore, useModelUi } from '../../store/model';
import { ModelDetails, ModelSpinner } from '../model/ModelFeedback';
import { Tooltip } from '../ui/Tooltip';

type Props = { api: Api; pickFolder?: () => Promise<string | null>; pickFolders?: () => Promise<string[]>; onDone(): void;
  /** false: không có intro/outro (test, QA). Giảm chuyển động của hệ thống cũng tắt chúng. */ animated?: boolean; introMs?: number; outroMs?: number };
type Phase = 'intro' | 'steps' | 'outro';
const MB = 1024 * 1024;
const kindOf = (p: string | null) => (!p ? 'trong máy' : p.startsWith('/Volumes/') ? 'ổ ngoài' : /iCloud|Mobile Documents/.test(p) ? 'iCloud' : 'trong máy');

export function Onboarding({ api, pickFolder, pickFolders = pickFoldersWithDialog, onDone, animated = true, introMs = INTRO_MS, outroMs = OUTRO_MS }: Props): React.JSX.Element | null {
  const reduced = usePrefersReducedMotion();
  const motion = animated && !reduced;
  const [phase, setPhase] = useState<Phase>('steps');
  const [stage, setStage] = useState<HTMLDivElement | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const hold = import.meta.env.DEV && new URLSearchParams(location.hash.replace(/^#/, '')).get('hold') === '1'; // QA: giữ intro/outro để chụp
  const motionRef = useRef(motion); motionRef.current = motion;
  const [state, setState] = useState<OnboardingDto | null>(null);
  const [step, setStep] = useState<Step>('o1');
  const [working, setWorking] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const reload = useCallback(async () => { const s = await api.onboardingState(); setState(s); return s; }, [api]);

  useEffect(() => {
    let alive = true;
    api.onboardingState().then(s => {
      if (!alive) return;
      setState(s);
      const dev = import.meta.env.DEV ? new URLSearchParams(location.hash.replace(/^#/, '')).get('ob') : null; // chỉ cho QA
      setStep(dev && (['o1', 'o2', 'o3', 'o4', 'o5', 'o6', 'o7', 'end'] as string[]).includes(dev) ? (dev as Step) : startStep(s));
      if (dev === 'intro' || dev === 'outro') setPhase(dev); // chỉ cho QA/ảnh chụp (#ob=intro|outro&hold=1)
      if (!dev && motionRef.current && s.step === null && startStep(s) === 'o1') setPhase('intro'); // chỉ lần đầu và khi cho phép chuyển động
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
      if (e.defaultPrevented || e.isComposing || e.repeat || e.metaKey || e.ctrlKey || e.altKey) return;
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

  if (!state) return <div className="fa-onb"><main className="fa-onb-body"><p role={loadError ? "alert" : "status"}>{loadError ? `Không tải được hướng dẫn: ${loadError}` : "Đang mở hướng dẫn…"}</p>{loadError && <button type="button" onClick={() => location.reload()}>Thử lại</button>}</main></div>;
  if (phase === 'intro') return <div className="fa-onb" ref={root} data-phase="intro"><Intro ms={hold ? 1e9 : introMs} onDone={() => setPhase('steps')} /></div>;
  if (phase === 'outro') return <div className="fa-onb" ref={root} data-phase="outro"><Outro ms={hold ? 1e9 : outroMs} onDone={onDone} /></div>;
  const steps = visibleSteps(state);
  // Số chấm cố định suốt hướng dẫn: bước quét (o4) luôn có chấm, kể cả trước khi thêm nguồn,
  // để chấm không tăng giữa chừng. Màn "Xong" tô hết.
  const dots: Step[] = steps.filter(x => x !== 'end').concat(steps.includes('o4') ? [] : ['o4']).sort((a, b) => a.localeCompare(b));
  const idx = step === 'end' ? dots.length - 1 : Math.max(0, dots.indexOf(step));
  const next = () => go(nextStep(step, state));
  const back = prevStep(step, state);
  const ctx = { api, state, reload, next, finish, stage, pickFolders: pickFolder ? async () => { const p = await pickFolder(); return p ? [p] : []; } : pickFolders, setWorking };

  return (
    <div className="fa-onb" ref={root} data-step={step}>
      <header className="fa-onb-top">
        {back ? <button type="button" className="fa-onb-back" disabled={working} onClick={() => go(back)}>Quay lại</button> : <span />}
        <div className="fa-onb-dots" role="group" aria-label={t('onb.step', { n: Math.min(idx + 1, dots.length), total: dots.length })}>
          {dots.map((d, i) => <i key={d} data-on={i <= idx} data-now={i === idx} aria-hidden />)}
        </div>
        <div className="fa-onb-meta" aria-hidden>
          <span className="fa-onb-stepno">{t('onb.step', { n: Math.min(idx + 1, dots.length), total: dots.length })}</span>
          <span className="fa-onb-keys"><kbd>←</kbd> {t('onb.keys.back')} · <kbd>→</kbd> {t('onb.keys.next')}{step === 'o1' && <> · <kbd>Esc</kbd> {t('onb.keys.skip')}</>}</span>
        </div>
      </header>
      <main className="fa-onb-body">
        <Illustration step={step} slotRef={setStage} />
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
    </div>
  );
}

type Ctx = { api: Api; state: OnboardingDto; reload(): Promise<OnboardingDto>; next(): Promise<void>; finish(quick?: boolean): Promise<void>; stage: HTMLElement | null; pickFolders(): Promise<string[]>; setWorking(busy: boolean): void };
const Actions = ({ children }: { children: React.ReactNode }) => <div className="fa-onb-actions">{children}</div>;
/** `safe`: cho phép phím → / Enter kích hoạt (chỉ nút không có hệ quả pháp lý hay đổi cài đặt). */
const Primary = ({ safe, ...p }: React.ButtonHTMLAttributes<HTMLButtonElement> & { safe?: boolean }) => <button type="button" className="primary" data-keynav={safe ? '' : undefined} {...p} />;

function O1({ api, next, finish }: Ctx) {
  const [lang, setLang] = useState<'vi' | 'en'>('vi');
  const [theme, setTheme] = useState<ThemeMode>('system');
  return (
    <>
      <h1>Chào mừng đến với Font Atlas</h1>
      <p>Phân tích giải phẫu font, tìm font tương tự và ghép cặp có giải thích theo từng trục.</p>
      <fieldset className="fa-onb-field"><legend>Ngôn ngữ</legend>
        {([['vi', 'Tiếng Việt'], ['en', 'English']] as const).map(([v, label]) => (
          <label key={v}><input type="radio" name="lang" checked={lang === v} onChange={() => { setLang(v); void api.setSettings({ uiLang: v }); }} />{label}</label>))}
      </fieldset>
      <fieldset className="fa-onb-field"><legend>Giao diện</legend>
        {([['system', 'Theo hệ thống'], ['light', 'Sáng'], ['dark', 'Tối']] as const).map(([v, label]) => (
          <label key={v}><input type="radio" name="theme" checked={theme === v} onChange={() => { setTheme(v); applyTheme(v); }} />{label}</label>))}
      </fieldset>
      <Actions><button type="button" data-skip onClick={() => void finish(true)}>Bỏ qua hướng dẫn</button><Primary safe onClick={() => void next()}>Bắt đầu</Primary></Actions>
    </>
  );
}

function O2({ api, state, reload, next }: Ctx) {
  const [busy, setBusy] = useState(false);
  const l = state.legacy;
  const p = l.preview;
  return (
    <>
      <h1>Dữ liệu từ phiên bản trước</h1>
      <p>Tìm thấy dữ liệu từ phiên bản trước.</p>
      {p && <p className="fa-onb-count">{p.sources} nguồn · {p.favorites} yêu thích · {p.collections} collection · {p.tags} thẻ</p>}
      {l.imported && <p role="status">Đã chuyển {l.imported.fonts} font, {l.imported.collections} collection.</p>}
      {l.error && <p role="alert">Không chuyển được: {l.error.message}</p>}
      <Actions>
        <button type="button" onClick={() => void next()}>Bỏ qua</button>
        {l.imported ? <Primary onClick={() => void next()}>Tiếp tục</Primary>
          : <Primary disabled={busy} onClick={async () => { setBusy(true); try { await api.legacyImport(); } finally { setBusy(false); await reload(); } }}>Chuyển dữ liệu</Primary>}
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
      if (existing === path) return 'Thư mục này đã có trong thư viện.';
      if (existing && path && existing.startsWith(`${path.replace(/\/$/, '')}/`)) return `Thư mục này chứa nguồn đã có: ${existing}. Giữ nguyên nguồn cũ.`;
      return 'Thư mục này đã nằm trong nguồn có sẵn. Giữ nguyên nguồn cũ.';
    }
    if (error.code === 'busy') return 'Nguồn đang quét. Hãy đợi quét xong rồi thử lại.';
    if (error.code === 'io') return `Không truy cập được thư mục. Kiểm tra quyền đọc hoặc kết nối ổ đĩa. ${error.message}`;
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
    }).then(u => { if (dead) u(); else off = u; }).catch(e => { if (!dead) setMessages([`Không theo dõi được tiến độ: ${sourceError(e)}`]); });
    api.scanStatus().then(s => { if (!dead && !eventSeen) setScanning([...s.queued, ...(s.currentSourceId === null ? [] : [s.currentSourceId])]); }).catch(e => { if (!dead) setMessages([`Không đọc được trạng thái quét: ${sourceError(e)}`]); });
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
      <h1>Thư mục font</h1>
      <p>Chọn một hoặc nhiều thư mục. Font được quét tự động.</p>
      <ul className="fa-onb-sources" aria-label="Nguồn font" aria-busy={loading || busy}>
        {sources.map(s => {
          const active = s.scanning || scanning.includes(s.id);
          return <li key={s.id}>
            <div className="fa-onb-source-info"><strong>{s.name}</strong><Tooltip label={s.path ?? s.name} onlyIfTruncated><span className="path">{s.path ?? s.name}</span></Tooltip>
              <span className="kind">{kindOf(s.path)} · {!s.available ? 'Không truy cập được' : active ? 'Đang quét / chờ quét' : `${s.faceCount.toLocaleString('vi-VN')} font`}</span>
            </div>
            <Tooltip label={active ? 'Đợi quét xong để xóa nguồn' : 'Gỡ nguồn khỏi thư viện'}><button type="button" aria-disabled={busy || active || undefined} aria-label={`Xoá ${s.name}`} onClick={() => { if (!busy && !active) void run(async () => { await api.removeSource(s.id); await refresh(); }); }}>Xoá</button></Tooltip>
          </li>;
        })}
        {sources.length === 0 && <li className="empty">{loading ? 'Đang tải nguồn…' : loadError ? 'Chưa tải được danh sách nguồn.' : 'Chưa có thư mục nào.'}</li>}
      </ul>
      {loadError && <div className="fa-onb-feedback" role="alert">Không tải được nguồn: {loadError} <button type="button" disabled={busy || loading} onClick={() => void load()}>Thử lại</button></div>}
      {messages.length > 0 && <ul className="fa-onb-feedback" role="alert">{messages.map((m, i) => <li key={i}>{m}</li>)}</ul>}
      {busy && <p role="status">Đang cập nhật thư mục…</p>}
      <div className="fa-onb-suggest">
        <button type="button" disabled={busy || loading || !!loadError} onClick={() => void add()}>+ Thêm thư mục</button>
        <button type="button" disabled={busy || loading || !!loadError} onClick={() => void add(['/Library/Fonts'])}>Font hệ thống</button>
      </div>
      {sources.some(s => s.path?.startsWith('/Volumes/')) && <p className="fa-onb-note">Ổ ngoài cần được kết nối để xem và dùng font.</p>}
      <Actions><button type="button" disabled={busy} onClick={() => void next()}>Để sau</button><Primary safe disabled={busy || loading || !!loadError || sources.length === 0} onClick={() => void next()}>Tiếp tục</Primary></Actions>
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
  const text = stats.finished ? `Đã tìm ${stats.seen} file · đã phân tích ${stats.analyzed} font · ${stats.failed} file có vấn đề` : `Đã tìm ${seen} file · đã phân tích ${written} font`;
  return (
    <>
      {stage && createPortal(<ScanBlocks seed={seed} scanning={st.scanning} progress={st.progress} done={st.done} />, stage)}
      <h1>{st.done ? 'Quét xong' : 'Đang quét thư viện'}</h1>
      <p className="sr-only" data-testid="scan-summary" aria-live="polite">{text}</p>
      {!st.done && <p className="fa-onb-note">Quét vẫn chạy nền nếu bạn đi tiếp.</p>}
      <Actions><Primary safe onClick={() => void next()}>Tiếp tục</Primary></Actions>
    </>
  );
}

function O5({ api, reload, next }: Ctx) {
  return (
    <>
      <h1>Giấy phép và trách nhiệm</h1>
      <p>Font Atlas phân tích mọi font bạn thêm, kể cả font thương mại. Bạn tự chịu trách nhiệm về giấy phép. Dữ liệu không rời khỏi máy.</p>
      <Actions><Primary onClick={async () => { await api.setSettings({ licenseNoticeAck: true }); await reload(); await next(); }}>Đã hiểu</Primary></Actions>
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
      <h1>Phân tích hình ảnh (tuỳ chọn)</h1>
      <p>So sánh hình dáng chữ bằng model ảnh để Similar sát với mắt nhìn hơn. Không bật vẫn dùng được số đo.</p>
      <ul className="fa-onb-list">
        <li>~371 MB, tải một lần từ Hugging Face (SigLIP 2, Apache-2.0). Chạy trên máy.</li>
        {info && <li>{info.faces > 0 ? `${info.faces.toLocaleString('vi-VN')} font · khoảng ${mins} phút · ` : ''}còn trống {Math.round(info.free / (1024 * MB))} GB</li>}
      </ul>
      {short > 0 && <p role="alert">Cần thêm {Math.ceil(short / MB)} MB trống.</p>}
      {initError && <p role="alert">Không đọc được trạng thái model.</p>}
      {showDownload && <p role={ui.severity === 'error' ? 'alert' : 'status'}><ModelSpinner ui={ui} />{ui.text ?? (model?.state === 'ready' ? 'Model đã sẵn sàng' : 'Phân tích hình ảnh đang tắt')}</p>}
      <ModelDetails detail={ui.detail ?? initError ?? undefined} />
      <Actions>
        {!showDownload && <><button type="button" onClick={async () => { await api.setSettings({ embedEnabled: false }); await next(); }}>Không dùng</button>
        <button type="button" onClick={async () => { await api.setSettings({ embedEnabled: true }); await next(); }}>Để sau</button>
        <Primary disabled={short > 0 || !info || busy || !!initError} onClick={async () => { setStarted(true); setBusy(true); try { await store.getState().enable(api); } finally { setBusy(false); } }}>Tải ngay</Primary></>}
        {showDownload && <>{ui.actions.map(a => <button key={a.label} type="button" disabled={busy} onClick={() => void store.getState().act(api, a.id)}>{a.label}</button>)}<Primary onClick={() => void next()}>Tiếp tục</Primary></>}
      </Actions>
    </>
  );
}

function O7({ api, state, next }: Ctx) {
  const [keep, setKeep] = useState(state.keepActiveOnExit);
  const managers = state.fontManagers.filter(m => m.installed);
  if (!state.activationSupported) {
    return (<><h1>Kích hoạt font cho app thiết kế</h1><p>Sắp có trên Windows.</p><Actions><Primary onClick={() => void next()}>Tiếp tục</Primary></Actions></>);
  }
  return (
    <>
      <h1>Kích hoạt font cho app thiết kế</h1>
      <p>Dùng font trong Photoshop, Figma… mà không cần cài vào hệ thống.</p>
      <fieldset className="fa-onb-field"><legend>Khi thoát Font Atlas</legend>
        <label><input type="radio" name="keep" checked={!keep} onChange={() => setKeep(false)} />Tắt các font đã kích hoạt <small>Mở lại app sẽ tự bật lại.</small></label>
        <label><input type="radio" name="keep" checked={keep} onChange={() => setKeep(true)} />Giữ font hoạt động <small>Cho tới khi bạn tắt trong Font Atlas.</small></label>
      </fieldset>
      {managers.length > 0 && (
        <p className="fa-onb-warn">⚠ Phát hiện {managers.map(m => m.name).join(', ')}. Đừng kích hoạt cùng một font ở hai app.</p>)}
      <Actions><button type="button" onClick={() => void next()}>Để sau</button><Primary onClick={async () => { await api.activationSetKeep(keep); await next(); }}>Bật kích hoạt font</Primary></Actions>
    </>
  );
}

function End({ finish }: Ctx) {
  return (
    <>
      <h1>Xong</h1>
      <p>Thư viện của bạn đang được phân tích. Vài gợi ý để bắt đầu:</p>
      <ul className="fa-onb-list"><li>Mở một font để xem giải phẫu.</li><li>Thử ghép cặp hai font.</li><li>Đổi chữ mẫu ở thanh dưới cùng.</li></ul>
      <Actions><Primary safe onClick={() => void finish()}>Vào thư viện</Primary></Actions>
    </>
  );
}
