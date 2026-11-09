import { useEffect, useMemo, useState } from 'react';
import type { Api, LangListDto, FaceQuery } from '../../api';
import { t } from '../../i18n';
import { useUiStore } from '../../store/ui';
import { langName } from './langNames';
import { TermTip } from '../ui/terms';

type Level = 'full' | 'near';

/** Chọn ngôn ngữ cần hỗ trợ. Bấm một dòng: bỏ → full → near → bỏ. 2+ ngôn ngữ thì chọn "đủ tất cả" hay "một trong". */
export function LangPicker({ api, query, refreshKey }: { api: Api; query: FaceQuery; refreshKey?: unknown }): React.JSX.Element {
  const langs = useUiStore(s => s.filters.langs);
  const mode = useUiStore(s => s.filters.langMode) ?? 'all';
  const [list, setList] = useState<LangListDto | null>(null);
  const [search, setSearch] = useState('');
  const key = JSON.stringify({ ...query, langs: undefined, langMode: undefined });
  useEffect(() => {
    let alive = true;
    const timer = setTimeout(() => { api.languageList(JSON.parse(key) as FaceQuery).then(l => { if (alive) setList(l); }, () => { if (alive) setList(null); }); }, 160);
    return () => { alive = false; clearTimeout(timer); };
  }, [api, key, refreshKey]);

  const set = (next: Array<{ code: string; level: Level }>, nextMode = mode) => {
    const { langs: _l, langMode: _m, ...rest } = useUiStore.getState().filters;
    useUiStore.getState().setFilters(next.length ? { ...rest, langs: next, ...(nextMode === 'any' && next.length > 1 ? { langMode: 'any' as const } : {}) } : rest);
  };
  const levelOf = (code: string): Level | null => langs?.find(l => l.code === code)?.level ?? null;
  const cycle = (code: string) => {
    const cur = levelOf(code); const others = (langs ?? []).filter(l => l.code !== code);
    set(cur === null ? [...others, { code, level: 'full' }] : cur === 'full' ? [...others, { code, level: 'near' }] : others);
  };
  const needle = search.trim().toLowerCase();
  const items = useMemo(() => (list?.items ?? []).filter(i => !needle || [i.nameEn, i.autonym, i.code, langName(i)].some(s => s.toLowerCase().includes(needle))), [list, needle]);
  // Ngôn ngữ đang chọn nhưng không còn face nào khớp vẫn hiện để bỏ chọn được.
  const ghosts = (langs ?? []).filter(l => !list?.items.some(i => i.code === l.code));

  return (
    <div className="fa-lang" data-testid="lang-picker">
      <input type="search" className="fa-lang-search" placeholder={t('lang.search')} aria-label={t('lang.search')} value={search} onChange={e => setSearch(e.target.value)} />
      {list && !list.complete && <p className="fa-lang-note" role="status" data-testid="lang-incomplete">{t('lang.incomplete', { done: list.ready.done, total: list.ready.total })}</p>}
      <ul className="fa-lang-list">
        {ghosts.map(g => <li key={g.code}><LangRow name={langName(g)} level={g.level} full={0} near={0} onClick={() => cycle(g.code)} code={g.code} /></li>)}
        {items.map(i => <li key={i.code}><LangRow name={langName(i)} sub={i.autonym !== langName(i) ? i.autonym : undefined} level={levelOf(i.code)} full={i.full} near={i.near} code={i.code} onClick={() => cycle(i.code)} /></li>)}
      </ul>
      {(langs?.length ?? 0) > 1 && (
        <div className="fa-lang-mode" role="radiogroup" aria-label={t('lang.mode')}>
          {(['all', 'any'] as const).map(m => <label key={m}><input type="radio" name="fa-lang-mode" checked={mode === m} onChange={() => set(langs ?? [], m)} />{t(m === 'all' ? 'lang.mode.all' : 'lang.mode.any')}</label>)}
        </div>
      )}
      <p className="fa-lang-gloss"><TermTip term="vi_marks"><span data-term="vi_marks">{t('lang.levels')}</span></TermTip></p>
    </div>
  );
}

function LangRow({ name, sub, level, full, near, code, onClick }: { name: string; sub?: string; level: Level | null; full: number; near: number; code: string; onClick(): void }): React.JSX.Element {
  return (
    <button type="button" className="fa-lang-row" data-code={code} data-level={level ?? 'off'} aria-pressed={level !== null} onClick={onClick}>
      <span className="fa-lang-lv" aria-hidden>{level === 'full' ? '●' : level === 'near' ? '◐' : '○'}</span>
      <span className="fa-lang-name">{name}{sub && <small> {sub}</small>}</span>
      <span className="fa-lang-n" title={t('lang.counts', { full, near })}>{level === 'near' ? near : full}</span>
    </button>
  );
}
