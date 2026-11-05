import { useEffect, useState } from 'react';
import type { Api, FaceDetailDto, SimilarSource } from '../../../api';
import { useModelStore, useModelUi, type ModelStore } from '../../../store/model';
import { faceKey, useUiStore } from '../../../store/ui';
import { ZeroState } from '../../zero/ZeroState';
import { AXIS_META } from './AnatomyTab';

export const SOURCE_BADGE: Record<SimilarSource, string> = { axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' };
const names = (keys: string[]): string => keys.map(k => (AXIS_META[k]?.label ?? k).toLowerCase()).join(', ');

/** Ba font gần nhất. Không điểm, không phần trăm: chỉ "Giống ở / Khác ở" và nguồn phân tích. */
const HINT_OFF = 'ui.similar_hint_off';

/** Dải gợi ý phân tích hình ảnh theo trạng thái model/embed (UX 03 §3.3). */
function ImageStrip({ api, store }: { api: Api; store: ModelStore }): React.JSX.Element | null {
  const ui = useModelUi(store);
  const embedEnabled = store(s => s.embedEnabled);
  const [off, setOff] = useState(true); // mặc định ẩn cho tới khi đọc xong setting
  useEffect(() => { let alive = true; api.getSetting(HINT_OFF).then(v => { if (alive) setOff(v === '1'); }, () => { if (alive) setOff(false); }); return () => { alive = false; }; }, [api]);
  const act = (id: Parameters<ReturnType<typeof store.getState>['act']>[1]) => void store.getState().act(api, id);

  let body: React.ReactNode = null; let actions: React.ReactNode = null;
  if (ui.code === 'E2') body = `${ui.text!.replace(/^Đang tải model (\d+%).*$/, 'Đang tải model ($1)')}. Kết quả hiện chỉ dựa trên số đo.`;
  else if (ui.code === 'E8' || ui.code === 'E9') body = `${ui.code === 'E9' ? 'Phân tích hình ảnh tạm dừng' : 'Đang phân tích hình ảnh'} ${ui.text!.match(/(\d[\d.]*\/\d[\d.]*)/)![1]} — kết quả sẽ đầy đủ hơn khi xong`;
  else if (ui.severity === 'error') {
    body = `${ui.text}${ui.detail ? ` — ${ui.detail}` : ''}`;
    actions = ui.actions.map(a => <button key={a.label} type="button" onClick={() => act(a.id)}>{a.label}</button>);
  } else if (!ui.imageReady && !off) {
    body = `Kết quả chỉ dựa trên số đo giải phẫu. Bật phân tích hình ảnh để so cả hình dáng chữ (~371 MB, khoảng 2 phút cho thư viện của bạn).`;
    actions = <>
      <button type="button" onClick={() => void store.getState().enable(api)}>{embedEnabled ? 'Tải model' : 'Bật'}</button>
      <button type="button" onClick={() => { setOff(true); void api.setSetting(HINT_OFF, '1'); }}>Không hiện lại</button>
    </>;
  }
  if (body === null) return null;
  return <div className="fa-strip" data-testid="image-strip" role="note"><span>ⓘ {body}</span><span className="fa-strip-actions">{actions}</span></div>;
}

export function SimilarTab({ api, detail, store = useModelStore }: { api: Api; detail: FaceDetailDto; store?: ModelStore }): React.JSX.Element {
  const select = useUiStore(s => s.select);
  const strip = <ImageStrip api={api} store={store} />;
  const ui = useModelUi(store);
  if (detail.similarTop.length === 0) {
    return (
      <>
        {strip}
        <ZeroState compact glyph="ơ" text="Font này có quá ít số đo để tìm font tương tự"
          action={ui.imageReady ? undefined : { id: 'enable', label: 'Bật phân tích hình ảnh' }} onAction={() => void store.getState().enable(api)} />
      </>
    );
  }
  return (
    <>
    {strip}
    <ul className="fa-sim">
      {detail.similarTop.map(h => (
        <li key={faceKey(h.face)} data-testid="similar-hit" className="fa-sim-hit">
          <button type="button" className="fa-sim-name" onClick={() => select(faceKey(h.face))}>
            {h.row.family}{h.row.subfamily ? ` ${h.row.subfamily}` : ''}
          </button>
          <span className="fa-badge" data-testid="source-badge">{SOURCE_BADGE[h.source]}</span>
          <p className="fa-sim-why">Giống ở: {names(h.same)} · Khác ở: {names(h.differ)}</p>
        </li>
      ))}
    </ul>
    </>
  );
}
