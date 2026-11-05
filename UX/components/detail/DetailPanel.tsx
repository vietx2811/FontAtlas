import { useCallback } from 'react';
import type { Api } from '../../api';
import { useUiStore, type PanelTab } from '../../store/ui';
import { FontInfoCard } from './FontInfoCard';
import { StyleDropdown, styleLabel } from './StyleDropdown';
import { useFaceDetail } from './useFaceDetail';
import { useFontFamily } from './useFontFamily';
import './detail.css';

const TABS: Array<[PanelTab, string, string]> = [['spec', 'Specimen', 'Aa'], ['vi', 'Tiếng Việt', 'Ặ'], ['anat', 'Giải phẫu', '⌖'], ['sim', 'Tương tự', '◎'], ['pair', 'Ghép cặp', '∞']];
const FORMAT: Record<string, string> = { glyf: 'TTF', cff: 'OTF', cff2: 'OTF', none: '' };

export function DetailPanel({ api }: { api: Api }): React.JSX.Element | null {
  const selectedId = useUiStore(s => s.selectedId);
  const style = useUiStore(s => s.style);
  const tab = useUiStore(s => s.panelTab);
  const infoExpanded = useUiStore(s => s.infoExpanded);
  const sample = useUiStore(s => s.sample);
  const active = useUiStore(s => s.active);
  const { setStyle, setPanelTab, toggleInfo, refreshActive } = useUiStore.getState();
  const { detail, styles, error, retry, face } = useFaceDetail(api);
  const family = useFontFamily(api, detail ? { hashHex: detail.row.hashHex, faceIndex: detail.row.faceIndex, fileSize: detail.row.fileSize } : null);
  const familyOf = useCallback(() => null, []); // font từng kiểu trong danh sách: dùng weight/italic CSS trên họ đã nạp

  if (!selectedId) return null;
  const isActive = !!detail && active.has(detail.row.hashHex);
  const shown = detail ? styles.find(s => s.face.hashHex === detail.row.hashHex && s.face.faceIndex === detail.row.faceIndex) : undefined;
  const label = style ?? (shown ? styleLabel(shown) : null) ?? detail?.row.subfamily ?? null;
  const chosen = styles.find(s => styleLabel(s) === label);
  const toggle = async () => {
    if (!face) return;
    try { await (isActive ? api.activationDeactivate([face]) : api.activationActivate([face])); await refreshActive(api); }
    catch (e) { console.warn('activation failed', e); }
  };
  const fmt = detail ? [detail.classification ?? '', `${detail.row.isVariable ? 'Variable ' : ''}${FORMAT[detail.file.outline] ?? ''}`.trim(), `${detail.file.numGlyphs.toLocaleString('vi-VN')} glyph`].filter(Boolean).join(' · ') : '';

  return (
    <aside className="fa-panel" aria-label="Chi tiết font">
      {error ? (
        <div role="alert" className="fa-panel-error">Không đọc được font này: {error} <button type="button" onClick={retry}>Thử lại</button></div>
      ) : (
        <div className="fa-p-head">
          <div className="fa-p-row">
            <h2 className="fa-p-name" style={{ fontFamily: family ? `"${family}", var(--font-ui)` : undefined }} title={detail?.row.family}>{detail?.row.family ?? ''}</h2>
            <button type="button" role="switch" className="fa-switch" aria-checked={isActive} aria-label="Kích hoạt font" title="Kích hoạt" disabled={!detail} onClick={toggle} />
          </div>
          <div className="fa-p-sub" data-testid="panel-sub">{fmt}</div>
          {styles.length > 0 && <StyleDropdown styles={styles} value={label} onChange={setStyle} familyOf={familyOf} />}
          <FontInfoCard detail={detail} expanded={infoExpanded} onToggle={toggleInfo} onReveal={() => { if (face) void api.revealInFinder(face); }} />
        </div>
      )}
      <div className="fa-tabs" role="tablist">
        {TABS.map(([id, name, glyph]) => (
          <button key={id} type="button" role="tab" aria-label={name} title={name} aria-selected={tab === id} onClick={() => setPanelTab(id)}><span className="g">{glyph}</span></button>
        ))}
      </div>
      <div className="fa-p-body" role="tabpanel" data-style={label ?? ''} data-tab={tab}
        style={{ fontFamily: family ? `"${family}", var(--font-ui)` : undefined, fontWeight: chosen?.weight ?? undefined, fontStyle: chosen?.italic ? 'italic' : undefined }}>
        {tab === 'spec' ? <div className="fa-big">{sample.trim() || 'Phở bò Hà Nội'}</div> : <div className="fa-soon">Tab này nằm ở T9.</div>}
      </div>
    </aside>
  );
}
