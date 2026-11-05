// Chỉ cho dev/QA/ảnh chụp: dựng trạng thái UI từ hash URL (index.tsx chỉ gọi khi import.meta.env.DEV).
// Ví dụ: #sel=Public%20Sans:Regular&style=Bold&tab=vi&info=1&size=50&q=sans
import type { Api } from '../api';
import { faceKey, useUiStore, type PanelTab } from './ui';

export async function applyDevPreset(api: Api, hash: string): Promise<void> {
  const p = new URLSearchParams(hash.replace(/^#/, ''));
  const ui = useUiStore.getState();
  const q = p.get('q'); if (q) ui.setQuery(q);
  const size = Number(p.get('size')); if (size) ui.setSampleSize(size);
  const sel = p.get('sel');
  if (sel) {
    const [family, sub] = sel.split(':');
    const items = (await api.queryFaces({ text: family, limit: 200 })).items.filter(r => r.family === family);
    const row = items.find(r => !sub || r.subfamily === sub) ?? items[0];
    if (row) ui.select(faceKey(row));
  }
  const style = p.get('style'); if (style) useUiStore.getState().setStyle(style);
  const tab = p.get('tab'); if (tab) ui.setPanelTab(tab as PanelTab);
  if (p.get('info') === '1' && !useUiStore.getState().infoExpanded) ui.toggleInfo();
}
