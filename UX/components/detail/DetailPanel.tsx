import { useEffect, useRef } from 'react';
import type { Api } from '../../api';
import { useUiStore, type PanelTab } from '../../store/ui';
import { ClassChip } from './ClassChip';
import { FontInfoCard } from './FontInfoCard';
import { StyleDropdown, styleLabel } from './StyleDropdown';
import { useActivationFlow } from '../activation/useActivationFlow';
import { ActiveRow } from '../activation/ActiveRow';
import { AnatomyTab } from './tabs/AnatomyTab';
import { PairTab } from './tabs/PairTab';
import { SimilarTab } from './tabs/SimilarTab';
import { SpecimenTab } from './tabs/SpecimenTab';
import { ViTab } from './tabs/ViTab';
import { useFaceDetail } from './useFaceDetail';
import { loaderFor, useFontFamily } from './useFontFamily';
import { FaceTitle } from '../ui/FaceTitle';
import { Tooltip } from '../ui/Tooltip';
import { NoteMark } from '../ui/noteMark';
import { t, tn, formatNumber } from '../../i18n';
import './detail.css';
import { ClashNote } from '../ui/ClashNote';
import { clashRoleOf, useNameClashes, vfDisplayName } from '../../store/nameClash';

const tabs = (): Array<[PanelTab, string, string]> => [['spec', t('det.specimen'), 'Aa'], ['vi', t('det.vietnamese'), 'Ặ'], ['anat', t('det.anatomy'), '⌖'], ['sim', t('det.similar'), '◎'], ['pair', t('det.pairing'), '∞']];
const FORMAT: Record<string, string> = { glyf: 'TTF', cff: 'OTF', cff2: 'OTF', none: '' };

export function DetailPanel({ api }: { api: Api }): React.JSX.Element | null {
  useNameClashes();
  const panel = useRef<HTMLElement>(null);
  const selectedId = useUiStore(s => s.selectedId);
  const style = useUiStore(s => s.style);
  const tab = useUiStore(s => s.panelTab);
  const infoExpanded = useUiStore(s => s.infoExpanded);
  const sample = useUiStore(s => s.sample);
  const { setStyle, setPanelTab, toggleInfo } = useUiStore.getState();
  const { detail, styles, error, retry, face } = useFaceDetail(api);
  const flow = useActivationFlow(api);
  const active = useUiStore(s => s.active);
  const available = useUiStore(s => s.available);
  const familyOn = styles.length > 0 && styles.every(s => active.has(s.face.hashHex) || available.has(s.face.hashHex));
  const family = useFontFamily(api, detail ? { hashHex: detail.row.hashHex, faceIndex: detail.row.faceIndex, fileSize: detail.row.fileSize } : null);

  useEffect(() => { if (panel.current) panel.current.scrollTop = 0; }, [selectedId, tab, style]);
  if (!selectedId) return null;
  const shown = detail ? styles.find(s => s.face.hashHex === detail.row.hashHex && s.face.faceIndex === detail.row.faceIndex) : undefined;
  const label = style ?? (shown ? styleLabel(shown) : null) ?? detail?.row.subfamily ?? null;
  const chosen = styles.find(s => styleLabel(s) === label);
  const bodyCss: React.CSSProperties = {
    fontFamily: family ? `"${family}", var(--font-ui)` : undefined, fontWeight: chosen?.weight ?? undefined, fontStyle: chosen?.italic ? 'italic' : undefined,
  };
  const fmt = detail ? [detail.classification ?? '', `${detail.row.isVariable ? 'Variable ' : ''}${FORMAT[detail.file.outline] ?? ''}`.trim(), tn('det.glyphs', { n: formatNumber(detail.file.numGlyphs) })].filter(Boolean).join(' · ') : '';

  return (
    <aside ref={panel} className="fa-panel" aria-label={t('det.panel')}>
      {error ? (
        <div role="alert" className="fa-panel-error">{t('det.readFail', { msg: error })} <button type="button" onClick={retry}>{t('det.retry')}</button></div>
      ) : (
        <div className="fa-p-head">
          <div className="fa-p-row">
            <Tooltip label={t('det.close')}><button type="button" className="fa-panel-close" aria-label={t('det.close')} onClick={() => useUiStore.getState().select(null)}>×</button></Tooltip>
            <Tooltip label={detail?.row.family} onlyIfTruncated>
              <FaceTitle as="h2" api={api} face={detail ? { hashHex: detail.row.hashHex, faceIndex: detail.row.faceIndex, fileSize: detail.row.fileSize } : null} className="fa-p-name">{detail ? vfDisplayName(detail.row.family, clashRoleOf(detail.row.faceId)) : ''}</FaceTitle>
            </Tooltip>
            {detail && <ClashNote faceId={detail.row.faceId} inline />}
          </div>
          <ActiveRow face={detail ? face : null} flow={flow} api={api} />
          <div className="fa-p-sub" data-testid="panel-sub">{fmt}{detail?.row.onCloudOnly && <NoteMark kind="info" inline testId="cloud-only-note" text={t('cloud.note')} />}</div>
          {detail && face && detail.row.hashHex === face.hashHex && detail.row.faceIndex === face.faceIndex && <ClassChip key={`${face.hashHex}:${face.faceIndex}`} api={api} face={face} info={detail.classificationInfo} onChanged={retry} />}
          {styles.length > 0 && <StyleDropdown styles={styles} value={label} onChange={setStyle} loader={loaderFor(api)} family={{ on: familyOn, onToggle: () => void flow.toggle(styles.map(s => s.face), !familyOn) }} />}
          <FontInfoCard detail={detail} expanded={infoExpanded} onToggle={toggleInfo} onReveal={() => { if (face) void api.revealInFinder(face); }} />
        </div>
      )}
      <div className="fa-tabs" role="tablist">
        {tabs().map(([id, name, glyph]) => (
          <Tooltip key={id} label={name}><button type="button" role="tab" aria-label={name} aria-selected={tab === id} onClick={() => setPanelTab(id)}><span className="g">{glyph}</span></button></Tooltip>
        ))}
      </div>
      <div className="fa-p-body" role="tabpanel" data-style={label ?? ''} data-tab={tab}>
        {tab === 'spec' && <SpecimenTab sample={sample} css={bodyCss} />}
        {tab === 'vi' && detail && <ViTab detail={detail} css={bodyCss} />}
        {tab === 'anat' && detail && <AnatomyTab detail={detail} family={family} />}
        {tab === 'sim' && detail && <SimilarTab api={api} detail={detail} />}
        {tab === 'pair' && detail && <PairTab api={api} detail={detail} css={bodyCss} />}
      </div>
    </aside>
  );
}
