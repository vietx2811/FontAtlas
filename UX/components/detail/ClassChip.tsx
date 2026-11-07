import { useEffect, useRef, useState } from 'react';
import type { Api, ClassificationDto, FaceKeyDto } from '../../api';
import { t, type I18nKey } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { HIDDEN_CLASSES, classGloss, classLabel, subclassLabel } from '../main/classLabels';
import { Tooltip } from '../ui/Tooltip';

const PICKABLE = ['serif', 'sans', 'mono', 'script', 'blackletter', 'symbol'];
const tr = (key: string, fallback: string): string => { try { return (t as (k: string) => string)(key) || fallback; } catch { return fallback; } };
const signalSource = (s: string) => tr(`class.signal.${s}`, s);
const usageLabel = (u: string) => tr(`class.usage.${u}`, u);

type Props = { api: Api; face: FaceKeyDto; info: ClassificationDto; onChanged(): void };

/**
 * Chip phân loại ở đầu panel chi tiết: class · subclass, Uncertain → "có lẽ là …", ghi đè qua menu.
 * Dùng với `key={faceKey}` để đổi font thì menu đóng và trạng thái đặt lại.
 */
export function ClassChip({ api, face, info, onChanged }: Props): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const chip = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  // `traits` (Stencil, Rounded…) do backend thêm sau (feat/classify-display-tag): tuỳ chọn, chưa có thì bỏ qua.
  const traits = info.traits ?? [];
  const uncertain = info.class === 'uncertain';
  const overridden = info.source === 'override';
  const items = () => [...(menu.current?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
  const close = (refocus: boolean) => { setOpen(false); if (refocus) chip.current?.focus(); };

  useEffect(() => {
    if (!open) return;
    const els = items();
    (els.find(e => e.getAttribute('aria-checked') === 'true') ?? els[0])?.focus();
    const away = (e: PointerEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false); };
    document.addEventListener('pointerdown', away);
    return () => document.removeEventListener('pointerdown', away);
  }, [open]);

  const act = async (fn: () => Promise<void>) => {
    close(true);
    try {
      await fn();
    } catch (e) {
      useNoticeStore.getState().push({ kind: 'error', text: t('class.error', { msg: e instanceof Error ? e.message : String(e) }) });
      return;
    }
    useUiStore.getState().bumpLibrary(true); // lưới và facet đang lọc theo class phải đọc lại
    onChanged();
  };
  const onMenuKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(true); return; }
    if (e.key === 'Tab') { close(true); return; }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const els = items(); const i = els.indexOf(document.activeElement as HTMLElement);
    els[(i + (e.key === 'ArrowDown' ? 1 : -1) + els.length) % els.length]?.focus();
  };

  const tip = [
    t('class.tip.conf', { pct: Math.round(info.confidence * 100) }),
    ...info.signals.map(s => `${signalSource(s.source)}: ${s.detail} → ${classLabel(s.vote)}`),
    traits.length ? traits.join(', ') : '',
    info.usage.length ? t('class.tip.usage', { usage: info.usage.map(usageLabel).join(', ') }) : '',
  ].filter(Boolean).join(' · ');
  const spoken = [classLabel(info.class), uncertain && info.candidate ? t('class.maybe', { label: classLabel(info.candidate) }) : info.subclass ? subclassLabel(info.subclass) : ''].filter(Boolean).join(', ');
  return (
    <div className="fa-class" ref={root} data-testid="class-wrap">
      <Tooltip label={tip}>
        <button ref={chip} type="button" className="fa-class-chip" data-testid="class-chip" data-uncertain={uncertain} aria-haspopup="menu" aria-expanded={open} aria-label={`${spoken} — ${t('class.menu')}`} onClick={() => setOpen(v => !v)}>
          <span className="c" title={classGloss(info.class)}>{classLabel(info.class)}</span>
          {uncertain && info.candidate && <span className="m">{t('class.maybe', { label: classLabel(info.candidate) })}</span>}
          {traits.map(x => <span key={x} className="m">· {x}</span>)}
          {!uncertain && info.subclass && <span className="m">· {subclassLabel(info.subclass)}</span>}
        </button>
      </Tooltip>
      {overridden && <Tooltip label={t('class.machine', { label: info.autoClass ? classLabel(info.autoClass) : '—' })}><span className="fa-class-edited" tabIndex={0}>{t('class.edited')}</span></Tooltip>}
      {open && <div ref={menu} role="menu" aria-label={t('class.menu')} className="fa-class-menu" onKeyDown={onMenuKey}>
        {PICKABLE.filter(c => !HIDDEN_CLASSES.has(c)).map(c => <button key={c} type="button" role="menuitemradio" aria-checked={info.class === c} data-current={info.class === c} onClick={() => void act(() => api.setClassificationOverride({ face, class: c, subclass: null }))}>{classLabel(c)}<small>{classGloss(c)}</small></button>)}
        {overridden && <button type="button" role="menuitem" className="auto" onClick={() => void act(() => api.clearClassificationOverride(face))}>{t('class.auto')}</button>}
      </div>}
    </div>
  );
}
