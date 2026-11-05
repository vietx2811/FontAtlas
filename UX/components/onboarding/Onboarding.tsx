import { useCallback, useEffect, useState } from 'react';
import type { Api, OnboardingDto, ScanEvent, SourceDto } from '../../api';
import { CmdError } from '../../api';
import { DONE, estimateAnalysisMinutes, nextStep, prevStep, startStep, visibleSteps, type Step } from '../../store/onboarding';
import { applyTheme, type ThemeMode } from '../../theme';
import { ScanBlocks } from './ScanBlocks';
import { pickFolderWithDialog } from '../sidebar/useDropPaths';
import './onboarding.css';

type Props = { api: Api; pickFolder?: () => Promise<string | null>; onDone(): void };
const MB = 1024 * 1024;
const kindOf = (p: string | null) => (!p ? 'trong máy' : p.startsWith('/Volumes/') ? 'ổ ngoài' : /iCloud|Mobile Documents/.test(p) ? 'iCloud' : 'trong máy');

export function Onboarding({ api, pickFolder = pickFolderWithDialog, onDone }: Props): React.JSX.Element | null {
  const [state, setState] = useState<OnboardingDto | null>(null);
  const [step, setStep] = useState<Step>('o1');
  const reload = useCallback(async () => { const s = await api.onboardingState(); setState(s); return s; }, [api]);

  useEffect(() => {
    let alive = true;
    api.onboardingState().then(s => {
      if (!alive) return;
      setState(s);
      const dev = import.meta.env.DEV ? new URLSearchParams(location.hash.replace(/^#/, '')).get('ob') : null; // chỉ cho QA
      setStep(dev && (['o1', 'o2', 'o3', 'o4', 'o5', 'o6', 'o7', 'end'] as string[]).includes(dev) ? (dev as Step) : startStep(s));
    });
    return () => { alive = false; };
  }, [api]);

  const go = useCallback(async (to: Step | null) => {
    if (!to) return;
    setStep(to);
    await api.setOnboardingStep(to);
  }, [api]);
  const finish = useCallback(async () => { await api.setOnboardingStep(DONE); onDone(); }, [api, onDone]);

  if (!state) return null;
  const steps = visibleSteps(state);
  const dots: Step[] = steps.filter(x => x !== 'end');
  const idx = Math.max(0, dots.indexOf(step));
  const next = () => go(nextStep(step, state));
  const back = prevStep(step, state);
  const ctx = { api, state, reload, next, finish, pickFolder };

  return (
    <div className="fa-onb" data-step={step}>
      <header className="fa-onb-top">
        {back ? <button type="button" className="fa-onb-back" onClick={() => go(back)}>Quay lại</button> : <span />}
        <div className="fa-onb-dots" role="group" aria-label={`Bước ${Math.min(idx + 1, dots.length)}/${dots.length}`}>
          {dots.map((d, i) => <i key={d} data-on={i <= idx} aria-hidden />)}
        </div>
        <span />
      </header>
      <main className="fa-onb-body">
        {step === 'o1' && <O1 {...ctx} />}
        {step === 'o2' && <O2 {...ctx} />}
        {step === 'o3' && <O3 {...ctx} />}
        {step === 'o4' && <O4 {...ctx} />}
        {step === 'o5' && <O5 {...ctx} />}
        {step === 'o6' && <O6 {...ctx} />}
        {step === 'o7' && <O7 {...ctx} />}
        {step === 'end' && <End {...ctx} />}
      </main>
    </div>
  );
}

type Ctx = { api: Api; state: OnboardingDto; reload(): Promise<OnboardingDto>; next(): Promise<void>; finish(): Promise<void>; pickFolder(): Promise<string | null> };
const Actions = ({ children }: { children: React.ReactNode }) => <div className="fa-onb-actions">{children}</div>;
const Primary = (p: React.ButtonHTMLAttributes<HTMLButtonElement>) => <button type="button" className="primary" {...p} />;

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
      <Actions><button type="button" onClick={() => void finish()}>Bỏ qua hướng dẫn</button><Primary onClick={() => void next()}>Bắt đầu</Primary></Actions>
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

function O3({ api, state, reload, next, pickFolder }: Ctx) {
  const [sources, setSources] = useState<SourceDto[]>([]);
  const [msg, setMsg] = useState<string | null>(null);
  const refresh = useCallback(async () => { setSources(await api.listSources()); await reload(); }, [api, reload]);
  useEffect(() => { void api.listSources().then(setSources); }, [api]);
  const add = async (path?: string | null) => {
    const p = path ?? await pickFolder();
    if (!p) return;
    try { await api.addSource(p); setMsg(null); await refresh(); }
    catch (e) {
      setMsg(e instanceof CmdError && e.code === 'nested_source' ? 'Thư mục này đã nằm trong nguồn có sẵn, nên không cần thêm lần nữa.' : `Không thêm được: ${(e as Error).message}`);
    }
  };
  return (
    <>
      <h1>Thư mục font</h1>
      <p>Thêm thư mục chứa font. Quét bắt đầu ngay khi thêm.</p>
      <ul className="fa-onb-sources">
        {sources.map(s => (
          <li key={s.id}><span className="path">{s.path}</span><span className="kind">{kindOf(s.path)}</span>
            <button type="button" aria-label={`Xoá ${s.name}`} onClick={async () => { await api.removeSource(s.id); await refresh(); }}>Xoá</button></li>))}
        {sources.length === 0 && <li className="empty">Chưa có thư mục nào.</li>}
      </ul>
      {msg && <p role="alert">{msg}</p>}
      <div className="fa-onb-suggest">
        <button type="button" onClick={() => void add()}>+ Thêm thư mục</button>
        <button type="button" onClick={() => void add('/Library/Fonts')}>Font hệ thống</button>
      </div>
      {sources.some(s => s.path?.startsWith('/Volumes/')) && <p className="fa-onb-note">Font trên ổ ngoài sẽ không khả dụng khi rút ổ; dữ liệu của bạn không mất.</p>}
      <Actions><button type="button" onClick={() => void next()}>Để sau</button><Primary disabled={!state.hasSources && sources.length === 0} onClick={() => void next()}>Tiếp tục</Primary></Actions>
    </>
  );
}

function O4({ api, next }: Ctx) {
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
      <h1>Đang quét thư viện</h1>
      <ScanBlocks seed={seed} scanning={st.scanning} progress={st.progress} done={st.done} />
      <p className="sr-only" data-testid="scan-summary" aria-live="polite">{text}</p>
      <p className="fa-onb-note">Quét vẫn chạy nền nếu bạn đi tiếp.</p>
      <Actions><Primary onClick={() => void next()}>Tiếp tục</Primary></Actions>
    </>
  );
}

function O5({ api, reload, next }: Ctx) {
  return (
    <>
      <h1>Giấy phép và trách nhiệm</h1>
      <p>Font Atlas phân tích mọi font bạn đưa vào thư viện, kể cả font thương mại. Bạn tự chịu trách nhiệm tuân thủ giấy phép của từng font. Dữ liệu phân tích và hình ảnh không bao giờ rời khỏi máy.</p>
      <Actions><Primary onClick={async () => { await api.setSettings({ licenseNoticeAck: true }); await reload(); await next(); }}>Đã hiểu</Primary></Actions>
    </>
  );
}

function O6({ api, next }: Ctx) {
  const [info, setInfo] = useState<{ free: number; required: number; faces: number } | null>(null);
  useEffect(() => {
    void Promise.all([api.modelStatus(), api.listSources()]).then(([m, s]) => setInfo({ free: m.diskFree, required: m.requiredBytes, faces: s.reduce((n, x) => n + x.faceCount, 0) }));
  }, [api]);
  const short = info ? Math.max(0, info.required - info.free) : 0;
  const mins = info ? estimateAnalysisMinutes(info.faces) : 0;
  return (
    <>
      <h1>Phân tích hình ảnh (tuỳ chọn)</h1>
      <p>So sánh hình dáng chữ bằng một model nhận dạng ảnh, cộng thêm vào số đo giải phẫu. Kết quả Similar sát với cảm nhận bằng mắt hơn, nhất là với font display và script.</p>
      <ul className="fa-onb-list">
        <li>Tải một lần ~371 MB từ Hugging Face (SigLIP 2 của Google, giấy phép Apache-2.0), có kiểm tra checksum.</li>
        {info && info.faces > 0 && <li>Thư viện hiện tại: {info.faces.toLocaleString('vi-VN')} font → phân tích khoảng {mins} phút, chạy nền.</li>}
        {info && <li>Ổ đĩa còn trống: {Math.round(info.free / (1024 * MB))} GB.</li>}
        <li>Mọi xử lý chạy trên máy. Không gửi font hay ảnh đi đâu.</li>
      </ul>
      <p className="fa-onb-note">Không bật thì Similar và Pairing vẫn chạy bằng số đo.</p>
      {short > 0 && <p role="alert">Cần thêm {Math.ceil(short / MB)} MB trống.</p>}
      <Actions>
        <button type="button" onClick={async () => { await api.setSettings({ embedEnabled: false }); await next(); }}>Không dùng</button>
        <button type="button" onClick={async () => { await api.setSettings({ embedEnabled: true }); await next(); }}>Để sau</button>
        <Primary disabled={short > 0} onClick={async () => { await api.setSettings({ embedEnabled: true }); await api.modelDownload(() => {}); await next(); }}>Tải ngay</Primary>
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
      <p>Bật một font trong Font Atlas để dùng ngay trong Photoshop, Illustrator, Figma… mà không cần cài vào hệ thống. File font giữ nguyên chỗ cũ.</p>
      <fieldset className="fa-onb-field"><legend>Khi tắt Font Atlas</legend>
        <label><input type="radio" name="keep" checked={!keep} onChange={() => setKeep(false)} />Tắt luôn các font đã kích hoạt <small>Font chỉ hoạt động khi Font Atlas đang mở; mở lại thì tự kích hoạt lại.</small></label>
        <label><input type="radio" name="keep" checked={keep} onChange={() => setKeep(true)} />Giữ font hoạt động cả khi tắt Font Atlas <small>Dùng được cho tới khi bạn tắt nó trong Font Atlas.</small></label>
      </fieldset>
      {managers.map(m => (
        <p key={m.name} className="fa-onb-warn">⚠ Phát hiện {m.name}{m.activeCount ? ` đang kích hoạt khoảng ${m.activeCount} font` : ' đã cài trên máy'}. Kích hoạt cùng một font ở hai app sẽ tạo bản trùng tên; app thiết kế có thể chọn nhầm bản.</p>))}
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
      <Actions><Primary onClick={() => void finish()}>Vào thư viện</Primary></Actions>
    </>
  );
}
