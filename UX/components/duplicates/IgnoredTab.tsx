import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, IgnoredGroupDto } from '../../api';
import { t } from '../../i18n';
import { Tooltip } from '../ui/Tooltip';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { kindLabel } from './GroupCard';
import { groupTitle, splitPath } from './duplicatesFormat';

/** Tab "Đã bỏ qua": các nhóm người dùng đã bỏ qua, có nút "Hiện lại". */
export function IgnoredTab({ api }: { api: Api }): React.JSX.Element {
  const rev = useUiStore(s => s.libraryRev);
  const [list, setList] = useState<IgnoredGroupDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const seq = useRef(0);
  const load = useCallback(() => {
    const mine = ++seq.current; setError(null);
    api.duplicateIgnored().then(l => { if (mine === seq.current) setList(l); }, e => { if (mine === seq.current) setError(e instanceof Error ? e.message : String(e)); });
  }, [api]);
  useEffect(() => { load(); return () => { seq.current++; }; }, [load, rev]);
  const restore = async (g: IgnoredGroupDto) => {
    try { await api.duplicateUnignore(g.group.kind, g.group.key); useUiStore.getState().bumpLibrary(); useNoticeStore.getState().push({ kind: 'info', text: t('dup.ign.restored', { name: groupTitle(g.group) }) }); load(); }
    catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('dup.ign.fail', { msg: e instanceof Error ? e.message : String(e) }) }); }
  };
  return (
    <div className="dup-tabbody" data-testid="dup-ignored">
      <p className="dup-note">{t('dup.ign.note')}</p>
      {error && <div role="alert" className="dup-state warn">{t('dup.done.error', { msg: error })} <button type="button" onClick={load}>{t('dup.retry')}</button></div>}
      {!error && !list && <div role="status" className="dup-state">{t('dup.done.loading')}</div>}
      {list && list.length === 0 && <div className="dup-state">{t('dup.ign.empty')}</div>}
      {list?.map(g => (
        <article key={`${g.group.kind}:${g.group.key}`} className="dup-group" data-testid="dup-ign-row">
          <header>
            <b>{groupTitle(g.group)}</b><span className="dup-fmt">{kindLabel(g.group.kind)}</span>
            <span className="dup-count">{t('dup.members', { n: g.group.faces.length })}</span>
            <span className="dup-sp" />
            <span className="dup-muted">{t('dup.ign.at', { d: new Date(g.ignoredAt * 1000).toLocaleDateString('vi-VN') })}</span>
            <button type="button" onClick={() => void restore(g)}>{t('dup.ign.show')}</button>
          </header>
          <ul className="dup-paths">{g.group.faces.map((f, i) => <li key={i} className="dup-path">{f.path ? <Tooltip label={f.path}><span tabIndex={0}>{splitPath(f.path, 2).tail}</span></Tooltip> : '—'}</li>)}</ul>
        </article>
      ))}
    </div>
  );
}
