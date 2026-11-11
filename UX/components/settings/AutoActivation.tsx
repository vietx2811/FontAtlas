import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, AutoAppDto, AutoLogDto, AutoStatusDto, AutoTargetDto } from '../../api';
import { t, type I18nKey } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';
import { Tooltip } from '../ui/Tooltip';

/** Trạng thái app đổi ngoài tầm nhìn của Font Atlas (app chạy/tắt) và không có sự kiện đẩy: đọc lại định kỳ khi trang đang mở. */
export const AUTO_POLL_MS = 7000;
const LOG_LIMIT = 50;
const num = (x: bigint | number | null | undefined): number | null => (x === null || x === undefined ? null : Number(x));
/** incopy dùng chung lệnh cài với indesign (script khởi động chép vào cả hai). */
export const targetOf = (id: string): AutoTargetDto | null => (id === 'illustrator' ? 'illustrator' : id === 'indesign' || id === 'incopy' ? 'indesign' : null);

export function whenText(ms: number, now = Date.now()): string {
  const s = Math.round((now - ms) / 1000);
  if (s < 45) return t('auto.now');
  const rtf = new Intl.RelativeTimeFormat('vi', { numeric: 'auto' });
  if (s < 3600) return rtf.format(-Math.round(s / 60), 'minute');
  if (s < 86400) return rtf.format(-Math.round(s / 3600), 'hour');
  return new Date(ms).toLocaleString('vi-VN', { dateStyle: 'short', timeStyle: 'short' });
}

type Pending = { kind: 'install' | 'remove'; app: AutoAppDto };

function AppRow({ app, busy, onAsk }: { app: AutoAppDto; busy: boolean; onAsk(p: Pending): void }): React.JSX.Element {
  const last = num(app.lastActivityMs);
  const target = targetOf(app.id);
  const state = app.state;
  const foreign = state === 'foreign';
  const stateText = t(`auto.state.${state}` as I18nKey, { app: app.label });
  const stateTone = state === 'ready' ? 'ok' : state === 'error' || state === 'foreign' || state === 'stale' ? 'warn' : 'idle';
  return (
    <li className="fa-auto-app" data-testid="auto-app" data-app={app.id} data-state={state} data-missing={!app.appInstalled || undefined}>
      <div className="fa-auto-name">
        <b>{app.label}</b>
        {app.appRunning && <small>{t('auto.running')}</small>}
        {!app.appInstalled && <small data-testid="auto-notinstalled">{t('auto.notInstalled', { app: app.label })}</small>}
      </div>
      <div className="fa-auto-state" data-tone={stateTone}>
        <span data-testid="auto-state">{stateText}</span>
        {foreign && <NoteMark inline kind="warn" text={t('auto.foreign.tip')} />}
        {state === 'isolated_build' && <NoteMark inline kind="info" text={t('auto.isolated.tip')} />}
        {state === 'error' && app.detail && <NoteMark inline kind="warn" text={app.detail} />}
        {state === 'stale' && app.detail && <NoteMark inline kind="info" text={app.detail} />}
        <small className="fa-auto-last" data-testid="auto-last">{last !== null ? t('auto.lastActivity', { when: whenText(last) }) : t('auto.never')}</small>
      </div>
      <div className="fa-auto-meta">
        <small>{t(`auto.method.${app.method}` as I18nKey)}</small>
        <small className="fa-auto-ev" data-evidence={app.evidence}>{t(`auto.evidence.${app.evidence}` as I18nKey, { app: app.label })}</small>
        {app.restartNeeded && app.appRunning && <small className="fa-auto-restart" data-testid="auto-restart">{t('auto.restart', { app: app.label })}</small>}
      </div>
      <div className="fa-auto-actions">
        {app.installable && target && (
          <>
            {foreign
              ? <Tooltip label={t('auto.foreign.tip')}><button type="button" data-testid="auto-install" aria-disabled="true">{t('auto.install')}</button></Tooltip>
              : <button type="button" data-testid="auto-install" disabled={busy} onClick={() => onAsk({ kind: 'install', app })}>{app.installed ? t('auto.reinstall') : t('auto.install')}</button>}
            {app.installed && <button type="button" data-testid="auto-remove" disabled={busy} onClick={() => onAsk({ kind: 'remove', app })}>{t('auto.remove')}</button>}
          </>
        )}
      </div>
    </li>
  );
}

function LogTable({ rows, appLabel }: { rows: AutoLogDto[]; appLabel(id: string): string }): React.JSX.Element {
  if (!rows.length) return <p className="fa-set-note" data-testid="auto-log-empty">{t('auto.log.empty')}</p>;
  return (
    <table className="fa-auto-log" data-testid="auto-log">
      <thead><tr><th scope="col">{t('auto.log.when')}</th><th scope="col">{t('auto.log.app')}</th><th scope="col">{t('auto.log.font')}</th><th scope="col">{t('auto.log.result')}</th></tr></thead>
      <tbody>
        {rows.map(r => (
          <tr key={String(r.id)} data-outcome={r.outcome}>
            <td>{whenText(Number(r.atMs))}</td>
            <td>{appLabel(r.app)}</td>
            <td>{r.requested}{r.doc && <small className="fa-auto-doc"> · {r.doc}</small>}</td>
            <td>{(() => { const k = `auto.outcome.${r.outcome}`; try { return t(k as I18nKey); } catch { return r.outcome; } })()}{r.outcome === 'ambiguous' && <NoteMark inline kind="info" testId="auto-ambiguous" text={t('auto.outcome.ambiguous.tip')} />}</td>
          </tr>))}
      </tbody>
    </table>
  );
}

/** Settings → Bật font tự động (0.3.0, hợp đồng docs/team/auto-activation/L5_settings_backend.md). */
export function AutoActivation({ api, pollMs = AUTO_POLL_MS }: { api: Api; pollMs?: number }): React.JSX.Element {
  const [status, setStatus] = useState<AutoStatusDto | null>(null);
  const [log, setLog] = useState<AutoLogDto[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [opError, setOpError] = useState<string | null>(null); // lỗi của lệnh cài/gỡ/công tắc: đọc lại trạng thái KHÔNG được xoá nó
  const error = opError ?? loadError;
  const [busy, setBusy] = useState(false);
  const [pending, setPending] = useState<Pending | null>(null);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

  const load = useCallback(async () => {
    try {
      const [s, l] = await Promise.all([api.autoStatus(), api.autoLog(LOG_LIMIT)]);
      if (alive.current) { setStatus(s); setLog(l); setLoadError(null); }
    } catch (e) { if (alive.current) setLoadError(t('auto.loadFail', { msg: msg(e) })); }
  }, [api]);
  useEffect(() => {
    void load();
    const id = setInterval(() => { if (typeof document === 'undefined' || document.visibilityState !== 'hidden') void load(); }, pollMs);
    return () => clearInterval(id);
  }, [load, pollMs]);

  // Mọi lệnh trả trạng thái mới; lỗi thì báo và đọc lại (không tin trạng thái cũ).
  const run = async (fn: () => Promise<AutoStatusDto>) => {
    setBusy(true); setOpError(null);
    try { const s = await fn(); if (alive.current) setStatus(s); void load(); }
    catch (e) { if (alive.current) setOpError(t('auto.opFail', { msg: msg(e) })); void load(); }
    finally { if (alive.current) setBusy(false); }
  };
  const label = (id: string) => status?.apps.find(a => a.id === id)?.label ?? id;
  const running = status?.otherManagers.filter(m => m.running) ?? [];

  return (
    <div className="fa-auto" data-testid="auto-activation">
      <p className="fa-set-note">{t('auto.intro')} <TermTip term="auto_activation"><span data-term="auto_activation">Auto-activation</span></TermTip></p>
      {error && <p role="alert" className="fa-set-note fa-auto-err" data-testid="auto-error">{error} <button type="button" onClick={() => { setOpError(null); void load(); }}>{t('auto.retry')}</button></p>}
      {status && (
        <>
          <div className="fa-set-row">
            <span>{t('auto.switch')}<small className="fa-auto-sub">{t('auto.switch.sub')}</small></span>
            <button type="button" role="switch" className="fa-switch" data-testid="auto-switch" aria-checked={status.enabled} aria-label={t('auto.switch')} disabled={busy}
              onClick={() => void run(() => api.autoSetEnabled(!status.enabled))} />
          </div>
          {status.enabled && !status.brokerRunning && <p className="fa-set-note"><NoteMark inline kind="warn" text={t('auto.broker')} /> {t('auto.broker')}</p>}
          {running.map(m => <p key={m.name} className="fa-set-note fa-auto-mgr" data-testid="auto-manager-warn"><NoteMark inline kind="warn" text={t('auto.managers.warn', { name: m.name })} /> {t('auto.managers.warn', { name: m.name })}</p>)}
          <h3 className="fa-auto-h">{t('auto.apps')}</h3>
          <ul className="fa-auto-apps" aria-label={t('auto.apps')}>
            {status.apps.map(a => <AppRow key={a.id} app={a} busy={busy} onAsk={setPending} />)}
          </ul>
          <h3 className="fa-auto-h">{t('auto.log')}</h3>
          <LogTable rows={log} appLabel={label} />
        </>
      )}
      {pending && (() => {
        const { kind, app } = pending; const target = targetOf(app.id)!;
        const title = kind === 'remove' ? t('auto.remove.title', { app: app.label }) : app.installed ? t('auto.reinstall.title', { app: app.label }) : t('auto.install.title', { app: app.label });
        const body = kind === 'remove' ? t('auto.remove.body', { app: app.label }) : t(`auto.install.body.${target}` as I18nKey);
        return (
          <Modal title={title} onClose={() => setPending(null)}>
            <p>{body}</p>
            <div className="fa-modal-actions">
              <button type="button" data-testid="auto-cancel" onClick={() => setPending(null)}>{t('auto.cancel')}</button>
              <button type="button" className="primary" data-testid="auto-confirm" onClick={() => { setPending(null); void run(() => (kind === 'remove' ? api.autoRemove(target) : api.autoInstall(target))); }}>
                {kind === 'remove' ? t('auto.remove') : app.installed ? t('auto.reinstall') : t('auto.install')}
              </button>
            </div>
          </Modal>
        );
      })()}
    </div>
  );
}
