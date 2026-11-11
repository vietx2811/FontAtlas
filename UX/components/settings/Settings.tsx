import { defaultSample, isDefaultSample } from '../../i18n/sample';
import { readSplashEnabled, writeSplashEnabled } from '../splash/splashState';
import { useCallback, useEffect, useState } from 'react';
import type { Api, OnboardingDto, ProblemDto, PreviewCacheInfoDto, ReplacedEntryDto, SettingsDto, SourceDto } from '../../api';
import { restoreReplaced } from '../activation/restoreReplaced';
import { openPrivacySettings } from '../../privacy';
import { useScanStore } from '../../store/scan';
import { cloudHint, notDownloadedOf } from '../zero/zeroStates';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';
import { chooseLocale, t, useLocale, tn } from '../../i18n';
import { useModelStore, useModelUi, type ModelStore } from '../../store/model';
import { useUiStore } from '../../store/ui';
import { applyTheme, type ThemeMode } from '../../theme';
import { SourcePriority } from './SourcePriority';
import { Modal } from '../Modal';
import { RemoveSourceDialog } from '../sources/RemoveSourceDialog';
import { useNoticeStore } from '../../store/notices';
import './settings.css';
import { ModelDetails, ModelSpinner } from '../model/ModelFeedback';
import { AutoActivation } from './AutoActivation';

export const SETTINGS_GROUPS = [
  { id: 'general', get label() { return t('set.g.general'); } }, { id: 'library', get label() { return t('set.g.library'); } }, { id: 'vision', get label() { return t('set.g.vision'); } },
  { id: 'activation', get label() { return t('set.g.activation'); } }, { id: 'auto', get label() { return t('auto.group'); } }, { id: 'license', get label() { return t('set.g.license'); } }, { id: 'data', get label() { return t('set.g.data'); } },
  { id: 'shortcuts', get label() { return t('set.g.shortcuts'); } }, { id: 'help', get label() { return t('set.g.help'); } },
] as const;
type GroupId = typeof SETTINGS_GROUPS[number]['id'];

const samplePresets = (): Array<[string, string]> => [
  [t('set.preset.fnb'), 'Phở bò Hà Nội'], [t('set.preset.marks'), 'ẮẰẲẴẶ ẤẦẨẪẬ ƠỚỜỞỠỢ ƯỨỪỬỮỰ'], [t('set.preset.menu'), 'Cà phê sữa đá 35.000đ'],
  [t('set.preset.para'), 'Hà Nội mùa thu, cây cơm nguội vàng'], [t('set.preset.numbers'), '0123456789 ₫ & @ % ?'],
];
const shortcuts = (): Array<[string, string]> => [['⌘K', t('set.key.search')], ['⌘,', t('set.key.settings')], ['Esc', t('set.key.esc')], ['↑ ↓', t('set.key.style')], ['Enter', t('set.key.enter')]];
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
      <nav className="fa-set-nav" aria-label={t('set.nav')}>
        {SETTINGS_GROUPS.map(g => (
          <button key={g.id} type="button" aria-current={group === g.id ? 'true' : undefined} onClick={() => setGroup(g.id)}>{g.label}</button>
        ))}
      </nav>
      <section className="fa-set-body">
        <header><h2>{label}</h2><button type="button" className="fa-set-done" onClick={onClose}>{t('set.done')}</button></header>
        {s && group === 'general' && <General api={api} s={s} patch={patch} />}
        {s && group === 'library' && <Library api={api} s={s} patch={patch} />}
        {s && group === 'vision' && <Vision api={api} s={s} patch={patch} store={modelStore} />}
        {s && group === 'activation' && <Activation api={api} s={s} patch={patch} />}
        {group === 'auto' && <AutoActivation api={api} />}
        {group === 'license' && (
          <div className="fa-set-text">
            <p>{t('set.lic1')}</p>
            <p>{t('set.lic2')}</p>
          </div>
        )}
        {group === 'data' && <Data api={api} />}
        {group === 'shortcuts' && (
          <table className="fa-set-keys"><tbody>{shortcuts().map(([k, d]) => <tr key={k}><th scope="row"><kbd>{k}</kbd></th><td>{d}</td></tr>)}</tbody></table>
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
  const locale = useLocale();
  const setSample = useUiStore(x => x.setSample);
  const apply = (t: string) => { setSample(t); void patch({ sampleText: t }); };
  // Chưa từng đổi (backend còn trả mặc định cũ/của ngôn ngữ khác) → hiện mặc định của ngôn ngữ hiện tại.
  const shownSample = isDefaultSample(s.sampleText) ? defaultSample(locale) : s.sampleText;
  return (
    <>
      <fieldset className="fa-set-group"><legend>{t('set.uiLang')}</legend>
        {([['vi', 'Tiếng Việt'], ['en', 'English']] as const).map(([v, l]) => <label key={v}><input type="radio" name="lang" checked={locale === v} onChange={() => { chooseLocale(v); void patch({ uiLang: v }); }} />{l}</label>)}
      </fieldset>
      <fieldset className="fa-set-group"><legend>{t('set.theme')}</legend>
        {([['system', t('set.theme.system')], ['light', t('set.theme.light')], ['dark', t('set.theme.dark')]] as const).map(([v, l]) => <label key={v}><input type="radio" name="theme" checked={theme === v} onChange={() => { setTheme(v); applyTheme(v); }} />{l}</label>)}
      </fieldset>
      <div className="fa-set-group"><label htmlFor="set-sample">{t('set.sample')}</label>
        <input id="set-sample" type="text" aria-label={t('set.sample')} defaultValue={shownSample} key={shownSample} onBlur={e => { if (e.target.value !== shownSample) apply(e.target.value); }} onChange={e => apply(e.target.value)} />
        <div className="fa-set-presets">{samplePresets().map(([n, t]) => <button key={n} type="button" onClick={() => apply(t)}>{n}</button>)}</div>
      </div>
      <Switch label={t('set.projectVi')} on={s.projectViDefault} onChange={v => void patch({ projectViDefault: v })} />
      <Switch label={t('set.splash')} on={splashOn} onChange={v => { setSplashOn(v); void writeSplashEnabled(api, v); }} />
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
  const state = (x: SourceDto) => (x.id in access ? t('set.src.access') : !x.available ? t('set.src.unavail') : x.scanning ? t('set.src.scanning') : t('set.src.ready'));
  return (
    <>
      <div className="fa-set-bar"><button type="button" onClick={() => useUiStore.getState().requestAddFolder()}>{t('set.src.add')}</button></div>
      <ul className="fa-set-sources">
        {sources.map(x => (
          <li key={x.id} data-testid="source-row">
            <div className="who"><strong>{x.name}{cloudHint(x) && <NoteMark kind="info" inline testId="cloud-hint" text={cloudHint(x)!} />}</strong><span className="path">{x.path}</span></div>
            <span className="meta">{tn('set.src.meta', { n: x.faceCount, state: state(x) })}{notDownloadedOf(x, cloud) > 0 ? t('set.src.notDl', { n: notDownloadedOf(x, cloud) }) : ''}</span>
            {x.id in access && <button type="button" onClick={() => void openPrivacySettings()}>{t('set.src.grant')}</button>}
            <button type="button" onClick={() => void api.rescanSources([x.id])}>{x.id in access ? t('set.src.retry') : t('set.src.rescan')}</button>
            <button type="button" className="danger" onClick={() => setRemoving(x)}>{t('set.src.remove')}</button>
          </li>
        ))}
      </ul>
      <SourcePriority api={api} sources={sources} />
      <Switch label={t('set.rescanLaunch')} on={s.rescanOnLaunch} onChange={v => void patch({ rescanOnLaunch: v })} />
      <Switch label={t('cloud.switch')} on={s.cloudPreviewDownload} onChange={v => void patch({ cloudPreviewDownload: v })} />
      <p className="fa-set-note">{t('cloud.switch.note')}</p>
      <p className="fa-set-note">
        <TermTip term="dataless"><span data-term="dataless" tabIndex={0} className="fa-termword">dataless</span></TermTip>
        {' · '}
        <TermTip term="always_keep"><span data-term="always_keep" tabIndex={0} className="fa-termword">Always keep on this device</span></TermTip>
      </p>
      {removing && <RemoveSourceDialog api={api} source={removing} onClose={() => setRemoving(null)}
        onRemoved={rep => { const s0 = removing; setRemoving(null); useNoticeStore.getState().push({ kind: 'info', text: t('rmsrc.done', { name: s0.name, n: rep.removedLocations }) }); useUiStore.getState().bumpLibrary(true); void refresh(); }} />}
    </>
  );
}

function Vision({ api, s, patch, store }: { api: Api; store: ModelStore } & SubProps) {
  const ui = useModelUi(store);
  const embed = store(x => x.embed);
  const model = store(x => x.model);
  const line = ui.text ?? (!s.embedEnabled ? t('set.vis.off') : ui.imageReady ? t('set.vis.ready') : t('set.vis.none'));
  return (
    <>
      <Switch label={t('set.vis.enable')} on={s.embedEnabled} onChange={async v => { await patch({ embedEnabled: v }); useModelStore.setState({ embedEnabled: v }); store.setState({ embedEnabled: v }); }} />
      <div className="fa-set-group">
        <p data-testid="model-line" role={ui.severity === 'error' ? 'alert' : 'status'}><ModelSpinner ui={ui} />{line}</p>
        {ui.actions.length > 0 && <div className="fa-set-presets">{ui.actions.map(a => <button key={a.label} type="button" onClick={() => void store.getState().act(api, a.id)}>{a.label}</button>)}</div>}
        <ModelDetails detail={ui.detail} />
        {model && model.state === 'missing' && s.embedEnabled && <p className="fa-set-note">{t('set.vis.once', { mb: Math.round(model.totalBytes / MB) })}</p>}
        {ui.settingsText && <p className="fa-set-note">{ui.settingsText}</p>}
        {embed?.modelVersion && <p className="fa-set-note">{t('set.vis.model', { v: embed.modelVersion })}</p>}
      </div>
    </>
  );
}

function Activation({ api, s, patch }: { api: Api } & SubProps) {
  const [ob, setOb] = useState<OnboardingDto | null>(null);
  const [confirm, setConfirm] = useState(false);
  const active = useUiStore(x => x.active);
  const [replaced, setReplaced] = useState<ReplacedEntryDto[]>([]);
  const reloadReplaced = useCallback(() => api.activationListReplaced().then(setReplaced, () => setReplaced([])), [api]);
  useEffect(() => { void api.onboardingState().then(setOb); void reloadReplaced(); }, [api, reloadReplaced]);
  if (ob && !ob.activationSupported) return <p className="fa-set-note">{t('set.act.unsupported')}</p>;
  return (
    <>
      <fieldset className="fa-set-group"><legend>{t('set.act.exit')}</legend>
        <label><input type="radio" name="keep" checked={!s.keepActiveOnExit} onChange={() => void patch({ keepActiveOnExit: false })} />{t('set.act.deactAll')}</label>
        <label><input type="radio" name="keep" checked={s.keepActiveOnExit} onChange={() => void patch({ keepActiveOnExit: true })} />{t('set.act.keep')}</label>
      </fieldset>
      {ob?.fontManagers.filter(m => m.installed).map(m => <p key={m.name} className="fa-set-note">{t('set.act.detected', { name: m.name, detail: m.activeCount ? t('set.act.detail.n', { n: m.activeCount }) : t('set.act.detail.installed') })}</p>)}
      <fieldset className="fa-set-group" data-testid="replaced-list"><legend>{t('set.replaced.title')}</legend>
        <p className="fa-set-note">{replaced.length ? t('set.replaced.note') : t('set.replaced.empty')}</p>
        {replaced.map(r => (
          <div key={r.id} className="fa-set-row">
            <span title={r.path}>{r.path} · {r.owner} · {r.scopes.join(' + ')}</span>
            <button type="button" onClick={async () => { await restoreReplaced(api, [r.id]); await reloadReplaced(); }}>{t('set.replaced.restore')}</button>
          </div>
        ))}
      </fieldset>
      <div className="fa-set-group"><button type="button" className="danger" disabled={active.size === 0} onClick={() => setConfirm(true)}>{t('set.act.offAll', { n: active.size })}</button></div>
      {confirm && (
        <Modal title={t('set.act.offTitle')} onClose={() => setConfirm(false)}>
          <p>{tn('set.act.offBody', { n: active.size })}</p>
          <div className="fa-modal-actions">
            <button type="button" onClick={() => setConfirm(false)}>{t('set.cancel')}</button>
            <button type="button" className="primary" onClick={async () => { setConfirm(false); await api.activationDeactivateAll(); await useUiStore.getState().refreshActive(api); }}>{t('set.act.offOk')}</button>
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
      <div className="fa-set-group"><span>{t('set.data.backup')}</span>
        <button type="button" onClick={async () => setBackup(await api.backupUserDb())}>{t('set.data.backupNow')}</button>
        {backup && <p className="fa-set-note" data-testid="backup-path">{backup}</p>}
        <p className="fa-set-note">{t('set.data.backupNote')}</p>
      </div>
      <div className="fa-set-group"><span>{t('set.data.cache')}</span>
        <p className="fa-set-note" data-testid="cache-info">{cache ? tn('set.data.cacheInfo', { mb: Math.round(cache.bytes / MB), n: cache.entries }) : '…'}</p>
        <button type="button" onClick={async () => { await api.clearPreviewCache(); await load(); }}>{t('set.data.clearCache')}</button>
      </div>
      <div className="fa-set-group" data-testid="problems"><span>{problems.length === 0 ? t('set.data.noProblems') : tn('set.data.problems', { n: problems.length })}</span>
        {problems.length > 0 && <>
          <ul className="fa-set-note">{problems.slice(0, 5).map(p => <li key={`${p.hashHex}${p.faceIndex}`}>{p.code}{p.path ? ` · ${p.path.split('/').pop()}` : ''}</li>)}</ul>
          <button type="button" onClick={async () => { await api.retryProblems(); await load(); }}>{t('set.data.retryAll')}</button>
        </>}
      </div>
      {ob?.legacy.found && !ob.legacy.imported && (
        <div className="fa-set-group"><span>{t('set.data.legacy')}</span>
          <button type="button" onClick={async () => { await api.legacyImport(); await load(); }}>{t('set.data.import')}</button></div>
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
      <div className="fa-set-group"><button type="button" onClick={async () => { await api.setOnboardingStep('o1'); onRerun(); }}>{t('set.help.rerun')}</button></div>
      <dl className="fa-set-about">
        <dt>Font Atlas</dt><dd>{version ?? '—'}</dd>
        <dt>{t('set.help.model')}</dt><dd>{model ?? t('set.help.none')}</dd>
      </dl>
    </>
  );
}
