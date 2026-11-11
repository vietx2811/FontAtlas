import type { Api, PlanMemberDto } from '../../api';
import { t } from '../../i18n';
import { useFontStatus } from '../detail/useFontFamily';

/** Dòng mẫu có chữ tiếng Việt bằng chính font của bản này. Hai bản cạnh nhau luôn dùng cùng một dòng mẫu. */
export function Specimen({ api, m, active }: { api: Api; m: PlanMemberDto; /** false: chưa cuộn tới, chưa nạp font */ active: boolean }): React.JSX.Element {
  const st = useFontStatus(api, active ? { hashHex: m.face.hashHex, faceIndex: m.face.faceIndex, fileSize: m.fileSize } : null);
  return (
    <>
      <div className="dup-spec" data-testid="dup-spec" data-status={st.status} lang="vi" style={st.family ? { fontFamily: `"${st.family}", system-ui` } : undefined}>
        {t('dup.sampleText')}
      </div>
      {st.status === 'failed' && <span className="dup-spec-note" role="status">{t('dup.sample.failed')}</span>}
    </>
  );
}
