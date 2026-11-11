import { useState } from 'react';
import type { Api, PlanItemDto, PlanMemberDto } from '../../api';
import { t } from '../../i18n';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';
import { useNoticeStore } from '../../store/notices';
import { faceKey, useUiStore } from '../../store/ui';
import { memberName, viewOf } from './dupPlan';
import { blockedText, formatBytes, reasonText, reviewText, warnText } from './dupText';
import { dateText, revisionText, splitPath, tailDepth } from './duplicatesFormat';
import { Specimen } from './Specimen';
import { useSeen } from './useInView';

const FORMAT_TERM: Record<string, string> = { otf: 'opentype', ttf: 'truetype', ttc: 'font_collection', otc: 'font_collection' };
const KNOWN_KINDS = ['same_file', 'versions', 'formats'];
export const kindLabel = (k: string): string => (KNOWN_KINDS.includes(k) ? t(`dup.kind.${k}` as 'dup.kind.same_file') : k.replace(/_/g, ' '));
export const kindGloss = (k: string): string | null => (KNOWN_KINDS.includes(k) ? t(`dup.kind.${k}.gloss` as 'dup.kind.same_file.gloss') : null);

function FormatChip({ format }: { format: string }): React.JSX.Element {
  const chip = <span className="dup-fmt" tabIndex={0}>{format.toUpperCase()}</span>;
  const term = FORMAT_TERM[format];
  return term ? <TermTip term={term}>{chip}</TermTip> : chip;
}

function Details({ api, m, showVersion }: { api: Api; m: PlanMemberDto; showVersion: boolean }): React.JSX.Element {
  const p = splitPath(m.path, 3);
  const reveal = async () => { try { await api.revealLocation(m.path); } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('dup.revealFail', { msg: e instanceof Error ? e.message : String(e) }) }); } };
  type Row = { k: string; v: React.ReactNode; term?: string };
  const rows: Row[] = [
    { k: t('dup.d.path'), v: <span className="dup-path" title={m.path}>{p.head && <span className="dup-head">{p.head}</span>}<span className="dup-tail">{p.tail}</span></span> },
    { k: t('dup.d.source'), v: m.sourceName },
    { k: t('dup.d.format'), v: <FormatChip format={m.format} /> },
    ...(showVersion ? [{ k: t('dup.d.version'), v: m.version ?? '—' }, { k: t('dup.d.revision'), v: m.revision !== null ? revisionText(m.revision) : '—', term: 'revision' }, { k: t('dup.d.modified'), v: dateText(m.modified) }] : []),
    { k: t('dup.d.glyphs'), v: m.numGlyphs !== null ? String(m.numGlyphs) : '—', term: 'glyph' },
    { k: t('dup.d.size'), v: formatBytes(m.fileSize) },
    { k: t('dup.d.vi'), v: m.viMissing === 0 ? t('dup.d.viFull') : t('dup.d.viMissing', { n: m.viMissing }) },
    { k: m.isVariable ? t('dup.d.variable') : t('dup.d.static'), v: m.active ? t('dup.d.active') : '', term: m.isVariable ? 'variable_font' : 'static_font' },
  ];
  return (
    <div className="dup-detail" data-testid="dup-detail">
      <dl>{rows.map(r => <div key={r.k}><dt>{r.term ? <TermTip term={r.term}><span tabIndex={0}>{r.k}</span></TermTip> : r.k}</dt><dd>{r.v}</dd></div>)}</dl>
      <div className="dup-acts">
        <button type="button" onClick={() => void reveal()}>{t('dup.reveal')}</button>
        <button type="button" onClick={() => useUiStore.getState().select(faceKey({ hashHex: m.face.hashHex, faceIndex: m.face.faceIndex }))}>{t('dup.open')}</button>
      </div>
    </div>
  );
}

function Side({ api, m, role, active, onKeep, note, canKeep, depth }: { api: Api; m: PlanMemberDto; role: 'keep' | 'hide'; active: boolean; onKeep?: () => void; note?: React.ReactNode; canKeep: boolean; depth: number }): React.JSX.Element {
  return (
    <div className={`dup-side ${role}`} data-testid={`dup-side-${role}`}>
      <div className="dup-side-h">
        <span className={`dup-role ${role}`}>{role === 'keep' ? t('dup.keep') : t('dup.hideLabel')}</span>
        <b className="dup-name" title={m.path}>{memberName(m)}</b>
        <FormatChip format={m.format} />
        {m.isVariable && <TermTip term="variable_font"><span className="dup-fmt" tabIndex={0}>{t('dup.d.variable')}</span></TermTip>}
        {m.viMissing > 0 && <NoteMark text={t('dup.d.viMissing', { n: m.viMissing })} inline />}
      </div>
      <Specimen api={api} m={m} active={active} />
      <div className="dup-where" title={m.path}>{m.sourceName} · <span className="dup-tail">{splitPath(m.path, depth).tail}</span></div>
      {note}
      {role === 'hide' && canKeep && onKeep && <button type="button" className="dup-linkbtn" onClick={onKeep}>{t('dup.swap.to')}</button>}
    </div>
  );
}

/** Một nhóm trùng: hai (hoặc nhiều) bản cạnh nhau cùng dòng mẫu, lý do bằng câu thường, chi tiết kỹ thuật sau "Chi tiết". */
export function GroupCard({ api, item, choice, onSwap, onResolve, onIgnore, busy }: {
  api: Api; item: PlanItemDto; /** Đường dẫn bản giữ người dùng đã chọn lại (nếu có). */ choice?: string; onSwap(path: string): void; onResolve(): void; onIgnore(): void; busy: boolean;
}): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const [ref, seen] = useSeen<HTMLElement>();
  const v = viewOf(item, choice);
  const sameFile = item.kind === 'same_file';
  const hasVer = item.kind === 'versions';
  const swapTo = (m: PlanMemberDto) => onSwap(m.path);
  const depth = tailDepth(item.memberPaths);
  return (
    <article ref={ref} className={`dup-group${item.autoSafe ? '' : ' review'}`} data-testid="dup-group" data-kind={item.kind} aria-label={memberName(v.keeper)}>
      <header>
        <b>{memberName(v.keeper)}</b>
        <span className="dup-count">{t('dup.members', { n: v.all.length })}</span>
        {item.kind === 'versions' && item.review?.code === 'newest_unknown' && <span className="dup-unknown">{t('dup.newestUnknown')}</span>}
      </header>
      {item.review && !v.chosen && <p className="dup-review" data-testid="dup-review"><span className="dup-excl" aria-hidden="true">!</span>{reviewText(item.review)}</p>}
      <div className="dup-cols" role="group" aria-label={t('dup.sample.cols')}>
        <Side api={api} m={v.keeper} role="keep" active={seen} canKeep={false} depth={depth} note={<p className="dup-why" data-testid="dup-why">{reasonText(v.reason)}{item.warnings.map(w => <NoteMark key={w.code} text={warnText(w)} inline />)}</p>} />
        <div className="dup-others">{v.losers.map(l => <Side key={l.path} api={api} m={l} role="hide" active={seen} canKeep depth={depth} onKeep={() => swapTo(l)} />)}</div>
      </div>
      {sameFile && <p className="dup-same">{t('dup.sample.same')}</p>}
      {open && <div className="dup-details">{v.all.map(m => <Details key={m.path} api={api} m={m} showVersion={hasVer || m.version !== null} />)}<p className="dup-key">{t('dup.d.key')}: <code>{item.key}</code></p></div>}
      <footer className="dup-foot">
        <button type="button" className="primary" aria-disabled={busy} onClick={() => { if (!busy) onResolve(); }}>{t('dup.resolve')}</button>
        {v.losers.length === 1 && <button type="button" aria-disabled={busy} onClick={() => { if (!busy) swapTo(v.losers[0]); }}>{t('dup.swap')}</button>}
        <button type="button" aria-disabled={busy} onClick={() => { if (!busy) onIgnore(); }}>{t('dup.ignore')}</button>
        <span className="dup-sp" />
        <button type="button" className="dup-linkbtn" aria-expanded={open} onClick={() => setOpen(o => !o)}>{open ? t('dup.details.hide') : t('dup.details')}</button>
      </footer>
      {v.losers.some(l => !l.trash.allowed) && open && <p className="dup-key">{v.losers.filter(l => !l.trash.allowed).map(l => `${memberName(l)}: ${blockedText(l.trash.blocked)}`).join(' · ')}</p>}
    </article>
  );
}
