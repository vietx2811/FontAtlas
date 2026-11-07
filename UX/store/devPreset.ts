// Chỉ cho dev/QA/ảnh chụp: dựng trạng thái UI từ hash URL (index.tsx chỉ gọi khi import.meta.env.DEV).
// Ví dụ: #sel=Public%20Sans:Regular&style=Bold&tab=vi&info=1&size=50&q=sans  ·  #classes=serif,sans
import type { Api } from '../api';
import type { MockOptions } from '../api/mock';
import { useModelStore } from './model';
import { faceKey, useUiStore, type PanelTab, type View } from './ui';

export async function applyDevPreset(api: Api, hash: string): Promise<void> {
  const p = new URLSearchParams(hash.replace(/^#/, ''));
  const ui = useUiStore.getState();
  if (p.get('enable') === '1') { await api.setSettings({ embedEnabled: true }); useModelStore.setState({ embedEnabled: true }); }
  const view = p.get('view'); if (view) ui.setView(view as View);
  if (p.get('settings') === '1') ui.setSettingsOpen(true);
  const q = p.get('q'); if (q) ui.setQuery(q);
  const size = Number(p.get('size')); if (size) ui.setSampleSize(size);
  const sel = p.get('sel');
  if (sel) {
    const [family, sub] = sel.split(':');
    const items = (await api.queryFaces({ text: family, limit: 200 })).items.filter(r => r.family === family);
    const row = items.find(r => !sub || r.subfamily === sub) ?? items[0];
    if (row) ui.select(faceKey(row));
  }
  const classes = p.get('classes'); if (classes) ui.setFilters({ ...useUiStore.getState().filters, classes: classes.split(',') });
  const style = p.get('style'); if (style) useUiStore.getState().setStyle(style);
  const tab = p.get('tab'); if (tab) ui.setPanelTab(tab as PanelTab);
  if (p.get('info') === '1' && !useUiStore.getState().infoExpanded) ui.toggleInfo();
}

const MB = 1024 * 1024;
/** #model=missing|downloading|checking|tls|cancelled|paused|checksum|corrupt|ready  #embed=running|paused|done|failed  (mock: mở app ở đúng trạng thái E*) */
export function devInitial(hash: string): NonNullable<MockOptions['initial']> {
  const p = new URLSearchParams(hash.replace(/^#/, ''));
  const out: NonNullable<MockOptions['initial']> = {};
  const model = p.get('model'); const embed = p.get('embed');
  if (model === 'downloading') out.model = { state: 'downloading', doneBytes: 140 * MB, bytesPerSec: 8_500_000, etaMs: 60_000 };
  else if (model === 'checking') out.model = { state: 'downloading', doneBytes: 371 * MB, totalBytes: 371 * MB, bytesPerSec: 0, etaMs: null };
  else if (model === 'tls') out.model = { state: 'paused', doneBytes: 12 * MB, lastError: { code: 'tls', message: 'invalid peer certificate: UnknownIssuer; proxy CA missing' } };
  else if (model === 'cancelled') out.model = { state: 'paused', doneBytes: 100 * MB, lastError: { code: 'cancelled', message: 'download cancelled by user' } };
  else if (model === 'paused') out.model = { state: 'paused', doneBytes: 100 * MB };
  else if (model === 'checksum') out.model = { state: 'checksum_mismatch' };
  else if (model === 'corrupt') out.model = { state: 'corrupt' };
  else if (model === 'ready') out.model = { state: 'ready', doneBytes: 371 * MB };
  else if (model === 'missing') out.model = { state: 'missing' };
  if (embed) {
    out.model ??= { state: 'ready', doneBytes: 371 * MB };
    out.embed = { enabled: true, running: embed === 'running' || embed === 'paused', paused: embed === 'paused', pending: embed === 'done' || embed === 'failed' ? 0 : 3388, done: embed === 'done' ? 3800 : embed === 'failed' ? 3788 : 412, failed: embed === 'failed' ? 12 : 0, device: 'metal', msPerFace: 28, etaMs: 120_000, modelVersion: 'siglip2' };
  }
  return out;
}

/** Dựng dữ liệu cho dev/QA trước khi render (chạy cả với backend thật): #addsource=<đường dẫn>&ob=done */
export async function applyDevBoot(api: Api, hash: string): Promise<void> {
  const p = new URLSearchParams(hash.replace(/^#/, ''));
  const path = p.get('addsource');
  if (path) { try { await api.addSource(path); } catch (e) { console.warn('addsource bỏ qua:', (e as Error).message); } }
  if (p.get('ob') === 'done') { await api.setSettings({ licenseNoticeAck: true }); await api.setOnboardingStep('done'); }
}
