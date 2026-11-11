import { useEffect, useState } from 'react';
import type { Api } from '../../api';
import { useScanStore } from '../../store/scan';
import { toGridQuery, useUiStore, type View } from '../../store/ui';
import { t } from '../../i18n';

const FIXED = (): Record<string, string> => ({ all: t('side.all'), star: t('side.favorites'), recent: t('side.recent'), active: t('side.active'), dups: t('main.dups') });

async function titleOf(api: Api, view: View): Promise<string> {
  if (view in FIXED()) return FIXED()[view];
  const id = Number(view.slice(4));
  if (view.startsWith('col:')) return (await api.listCollections()).find(c => c.id === id)?.name ?? t('main.project');
  return (await api.listSources()).find(s => s.id === id)?.name ?? t('main.source');
}

/** Tiêu đề mục + số kết quả của bộ lọc hiện tại. count = null khi backend chưa lọc được mục này. */
export function useViewSummary(api: Api): { title: string; count: number | null } {
  const filters = useUiStore(s => s.filters);
  const familyScope = useUiStore(s => s.familyScope);
  const view = useUiStore(s => s.view);
  const query = useUiStore(s => s.query);
  const viOnly = useUiStore(s => s.viOnly);
  const scanVersion = useScanStore(s => s.version);
  const recentViews = useUiStore(s => s.recentViews);
  const active = useUiStore(s => s.active);
  const [out, setOut] = useState<{ title: string; count: number | null }>({ title: FIXED[view] ?? '', count: null });

  useEffect(() => {
    let alive = true;
    const q = toGridQuery({ view, query, viOnly, recentViews, active, filters, familyScope });
    Promise.all([titleOf(api, view), api.queryFaces({ ...q, limit: 1 }).then(p => p.total)])
      .then(([title, count]) => { if (alive) setOut({ title: familyScope?.name ?? title, count }); })
      .catch(() => { if (alive) setOut(o => ({ ...o, count: null })); });
    return () => { alive = false; };
  }, [api, view, query, viOnly, recentViews, active, scanVersion, filters, familyScope]);
  return out;
}
