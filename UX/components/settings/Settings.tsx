import { readSplashEnabled, writeSplashEnabled } from '../splash/splashState';
import { useCallback, useEffect, useState } from 'react';
import type { Api, OnboardingDto, ProblemDto, PreviewCacheInfoDto, SettingsDto, SourceDto } from '../../api';
import { openPrivacySettings } from '../../privacy';
import { useScanStore } from '../../store/scan';
import { cloudHint, notDownloadedOf } from '../zero/zeroStates';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';
import { t } from '../../i18n';
import { useModelStore, useModelUi, type ModelStore } from '../../store/model';
import { useUiStore } from '../../store/ui';
import { applyTheme, type ThemeMode } from '../../theme';
import { SourcePriority } from './SourcePriority';
import { Modal } from '../Modal';
import './settings.css';
import { ModelDetails, ModelSpinner } from '../model/ModelFeedback';

export const SETTINGS_GROUPS = [
  { id: 'general', label: 'Chung' }, { id: 'library', label: 'Thư viện và nguồn' }, { id: 'vision', label: 'Phân tích hình ảnh' },
  { id: 'activation', label: 'Kích hoạt font' }, { id: 'license', label: 'Giấy phép' }, { id: 'data', label: 'Dữ liệu' },
  { id: 'shortcuts', label: 'Phím tắt' }, { id: 'help', label: 'Trợ giúp' },
] as const;
type GroupId = typeof SETTINGS_GROUPS[number]['id'];

const SAMPLE_PRESETS: Array<[string, string]> = [
  ['Tiêu đề F&B', 'Phở bò Hà Nội'], ['Chữ hoa chồng dấu', 'ẮẰẲẴẶ ẤẦẨẪẬ ƠỚỜỞỠỢ ƯỨỪỬỮỰ'], ['Thực đơn có giá', 'Cà phê sữa đá 35.000đ'],
  ['Đoạn văn', 'Hà Nội mùa thu, cây cơm nguội vàng'], ['Số và ký hiệu', '0123456789 ₫ & @ % ?'],
];
const SHORTCUTS: Array<[string, string]> = [['⌘K', 'Tìm font'], ['⌘,', 'Mở Cài đặt'], ['Esc', 'Xoá ô tìm / đóng hộp thoại / thoát Cài đặt'], ['↑ ↓', 'Chọn kiểu trong danh sách kiểu chữ'], ['Enter', 'Chọn mục đang trỏ']];
const MB = 1024 * 1024;

type Props = { api: Api; onClose(): void; onRerunOnboarding(): void; modelStore?: ModelStore };

function Switch({ label, on, onChange }: { label: string; on: boolean; onChange(v: boolean): void }) {
  return (
    <div className="fa-set-row">
      <span>{label}</span>
      <button type="button" role="switch" className="fa-switch" aria-checked={on} aria-label={label} onClick={() => onChange(!on)} />
    </div>
  );
}

export function Settings({ api, onClose, onRerunOnboarding, modelStore = useModelStore }: Props): React.JSX.Element {
  const [group, setGroup] = useState<GroupId>('general');
  const [s, setS] = useState<SettingsDto | null>(null);
  useEffect(() => { let alive = true; api.getSettings().then(v => { if (alive) setS(v); }); return () => { alive = false; }; }, [api]);
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      // Esc thuộc về lớp con đang mở (menu ngữ cảnh, hộp thoại xác nhận) trước.
      if (e.key !== 'Escape' || e.defaultPrevented || document.querySelector('[role="menu"], [aria-modal="true"]')) return;
      onClose();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, [onClose]);
  const patch = useCallback(async (p: Partial<SettingsDto>) => {
    setS(prev => (prev ? { ...prev, ...p } : prev));
    await api.setSettings(p);
  }, [api]);

  const label = SETTINGS_GROUPS.find(g => g.id === group)!.label;
  return (
    <div className="fa-settings" data-testid="settings">
      <nav className="fa-set-nav" aria-label="Nhóm cài đặt">
        {SETTINGS_GROUPS.map(g => (
          <button key={g.id} type="button" aria-current={group === g.id ? 'true' : undefined} onClick={() => setGroup(g.id)}>{g.label}</button>
        ))}
      </nav>
      <section className="fa-set-body">
        <header><h2>{label}</h2><button type="button" className="fa-set-done" onClick={onClose}>Xong</button></header>
        {s && group === 'general' && <General api={api} s={s} patch={patch} />}
        {s && group === 'library' && <Library api={api} s={s} patch={patch} />}
        {s && group === 'vision' && <Vision api={api} s={s} patch={patch} store={modelStore} />}
        {s && group === 'activation' && <Activation api={api} s={s} patch={patch} />}
        {group === 'license' && (
          <div className="fa-set-text">
            <p>Bạn tự chịu trách nhiệm tuân thủ giấy phép font của mình.</p>
            <p>Font Atlas phân tích mọi font trong thư viện và không gửi font hay kết quả đi đâu.</p>
          </div>
        )}
        {group === 'data' && <Data api={api} />}
        {group === 'shortcuts' && (
          <table className="fa-set-keys"><tbody>{SHORTCUTS.map(([k, d]) => <tr key={k}><th scope="row"><kbd>{k}</kbd></th><td>{d}</td></tr>)}</tbody></table>
        )}
        {group === 'help' && <Help api={api} onRerun={onRerunOnboarding} />}
      </section>
    </div>
  );
}

type SubProps = { s: SettingsDto; patch(p: Partial<SettingsDto>): Promise<void> };

function General({ api, s, patch }: { api: Api } & SubProps) {
  const [splashOn, setSplashOn] = useState(true);
  useEffect(() => { let alive = true; void readSplashEnabled(api).then(v => { if (alive) setSplashOn(v); }); return () => { alive = false; }; }, [api]);
  const [theme, setTheme] = useState<ThemeMode>(() => (document.documentElement.dataset.theme as ThemeMode | undefined) ?? 'system');
  const setSample = useUiStore(x => x.setSample);
  const apply = (t: string) => { setSample(t); void patch({ sampleText: t }); };
  return (
    <>
      <fieldset className="fa-set-group"><legend>Ngôn ngữ giao diện</legend>
        {([['vi', 'Tiếng Việt'], ['en', 'English']] as const).map(([v, l]) => <label key={v}><input type="radio" name="lang" checked={s.uiLang === v} onChange={() => void patch({ uiLang: v })} />{l}</label>)}
      </fieldset>
      <fieldset className="fa-set-group"><legend>Giao diện</legend>
        {([['system', 'Theo hệ thống'], ['light', 'Sáng'], ['dark', 'Tối']] as const).map(([v, l]) => <label key={v}><input type="radio" name="theme" checked={theme === v} onChange={() => { setTheme(v); applyTheme(v); }} />{l}</label>)}
      </fieldset>
      <div className="fa-set-group"><label htmlFor="set-sample">Chữ mẫu mặc định</label>
        <input id="set-sample" type="text" aria-label="Chữ mẫu mặc định" defaultValue={s.sampleText} key={s.sampleText} onBlur={e => { if (e.target.value !== s.sampleText) apply(e.target.value); }} onChange={e => apply(e.target.value)} />
        <div className="fa-set-presets">{SAMPLE_PRESETS.map(([n, t]) => <button key={n} type="button" onClick={() => apply(t)}>{n}</button>)}</div>
      </div>
      <Switch label="Dự án dùng tiếng Việt mặc định" on={s.projectViDefault} onChange={v => void patch({ projectViDefault: v })} />
      <Switch label="Hiện splash khi mở app" on={splashOn} onChange={v => { setSplashOn(v); void writeSplashEnabled(api, v); }} />
    </>
  );
}

function Library({ api, s, patch }: { api: Api } & SubProps) {
  const [sources, setSources] = useState<SourceDto[]>([]);
  const [removing, setRemoving] = useState<SourceDto | null>(null);
  const refresh = useCallback(async () => setSources(await api.listSources()), [api]);
  useEffect(() => { void refresh(); }, [refresh]);
  const access = useScanStore(x => x.access);
  const cloud = useScanStore(x => x.cloud);
  const state = (x: SourceDto) => (x.id in access ? 'Cần quyền truy cập' : !x.available ? 'Không khả dụng' : x.scanning ? 'Đang quét' : 'Sẵn sàng');
  return (
    <>
      <div className="fa-set-bar"><button type="button" onClick={() => useUiStore.getState().requestAddFolder()}>+ Thêm</button></div>
      <ul className="fa-set-sources">
        {sources.map(x => (
          <li key={x.id} data-testid="source-row">
            <div className="who"><strong>{x.name}{cloudHint(x) && <NoteMark kind="info" inline testId="cloud-hint" text={cloudHint(x)!} />}</strong><span className="path">{x.path}</span></div>
            <span className="meta">{x.faceCount} font · {state(x)}{notDownloadedOf(x, cloud) > 0 ? ` · ${notDownloadedOf(x, cloud)} chưa tải về` : ''}</span>
            {x.id in access && <button type="button" onClick={() => void openPrivacySettings()}>Cấp quyền</button>}
            <button type="button" onClick={() => void api.rescanSources([x.id])}>{x.id in access ? 'Thử lại' : 'Quét lại'}</button>
            <button type="button" className="danger" onClick={() => setRemoving(x)}>Xoá nguồn</button>
          </li>
        ))}
      </ul>
      <SourcePriority api={api} sources={sources} />
      <Switch label="Quét lại khi mở app" on={s.rescanOnLaunch} onChange={v => void patch({ rescanOnLaunch: v })} />
      <Switch label={t('cloud.switch')} on={s.cloudPreviewDownload} onChange={v => void patch({ cloudPreviewDownload: v })} />
      <p className="fa-set-note">{t('cloud.switch.note')}</p>
      <p className="fa-set-note">
        <TermTip term="dataless"><span data-term="dataless" tabIndex={0} className="fa-termword">dataless</span></TermTip>
        {' · '}
        <TermTip term="always_keep"><span data-term="always_keep" tabIndex={0} className="fa-termword">Always keep on this device</span></TermTip>
      </p>
      {removing && (
        <Modal title={`Xoá nguồn “${removing.name}”?`} onClose={() => setRemoving(null)}>
          <p>Chỉ gỡ khỏi thư viện, không xoá file font. Yêu thích, dự án và thẻ được giữ.</p>
          <div className="fa-modal-actions">
            <button type="button" onClick={() => setRemoving(null)}>Huỷ</button>
            <button type="button" className="primary" onClick={async () => { const id = removing.id; setRemoving(null); await api.removeSource(id); await refresh(); }}>Xoá khỏi thư viện</button>
          </div>
        </Modal>
      )}
    </>
  );
}

function Vision({ api, s, patch, store }: { api: Api; store: ModelStore } & SubProps) {
  const ui = useModelUi(store);
  const embed = store(x => x.embed);
  const model = store(x => x.model);
  const line = ui.text ?? (!s.embedEnabled ? 'Đã tắt: Similar và Ghép cặp dùng số đo.' : ui.imageReady ? 'Sẵn sàng' : 'Chưa có trạng thái');
  return (
    <>
      <Switch label="Bật phân tích hình ảnh" on={s.embedEnabled} onChange={async v => { await patch({ embedEnabled: v }); useModelStore.setState({ embedEnabled: v }); store.setState({ embedEnabled: v }); }} />
      <div className="fa-set-group">
        <p data-testid="model-line" role={ui.severity === 'error' ? 'alert' : 'status'}><ModelSpinner ui={ui} />{line}</p>
        {ui.actions.length > 0 && <div className="fa-set-presets">{ui.actions.map(a => <button key={a.label} type="button" onClick={() => void store.getState().act(api, a.id)}>{a.label}</button>)}</div>}
        <ModelDetails detail={ui.detail} />
        {model && model.state === 'missing' && s.embedEnabled && <p className="fa-set-note">Tải một lần khoảng {Math.round(model.totalBytes / MB)} MB, có kiểm tra checksum.</p>}
        {ui.settingsText && <p className="fa-set-note">{ui.settingsText}</p>}
        {embed?.modelVersion && <p className="fa-set-note">Model: {embed.modelVersion}</p>}
      </div>
    </>
  );
}

function Activation({ api, s, patch }: { api: Api } & SubProps) {
  const [ob, setOb] = useState<OnboardingDto | null>(null);
  const [confirm, setConfirm] = useState(false);
  const active = useUiStore(x => x.active);
  useEffect(() => { void api.onboardingState().then(setOb); }, [api]);
  if (ob && !ob.activationSupported) return <p className="fa-set-note">Kích hoạt font chưa có trên hệ điều hành này.</p>;
  return (
    <>
      <fieldset className="fa-set-group"><legend>Khi tắt Font Atlas</legend>
        <label><input type="radio" name="keep" checked={!s.keepActiveOnExit} onChange={() => void patch({ keepActiveOnExit: false })} />Tắt luôn các font đã kích hoạt</label>
        <label><input type="radio" name="keep" checked={s.keepActiveOnExit} onChange={() => void patch({ keepActiveOnExit: true })} />Giữ font hoạt động cả khi tắt Font Atlas</label>
      </fieldset>
      {ob?.fontManagers.filter(m => m.installed).map(m => <p key={m.name} className="fa-set-note">Phát hiện {m.name}{m.activeCount ? `: ~${m.activeCount} font đang kích hoạt` : ' đã cài trên máy'}. Chỉ là thông tin.</p>)}
      <div className="fa-set-group"><button type="button" className="danger" disabled={active.size === 0} onClick={() => setConfirm(true)}>Tắt mọi font đã kích hoạt ({active.size})</button></div>
      {confirm && (
        <Modal title="Tắt mọi font đã kích hoạt?" onClose={() => setConfirm(false)}>
          <p>{active.size} font sẽ được tắt. Font vẫn nằm trong thư viện. Nên làm trước khi gỡ app.</p>
          <div className="fa-modal-actions">
            <button type="button" onClick={() => setConfirm(false)}>Huỷ</button>
            <button type="button" className="primary" onClick={async () => { setConfirm(false); await api.activationDeactivateAll(); await useUiStore.getState().refreshActive(api); }}>Tắt tất cả</button>
          </div>
        </Modal>
      )}
    </>
  );
}

function Data({ api }: { api: Api }) {
  const [backup, setBackup] = useState<string | null>(null);
  const [cache, setCache] = useState<PreviewCacheInfoDto | null>(null);
  const [problems, setProblems] = useState<ProblemDto[]>([]);
  const [ob, setOb] = useState<OnboardingDto | null>(null);
  const load = useCallback(async () => {
    const [c, p, o] = await Promise.all([api.previewCacheInfo(), api.listProblems(), api.onboardingState()]);
    setCache(c); setProblems(p); setOb(o);
  }, [api]);
  useEffect(() => { void load(); }, [load]);
  return (
    <>
      <div className="fa-set-group"><span>Sao lưu dữ liệu người dùng</span>
        <button type="button" onClick={async () => setBackup(await api.backupUserDb())}>Sao lưu ngay</button>
        {backup && <p className="fa-set-note" data-testid="backup-path">{backup}</p>}
        <p className="fa-set-note">Gồm yêu thích, dự án, thẻ và trạng thái kích hoạt.</p>
      </div>
      <div className="fa-set-group"><span>Bộ nhớ đệm preview</span>
        <p className="fa-set-note" data-testid="cache-info">{cache ? `${Math.round(cache.bytes / MB)} MB · ${cache.entries} mục` : '…'}</p>
        <button type="button" onClick={async () => { await api.clearPreviewCache(); await load(); }}>Xoá bộ nhớ đệm</button>
      </div>
      <div className="fa-set-group" data-testid="problems"><span>{problems.length === 0 ? 'Không có font nào có vấn đề' : `${problems.length} font có vấn đề`}</span>
        {problems.length > 0 && <>
          <ul className="fa-set-note">{problems.slice(0, 5).map(p => <li key={`${p.hashHex}${p.faceIndex}`}>{p.code}{p.path ? ` · ${p.path.split('/').pop()}` : ''}</li>)}</ul>
          <button type="button" onClick={async () => { await api.retryProblems(); await load(); }}>Thử lại tất cả</button>
        </>}
      </div>
      {ob?.legacy.found && !ob.legacy.imported && (
        <div className="fa-set-group"><span>Dữ liệu từ phiên bản trước</span>
          <button type="button" onClick={async () => { await api.legacyImport(); await load(); }}>Nhập dữ liệu cũ</button></div>
      )}
    </>
  );
}

function Help({ api, onRerun }: { api: Api; onRerun(): void }) {
  const [version, setVersion] = useState<string | null>(null);
  const [model, setModel] = useState<string | null>(null);
  useEffect(() => {
    void api.embedStatus().then(e => setModel(e.modelVersion));
    if ('__TAURI_INTERNALS__' in window) import('@tauri-apps/api/app').then(m => m.getVersion()).then(setVersion, () => {});
  }, [api]);
  return (
    <>
      <div className="fa-set-group"><button type="button" onClick={async () => { await api.setOnboardingStep('o1'); onRerun(); }}>Chạy lại hướng dẫn</button></div>
      <dl className="fa-set-about">
        <dt>Font Atlas</dt><dd>{version ?? '—'}</dd>
        <dt>Model hình ảnh</dt><dd>{model ?? 'chưa có'}</dd>
      </dl>
    </>
  );
}
