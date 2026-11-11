import { useEffect, useState } from 'react';
import type { Api, BootStatusDto } from '../../api';
import { useNoticeStore } from '../../store/notices';
import { needsOnboarding } from '../../store/onboarding';
import { Onboarding } from '../onboarding/Onboarding';
import './boot.css';
import { t, initLocale, hasChosenThisSession } from '../../i18n';

const phaseText = (phase: string): string => ({ starting: t('boot.starting'), scanning: t('boot.scanning'), indexing: t('boot.indexing'), activation: t('boot.activation') } as Record<string, string>)[phase] ?? t('boot.default');

type Gate = { kind: 'loading'; phase: string } | { kind: 'failed'; code: string; message: string; backup: string | null } | { kind: 'onboarding' } | { kind: 'ready' };

/** Cổng khởi động: chờ backend `ready`, báo lỗi nếu không mở được, rồi onboarding (lần đầu) trước khi vào thư viện. */
export function BootGate({ api, children }: { api: Api; children: React.ReactNode }): React.JSX.Element {
  const [gate, setGate] = useState<Gate>({ kind: 'loading', phase: 'starting' });

  useEffect(() => {
    let alive = true; let off: (() => void) | undefined; let announced = false;
    const fail = (b: BootStatusDto | null, e?: Error) => alive && setGate({ kind: 'failed', code: b?.storeError?.code ?? 'internal', message: b?.storeError?.message ?? e?.message ?? t('boot.unknownReason'), backup: b?.userBackup ?? null });
    const advance = async (b: BootStatusDto) => {
      if (!alive) return;
      if (b.phase === 'failed') return fail(b);
      if (b.phase !== 'ready') { setGate({ kind: 'loading', phase: b.phase }); return; }
      if (b.libraryRecreated && !announced) {
        announced = true;
        useNoticeStore.getState().push({ kind: 'info', text: t('boot.recreated') });
      }
      // Backend đã sẵn sàng: đọc lại ngôn ngữ đã lưu (lần vẽ đầu có thể đã dùng ngôn ngữ hệ thống). Không ghi đè lựa chọn vừa làm trong phiên.
      void api.getSettings().then(s => { if (alive && !hasChosenThisSession()) initLocale(s.uiLang); }, () => {});
      const ob = await api.onboardingState();
      if (alive) setGate(needsOnboarding(ob) ? { kind: 'onboarding' } : { kind: 'ready' });
    };
    (async () => {
      off = await api.bootSubscribe(e => {
        if (e.type === 'phase') {
          if (e.phase === 'ready' || e.phase === 'failed') void api.startupStatus().then(advance, err => fail(null, err)); // lấy đủ chi tiết (libraryRecreated, storeError…)
          else setGate({ kind: 'loading', phase: e.phase });
        }
        else if (e.type === 'failed') setGate({ kind: 'failed', code: e.error.code, message: e.error.message, backup: null });
      });
      await advance(await api.startupStatus());
    })().catch(e => fail(null, e));
    return () => { alive = false; off?.(); };
  }, [api]);

  if (gate.kind === 'ready') return <>{children}</>;
  if (gate.kind === 'onboarding') return <Onboarding api={api} onDone={() => setGate({ kind: 'ready' })} />;
  if (gate.kind === 'failed') {
    return (
      <div className="fa-boot" role="alert">
        <h1>{t('boot.failTitle')}</h1>
        <p><code>{gate.code}</code> · {gate.message}</p>
        {gate.backup && <p>{t('boot.backup')} <code>{gate.backup}</code>.</p>}
        <button type="button" onClick={() => location.reload()}>{t('boot.retry')}</button>
      </div>
    );
  }
  return <div className="fa-boot" role="status"><p>{phaseText(gate.phase)}</p></div>;
}
