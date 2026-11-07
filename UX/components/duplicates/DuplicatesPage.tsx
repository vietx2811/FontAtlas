import { useEffect, useMemo, useState } from 'react';
import type { Api, DuplicateGroupDto, DuplicateMemberDto } from '../../api';
import { hasKey, t, type I18nKey } from '../../i18n';
import { faceKey, toMetadataQuery, useUiStore } from '../../store/ui';
import { useScanStore } from '../../store/scan';
import { Tooltip } from '../ui/Tooltip';
import { ZeroState } from '../zero/ZeroState';
import { dateText, groupTitle, midEllipsis, revisionText } from './duplicatesFormat';
import './duplicates.css';

const KINDS = ['same_file', 'versions', 'formats'] as const;
const humanize = (code: string): string => { const x = code.replace(/_/g, ' ').trim(); return x.charAt(0).toUpperCase() + x.slice(1); };
const kindName = (k: string): string => (hasKey(`dup.kind.${k}`) ? t(`dup.kind.${k}` as I18nKey) : humanize(k));
const kindGloss = (k: string): string | null => (hasKey(`dup.kind.${k}.gloss`) ? t(`dup.kind.${k}.gloss` as I18nKey) : null);

function Member({ api, m }: { api: Api; m: DuplicateMemberDto }): React.JSX.Element {
  const key = { hashHex: m.row.hashHex, faceIndex: m.row.faceIndex };
  // Chỉ chỉ đúng chỗ khi đường dẫn của thành viên chính là đường dẫn của face (same_file có nhiều đường dẫn cho một face).
  const canReveal = !!m.path && m.path === m.row.path;
  return (
    <li className="dup-member" data-testid="dup-member">
      <div className="dup-main">
        <b className="dup-name">{m.row.family}{m.row.subfamily ? ` ${m.row.subfamily}` : ''}</b>
        <span className="dup-fmt">{m.format.toUpperCase()}</span>
        {m.isNewest && <span className="dup-new">{t('dup.newest')}</span>}
      </div>
      <Tooltip label={m.path ?? '—'}><span className="dup-path" data-testid="dup-path" title={m.path ?? undefined}>{m.path ? midEllipsis(m.path, 64) : '—'}</span></Tooltip>
      <div className="dup-meta">
        {m.version === null && m.revision === null && m.modified === null ? <span>—</span> : <>
          <span>{m.version ?? '—'}</span>
          <span>{m.revision !== null ? t('dup.revision', { n: revisionText(m.revision) }) : '—'}</span>
          <span>{dateText(m.modified)}</span>
        </>}
      </div>
      <div className="dup-acts">
        {canReveal && <button type="button" onClick={() => void api.revealInFinder(key).catch(() => {})}>{t('dup.reveal')}</button>}
        <button type="button" onClick={() => useUiStore.getState().select(faceKey(key))}>{t('dup.open')}</button>
      </div>
    </li>
  );
}

function Group({ api, g }: { api: Api; g: DuplicateGroupDto }): React.JSX.Element {
  return (
    <article className="dup-group" data-testid="dup-group">
      <header>
        <b>{groupTitle(g)}</b>
        <span className="dup-count">{t('dup.members', { n: g.faces.length })}</span>
        {g.kind === 'versions' && !g.newestKnown && <span className="dup-unknown">{t('dup.newestUnknown')}</span>}
      </header>
      <ul>{g.faces.map((m, i) => <Member key={`${m.row.hashHex}:${m.row.faceIndex}:${m.path ?? i}`} api={api} m={m} />)}</ul>
    </article>
  );
}

/** Danh sách nhóm font trùng (cùng tệp / phiên bản / định dạng), tôn trọng bộ lọc hiện tại. Chỉ để xem: không có xoá hay di chuyển. */
export function DuplicatesPage({ api }: { api: Api }): React.JSX.Element {
  const ui = useUiStore();
  const version = useScanStore(s => s.version);
  const q = useMemo(() => toMetadataQuery({ view: 'all', tag: null, query: ui.query, viOnly: ui.viOnly, filters: ui.filters }), [ui.query, ui.viOnly, ui.filters]);
  const qKey = JSON.stringify(q);
  const [groups, setGroups] = useState<DuplicateGroupDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let alive = true; setError(null);
    api.duplicateGroups(q).then(g => { if (alive) setGroups(g); }, e => { if (alive) { setGroups(null); setError(e instanceof Error ? e.message : String(e)); } });
    return () => { alive = false; };
  }, [api, qKey, version, ui.libraryRev, attempt]); // eslint-disable-line react-hooks/exhaustive-deps

  // Loại đã biết theo thứ tự cố định, loại lạ (backend mới) xếp sau.
  const kinds = groups ? [...KINDS.filter(k => groups.some(g => g.kind === k)), ...new Set(groups.map(g => g.kind).filter(k => !(KINDS as readonly string[]).includes(k)))] : [];
  return (
    <section className="dup" aria-label={t('dup.title')}>
      <header className="dup-h">
        <h1>{t('dup.title')}</h1>
        {groups && groups.length > 0 && <span className="dup-total">{t('dup.count', { n: groups.length })}</span>}
        <span className="dup-sp" />
        <span className="dup-note">{t('dup.note')}</span>
      </header>
      <div className="dup-body">
        {error && <div role="alert" className="dup-state warn">{t('dup.error', { msg: error })} <button type="button" onClick={() => setAttempt(n => n + 1)}>{t('dup.retry')}</button></div>}
        {!error && !groups && <div role="status" className="dup-state">{t('dup.loading')}</div>}
        {groups && groups.length === 0 && <ZeroState glyph="=" text={t('dup.empty')} />}
        {kinds.map(k => {
          const list = groups!.filter(g => g.kind === k);
          return (
            <section key={k} className="dup-kind" aria-labelledby={`dup-k-${k}`}>
              <h2 id={`dup-k-${k}`}>{kindName(k)}{kindGloss(k) && <small> · {kindGloss(k)}</small>}<span className="dup-total">{list.length}</span></h2>
              {list.map(g => <Group key={`${g.kind}:${g.key}`} api={api} g={g} />)}
            </section>
          );
        })}
      </div>
    </section>
  );
}
