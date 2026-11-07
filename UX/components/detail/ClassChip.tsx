import { useEffect, useRef, useState } from 'react';
import type { Api, ClassificationDto, FaceKeyDto } from '../../api';
import { t } from '../../i18n';
import { HIDDEN_CLASSES, classGloss, classLabel, subclassLabel } from '../main/classLabels';
import { Tooltip } from '../ui/Tooltip';

const PICKABLE = ['serif', 'sans', 'mono', 'script', 'blackletter', 'symbol'];
const USAGE: Record<string, string> = { text: 'Text', display: 'Display', ui: 'UI' };

type Props = { api: Api; face: FaceKeyDto; info: ClassificationDto; onChanged(): void };

/** Chip phân loại ở đầu panel chi tiết: class · subclass, Uncertain → "có lẽ là …", ghi đè qua menu. */
export function ClassChip({ api, face, info, onChanged }: Props): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  // `traits` (Stencil, Rounded…) do backend thêm sau (feat/classify-display-tag): tuỳ chọn, chưa có thì bỏ qua.
  const traits = (info as { traits?: string[] }).traits ?? [];
  const uncertain = info.class === 'uncertain';
  const overridden = info.source === 'override';
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false); };
    document.addEventListener('pointerdown', away);
    return () => document.removeEventListener('pointerdown', away);
  }, [open]);
  const act = async (fn: () => Promise<void>) => { setOpen(false); await fn(); onChanged(); };
  const tip = [
    t('class.tip.conf', { pct: Math.round(info.confidence * 100) }),
    ...info.signals.map(s => `${s.source}: ${s.detail} → ${classLabel(s.vote)}`),
    traits.length ? traits.join(', ') : '',
    info.usage.length ? t('class.tip.usage', { usage: info.usage.map(u => USAGE[u] ?? u).join(', ') }) : '',
  ].filter(Boolean).join(' · ');
  return (
    <div className="fa-class" ref={root} data-testid="class-wrap">
      <Tooltip label={tip}>
        <button type="button" className="fa-class-chip" data-testid="class-chip" data-uncertain={uncertain} aria-haspopup="menu" aria-expanded={open} aria-label={t('class.menu')} onClick={() => setOpen(v => !v)}>
          <span className="c" title={classGloss(info.class)}>{classLabel(info.class)}</span>
          {uncertain && info.candidate && <span className="m">{t('class.maybe', { label: classLabel(info.candidate) })}</span>}
          {traits.map(x => <span key={x} className="m">· {x}</span>)}
          {!uncertain && info.subclass && <span className="m">· {subclassLabel(info.subclass)}</span>}
        </button>
      </Tooltip>
      {overridden && <Tooltip label={t('class.machine', { label: info.autoClass ? classLabel(info.autoClass) : '—' })}><span className="fa-class-edited" tabIndex={0}>{t('class.edited')}</span></Tooltip>}
      {open && <div role="menu" className="fa-class-menu" onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); setOpen(false); } }}>
        {PICKABLE.filter(c => !HIDDEN_CLASSES.has(c)).map(c => <button key={c} type="button" role="menuitem" data-current={info.class === c} onClick={() => void act(() => api.setClassificationOverride({ face, class: c, subclass: null }))}>{classLabel(c)}<small>{classGloss(c)}</small></button>)}
        {overridden && <button type="button" role="menuitem" className="auto" onClick={() => void act(() => api.clearClassificationOverride(face))}>{t('class.auto')}</button>}
      </div>}
    </div>
  );
}
