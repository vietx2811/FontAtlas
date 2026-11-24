import type { Api } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { CREDITS, CREDIT_GROUPS } from './creditsData';

/** «Giới thiệu và ghi công»: dữ liệu, model và thư viện bên thứ ba cùng giấy phép (nghĩa vụ ghi công của Apache-2.0/MIT/BSD). */
export function Credits({ api }: { api: Api }): React.JSX.Element {
  const openAll = async () => { try { await api.openThirdPartyNotices(); } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('credits.allLicences.fail', { msg: e instanceof Error ? e.message : String(e) }) }); } };
  return (
    <div className="fa-credits" data-testid="credits">
      <p className="fa-set-note">{t('credits.intro')}</p>
      {CREDIT_GROUPS.map(g => (
        <section key={g} className="fa-credits-group" aria-labelledby={`credits-${g}`}>
          <h3 id={`credits-${g}`}>{t(`credits.group.${g}`)}</h3>
          <ul>
            {CREDITS.filter(c => c.group === g).map(c => (
              <li key={c.id} data-testid={`credit-${c.id}`}>
                <div className="fa-credits-head"><b>{c.name}</b><span className="fa-credits-lic">{c.licence}</span></div>
                <div className="fa-credits-by">{c.by}</div>
                <div className="fa-credits-what">{t(c.what)}</div>
                {c.id === 'hyperglot' && <div className="fa-credits-what">{t('credit.hyperglot')} · {t('credits.hyperglot.changes')}</div>}
                {c.fullText && <div className="fa-credits-by">{t('credits.fullText', { file: c.fullText })}</div>}
              </li>
            ))}
          </ul>
        </section>
      ))}
      <p className="fa-set-note"><button type="button" data-testid="credits-all-licences" onClick={() => void openAll()}>{t('credits.allLicences')}</button> {t('credits.allLicences.note')}</p>
      <p className="fa-set-note">{t('credits.fonts')}</p>
    </div>
  );
}
