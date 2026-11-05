import { useEffect, useState } from 'react';
import type { Api } from '../../api';
import { toGridQuery, useUiStore, type View } from '../../store/ui';

const FIXED: Record<string, string> = { all: 'Tất cả', star: 'Yêu thích', recent: 'Gần đây', active: 'Đang bật' };

async function titleOf(api: Api, view: View): Promise<string> {
  if (view in FIXED) return FIXED[view];
  const id = Number(view.slice(4));
  if (view.startsWith('col:')) return (await api.listCollections()).find(c => c.id === id)?.name ?? 'Dự án';
  return (await api.listSources()).find(s => s.id === id)?.name ?? 'Nguồn';
}

/** Tiêu đề mục + số kết quả của bộ lọc hiện tại. count = null khi backend chưa lọc được mục này. */
export function useViewSummary(api: Api): { title: string; count: number | null } {
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => s.tag);
  const query = useUiStore(s => s.query);
  const viOnly = useUiStore(s => s.viOnly);
  const [out, setOut] = useState<{ title: string; count: number | null }>({ title: FIXED[view] ?? '', count: null });

  useEffect(() => {
    let alive = true;
    const q = toGridQuery({ view, tag, query, viOnly });
    Promise.all([titleOf(api, view), q ? api.queryFaces({ ...q, limit: 1 }).then(p => p.total) : Promise.resolve(null)])
      .then(([title, count]) => { if (alive) setOut({ title, count }); })
      .catch(() => { if (alive) setOut(o => ({ ...o, count: null })); });
    return () => { alive = false; };
  }, [api, view, tag, query, viOnly]);
  return out;
}
