import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto, PairComboDto } from '../../api';
import type { FaceRowDto } from '../../bindings/FaceRowDto';
import { hasKey, t, type I18nKey } from '../../i18n';
import { useUiStore } from '../../store/ui';
import { AXIS_META } from '../detail/tabs/measurements';
import { PARAGRAPH } from '../detail/tabs/SpecimenTab';
import { useFontFamily } from '../detail/useFontFamily';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import { ZeroState } from '../zero/ZeroState';
import './pair.css';

const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const tr = (key: string, fallback: string): string => (hasKey(key) ? t(key as I18nKey) : fallback);
const css = (family: string | null, row: FaceRowDto) => ({ fontFamily: family ? `"${family}", var(--font-ui)` : 'var(--font-ui)', fontWeight: row.weightClass ?? undefined, fontStyle: row.isItalic ? 'italic' as const : undefined });
const name = (r: FaceRowDto) => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;

function Specimen({ api, row, testId, text, size, tag: Tag }: { api: Api; row: FaceRowDto; testId: string; text: string; size: number; tag: 'div' | 'p' }): React.JSX.Element {
  const family = useFontFamily(api, { hashHex: row.hashHex, faceIndex: row.faceIndex, fileSize: row.fileSize });
  return <Tag data-testid={testId} className={Tag === 'div' ? 'pr-heading' : 'pr-body'} style={{ ...css(family, row), fontSize: size }}>{text}</Tag>;
}

function Combo({ api, c, sample }: { api: Api; c: PairComboDto; sample: string }): React.JSX.Element {
  const a = c.assessment;
  const warns = [...(a.bodyWarnings ?? [])];
  return (
    <article className="pr-combo" data-testid="pair-combo" aria-label={t('pair.combo', { n: c.rank })}>
      <header>
        <b>{t('pair.combo', { n: c.rank })}</b>
        <span className="pr-gate" data-testid="pair-gate" data-gate={a.gate}>{tr(`pair.gate.${a.gate}`, a.gate)}</span>
      </header>
      <div className="pr-page">
        <Specimen api={api} row={c.heading} testId="pair-heading" text={sample} size={28} tag="div" />
        <Specimen api={api} row={c.body} testId="pair-body" text={PARAGRAPH} size={14} tag="p" />
      </div>
      <dl className="pr-names">
        <div><dt>{t('pair.heading')}</dt><dd>{name(c.heading)}</dd></div>
        <div><dt>{t('pair.body')}</dt><dd>{name(c.body)}</dd></div>
      </dl>
      <ul className="pr-labels">
        {a.labels.map(l => {
          const axis = AXIS_META[l.axis]?.label ?? l.axis;
          return <Tooltip key={l.axis} label={l.relation}><li data-testid="pair-label" aria-label={`${axis}: ${l.relation}`}>{l.symbol} {axis}</li></Tooltip>;
        })}
      </ul>
      <p className="pr-summary" data-testid="pair-summary">{a.summary}</p>
      {a.sentences.length > 1 && <ul className="pr-sentences">{a.sentences.map(s => <li key={s}>{s}</li>)}</ul>}
      <div className="pr-warns">
        {a.clashes.length > 0 && <span className="pr-warn">{t('pair.clash', { axes: a.clashes.map(x => AXIS_META[x]?.label ?? x).join(', ') })}</span>}
        {a.headingOverflow && <span className="pr-warn">{t('pair.overflow')}</span>}
        {a.insufficientData && <span className="pr-note">{t('pair.unsure')}</span>}
        {warns.map(w => <span key={w} className="pr-warn">{tr(`pair.warn.${w}`, w)}</span>)}
      </div>
      {a.weightHint && <p className="pr-hint">{a.weightHint}</p>}
    </article>
  );
}

/** Màn "Pair these": 2–4 font → 1–3 Combo (backend thử mọi cách gán vai heading/body), nhãn ●◐◆○ và câu giải thích; không có điểm số. */
export function PairThesePage({ api, faces }: { api: Api; faces: FaceKeyDto[] }): React.JSX.Element {
  const sample = useUiStore(s => s.sample);
  const [combos, setCombos] = useState<PairComboDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [projectVi, setProjectVi] = useState<boolean | null>(null); // null: đang đọc cài đặt mặc định
  const [attempt, setAttempt] = useState(0);
  const title = useRef<HTMLHeadingElement>(null);
  const close = () => useUiStore.getState().closePairThese();
  const idsKey = faces.map(keyOf).join('|');

  useEffect(() => { // mặc định lấy từ cài đặt "Dự án dùng tiếng Việt"
    let alive = true;
    api.getSettings().then(s => { if (alive) setProjectVi(typeof s.projectViDefault === 'boolean' ? s.projectViDefault : true); }, () => { if (alive) setProjectVi(true); });
    return () => { alive = false; };
  }, [api]);

  useEffect(() => {
    if (projectVi === null) return;
    let alive = true; setCombos(null); setError(null);
    api.pairThese(faces, { projectVi, maxCombos: 3 }).then(c => { if (alive) setCombos(c); }, e => { if (alive) setError(e instanceof Error ? e.message : String(e)); });
    return () => { alive = false; };
  }, [api, idsKey, projectVi, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    title.current?.focus();
    return () => { if (before && before.isConnected) before.focus(); };
  }, []);
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented || document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]')) return;
      close();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);

  return (
    <div className="pr">
      <div className="pr-h">
        <Tooltip label={t('pair.back')} shortcut="Esc"><button type="button" className="pr-back" aria-label={t('pair.back')} onClick={close}><Icon name="back" size={14} /></button></Tooltip>
        <h1 ref={title} tabIndex={-1}>{t('pair.title')} · {faces.length}</h1>
        <span className="pr-sp" />
        <label className="pr-switch"><span>{t('pair.vi')}</span>
          <button type="button" role="switch" className="fa-switch" aria-checked={projectVi === true} aria-label={t('pair.vi')} disabled={projectVi === null} onClick={() => setProjectVi(v => !v)} />
        </label>
      </div>
      <div className="pr-body-wrap">
        {error && <div role="alert" className="pr-state warn">{t('pair.error', { msg: error })} <button type="button" onClick={() => setAttempt(n => n + 1)}>{t('pair.retry')}</button></div>}
        {!error && !combos && <div role="status" className="pr-state">{t('pair.loading')}</div>}
        {combos && combos.length === 0 && <ZeroState compact glyph="ơ" text={t('pair.empty')} action={projectVi ? { id: 'vi', label: t('pair.emptyVi') } : undefined} onAction={() => setProjectVi(false)} />}
        {combos && combos.length > 0 && <>
          <div className="pr-list">{combos.map(c => <Combo key={`${keyOf(c.heading)}>${keyOf(c.body)}`} api={api} c={c} sample={sample.trim() || 'Phở bò Hà Nội'} />)}</div>
          <p className="pr-legend">{t('pair.legend')}</p>
        </>}
      </div>
    </div>
  );
}
