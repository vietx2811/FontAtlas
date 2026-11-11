import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, AutoAppDto, AutoLogDto, AutoStatusDto, AutoTargetDto } from '../../api';
import { hasKey, t, type I18nKey } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';

/** Trạng thái app đổi ngoài tầm nhìn của Font Atlas (app chạy/tắt) và không có sự kiện đẩy: đọc lại định kỳ khi trang đang mở. */
export const AUTO_POLL_MS = 7000;
const LOG_LIMIT = 50;
const num = (x: bigint | number | null | undefined): number | null => (x === null || x === undefined ? null : Number(x));
/** incopy dùng chung lệnh cài với indesign (script khởi động chép vào cả hai). */
export const targetOf = (id: string): AutoTargetDto | null => (id === 'illustrator' ? 'illustrator' : id === 'indesign' || id === 'incopy' ? 'indesign' : null);

export function whenText(ms: number, now = Date.now()): string {
  const s = Math.round((now - ms) / 1000);
  if (s < 45) return t('auto.now');
  const rtf = new Intl.RelativeTimeFormat(t('auto.locale'), { numeric: 'auto' });
  if (s < 3600) return rtf.format(-Math.round(s / 60), 'minute');
  if (s < 86400) return rtf.format(-Math.round(s / 3600), 'hour');
  return new Date(ms).toLocaleString(t('auto.locale'), { dateStyle: 'short', timeStyle: 'short' });
}

type Pending = { kind: 'install' | 'remove'; app: AutoAppDto } | { kind: 'forget' };

function AppRow({ app, busy, isolated, onAsk }: { app: AutoAppDto; busy: boolean; isolated: boolean; onAsk(p: Pending): void }): React.JSX.Element {
  const last = num(app.lastActivityMs);
  const target = targetOf(app.id);
  const state = app.state;
  const foreign = state === 'foreign';
  const stateText = t(`auto.state.${state}` as I18nKey, { app: app.label });
  // "Sẵn sàng" chỉ xanh khi đã đo với app thật; chưa kiểm thì trung tính để không hứa quá.
  const stateTone = state === 'ready' ? (app.evidence === 'untested' ? 'idle' : 'ok') : state === 'error' || state === 'foreign' || state === 'stale' ? 'warn' : 'idle';
  const why = isolated ? t('auto.isolatedBuild.why') : foreign ? t('auto.foreign.tip') : busy ? t('auto.busy') : null; // lý do nút bị khoá: đọc được bằng bàn phím (aria-describedby)
  const whyId = `auto-why-${app.id}`;
  const installDisabled = foreign || busy || isolated;
  const removeDisabled = busy || isolated;
  return (
    <li className="fa-auto-app" tabIndex={-1} data-testid="auto-app" data-app={app.id} data-state={state} data-missing={!app.appInstalled || undefined}>
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
            <button type="button" data-testid="auto-install" aria-disabled={installDisabled || undefined} aria-describedby={installDisabled && why ? whyId : undefined}
              onClick={() => { if (!installDisabled) onAsk({ kind: 'install', app }); }}>{app.installed ? t('auto.reinstall') : t('auto.install')}</button>
            {app.installed && <button type="button" data-testid="auto-remove" aria-disabled={removeDisabled || undefined} aria-describedby={removeDisabled && why ? whyId : undefined} onClick={() => { if (!removeDisabled) onAsk({ kind: 'remove', app }); }}>{t('auto.remove')}</button>}
            {why && <span id={whyId} className="fa-sr-only">{why}</span>}
          </>
        )}
      </div>
    </li>
  );
}

const outcomeText = (o: string): string => { const k = `auto.outcome.${o}`; return hasKey(k) ? t(k) : o; }; // outcome lạ của backend mới: hiện nguyên chuỗi gốc, không để lộ khoá

function LogTable({ rows, appLabel }: { rows: AutoLogDto[]; appLabel(id: string): string }): React.JSX.Element {
  if (!rows.length) return <p className="fa-set-note" data-testid="auto-log-empty">{t('auto.log.empty')}</p>;
  return (
    <table className="fa-auto-log" data-testid="auto-log" aria-label={t('auto.log')}>
      <caption className="fa-sr-only">{t('auto.log')}</caption>
      <thead><tr><th scope="col">{t('auto.log.when')}</th><th scope="col">{t('auto.log.app')}</th><th scope="col">{t('auto.log.font')}</th><th scope="col">{t('auto.log.result')}</th></tr></thead>
      <tbody>
        {rows.map(r => {
          const at = Number(r.atMs);
          return (
            <tr key={String(r.id)} data-outcome={r.outcome}>
              <td><time dateTime={new Date(at).toISOString()}>{whenText(at)}</time></td>
              <td>{appLabel(r.app)}</td>
              <td>{r.requested}{r.doc && <small className="fa-auto-doc"> · {r.doc}</small>}</td>
              <td>{outcomeText(r.outcome)}{r.outcome === 'ambiguous' && <NoteMark inline kind="info" testId="auto-ambiguous" text={t('auto.outcome.ambiguous.tip')} />}</td>
            </tr>);
        })}
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
  const [live, setLive] = useState('');
  const alive = useRef(true);
  const seq = useRef(0); // mỗi lệnh thay đổi tăng seq: kết quả của lần đọc bắt đầu TRƯỚC đó bị bỏ (poll cũ không ghi đè trạng thái mới)
  const inflight = useRef(false);
  const running = useRef(false); // chống bấm đôi
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

  const load = useCallback(async (force = false) => {
    if (!force && (inflight.current || running.current)) return; // không chồng request, không đọc giữa lúc đang cài/gỡ
    inflight.current = true; const mine = seq.current;
    try {
      const [s, l] = await Promise.all([api.autoStatus(), api.autoLog(LOG_LIMIT)]);
      if (alive.current && mine === seq.current) { setStatus(s); setLog(l); setLoadError(null); }
    } catch (e) { if (alive.current && mine === seq.current) setLoadError(t('auto.loadFail', { msg: msg(e) })); }
    finally { inflight.current = false; }
  }, [api]);
  useEffect(() => {
    void load();
    const id = setInterval(() => { if (typeof document === 'undefined' || document.visibilityState !== 'hidden') void load(); }, pollMs);
    // Quay lại cửa sổ / tab: đọc ngay, đừng đợi nhịp kế (app có thể đã chạy hoặc tắt trong lúc vắng mặt).
    const back = () => { if (document.visibilityState !== 'hidden') void load(); };
    document.addEventListener('visibilitychange', back); window.addEventListener('focus', back);
    return () => { clearInterval(id); document.removeEventListener('visibilitychange', back); window.removeEventListener('focus', back); };
  }, [load, pollMs]);

  const focusRow = (id: string) => requestAnimationFrame(() => {
    const row = document.querySelector<HTMLElement>(`[data-testid=auto-app][data-app="${id}"]`);
    (row?.querySelector<HTMLElement>('[data-testid=auto-install], [data-testid=auto-remove]') ?? row)?.focus();
  });
  // Mọi lệnh trả trạng thái mới; lỗi thì báo và đọc lại (không tin trạng thái cũ).
  const run = async (fn: () => Promise<AutoStatusDto>, announce: string, focusApp?: string) => {
    if (running.current) return;
    running.current = true; seq.current++; setBusy(true); setOpError(null);
    try { const s = await fn(); seq.current++; if (alive.current) { setStatus(s); setLive(announce); } }
    catch (e) { seq.current++; if (alive.current) setOpError(t('auto.opFail', { msg: msg(e) })); }
    finally { running.current = false; if (alive.current) setBusy(false); void load(true); if (focusApp) focusRow(focusApp); }
  };
  const label = (id: string) => status?.apps.find(a => a.id === id)?.label ?? id;
  // Khoá ký riêng của máy: gỡ panel KHÔNG xoá khoá; chỉ cho xoá khi panel Illustrator đã gỡ (06).
  const keyShown = !!status && status.signingKeyPresent && status.apps.find(a => a.id === 'illustrator')?.installed === false;
  const runningMgrs = status?.otherManagers.filter(m => m.running) ?? [];

  return (
    <div className="fa-auto" data-testid="auto-activation">
      <p className="fa-set-note">{t('auto.intro')} <TermTip term="auto_activation"><span data-term="auto_activation">Auto-activation</span></TermTip></p>
      <div role="status" aria-live="polite" className="fa-sr-only" data-testid="auto-live">{live}</div>
      {error && (
        <div className="fa-set-note fa-auto-err" data-testid="auto-error">
          <p role="alert">{error}</p>
          <button type="button" onClick={() => { setOpError(null); void load(true); }}>{t('auto.retry')}</button>
        </div>
      )}
      {status && (
        <>
          <div className="fa-set-row">
            <span>{t('auto.switch')}<small className="fa-auto-sub">{t('auto.switch.sub')}</small></span>
            <button type="button" role="switch" className="fa-switch" data-testid="auto-switch" aria-checked={status.enabled} aria-label={t('auto.switch')} aria-disabled={busy || undefined}
              onClick={() => { if (!busy) void run(() => api.autoSetEnabled(!status.enabled), t(status.enabled ? 'auto.live.off' : 'auto.live.on')); }} />
          </div>
          {status.isolatedBuild && <p className="fa-set-note" data-testid="auto-isolated"><NoteMark inline kind="info" text={t('auto.isolatedBuild.why')} /> {t('auto.isolatedBuild.short')}</p>}
          {status.enabled && !status.brokerRunning && <p className="fa-set-note" data-testid="auto-broker-warn">{t('auto.broker')}</p>}
          {status.enabled && status.fontRequestError && (
            <p className="fa-set-note" data-testid="auto-fontreq-warn">
              <NoteMark inline kind="warn" text={t('auto.fontreq.warn', { why: t(status.fontRequestError === 'timeout' ? 'auto.fontreq.timeout' : status.fontRequestError === 'thread' ? 'auto.fontreq.thread' : 'auto.fontreq.refused') })} /> {t('auto.fontreq.short')}
              {runningMgrs.length > 0 && <> {t('auto.fontreq.hint', { names: runningMgrs.map(m => m.name).join(', ') })}</>}
            </p>
          )}
          {runningMgrs.map(m => <p key={m.name} className="fa-set-note fa-auto-mgr" data-testid="auto-manager-warn"><NoteMark inline kind="warn" text={t('auto.managers.warn', { name: m.name })} /> {t('auto.managers.short', { name: m.name })}</p>)}
          <h3 className="fa-auto-h">{t('auto.apps')}</h3>
          <ul className="fa-auto-apps" aria-label={t('auto.apps')}>
            {status.apps.map(a => <AppRow key={a.id} app={a} busy={busy} isolated={status.isolatedBuild} onAsk={setPending} />)}
          </ul>
          {keyShown && (
            <div className="fa-set-note fa-auto-key" data-testid="auto-key">
              <span>{t('auto.key.note')}</span>
              <button type="button" data-testid="auto-forget" aria-disabled={busy || status.isolatedBuild || undefined} aria-describedby="auto-key-why"
                onClick={() => { if (!busy && !status.isolatedBuild) setPending({ kind: 'forget' }); }}>{t('auto.key.forget')}</button>
              {(busy || status.isolatedBuild) && <span id="auto-key-why" className="fa-sr-only">{status.isolatedBuild ? t('auto.isolatedBuild.why') : t('auto.busy')}</span>}
            </div>
          )}
          <h3 className="fa-auto-h">{t('auto.log')}</h3>
          <LogTable rows={log} appLabel={label} />
        </>
      )}
      {pending && (() => {
        if (pending.kind === 'forget') {
          return (
            <Modal title={t('auto.key.title')} onClose={() => setPending(null)}>
              <p>{t('auto.key.body')}</p>
              <div className="fa-modal-actions">
                <button type="button" data-testid="auto-cancel" onClick={() => setPending(null)}>{t('auto.cancel')}</button>
                <button type="button" className="primary" data-testid="auto-confirm" onClick={() => { setPending(null); void run(() => api.autoForgetKey(), t('auto.live.keyForgotten')); }}>{t('auto.key.forget')}</button>
              </div>
            </Modal>
          );
        }
        const { kind, app } = pending; const target = targetOf(app.id)!;
        const title = kind === 'remove' ? t('auto.remove.title', { app: app.label }) : app.installed ? t('auto.reinstall.title', { app: app.label }) : t('auto.install.title', { app: app.label });
        const body = kind === 'remove' ? t('auto.remove.body', { app: app.label }) : t(`auto.install.body.${target}` as I18nKey);
        return (
          <Modal title={title} onClose={() => { setPending(null); focusRow(app.id); }}>
            <p>{body}</p>
            <div className="fa-modal-actions">
              <button type="button" data-testid="auto-cancel" onClick={() => { setPending(null); focusRow(app.id); }}>{t('auto.cancel')}</button>
              <button type="button" className="primary" data-testid="auto-confirm" onClick={() => {
                setPending(null);
                void run(() => (kind === 'remove' ? api.autoRemove(target) : api.autoInstall(target)), t(kind === 'remove' ? 'auto.live.removed' : 'auto.live.installed', { app: app.label }), app.id);
              }}>
                {kind === 'remove' ? t('auto.remove') : app.installed ? t('auto.reinstall') : t('auto.install')}
              </button>
            </div>
          </Modal>
        );
      })()}
    </div>
  );
}
