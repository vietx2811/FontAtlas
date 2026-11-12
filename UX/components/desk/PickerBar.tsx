import { t, tn, formatNumber } from '../../i18n';
import { classLabel } from '../main/classLabels';
import { TermTip, classTermKey } from '../ui/terms';
import { Tooltip } from '../ui/Tooltip';
import { PROFILES, SOURCES, type PickSource, type Profile } from './pickerSources';

export const PICK_CLASSES = ['sans', 'serif', 'mono', 'slab', 'script', 'handwriting', 'display', 'blackletter'] as const;
const profileName = (p: Profile) => t(p === 'label' ? 'pick.profile.label.name' : (`pick.profile.${p}` as 'pick.profile.body'));

/** Đầu Picker (G5): nguồn (chọn một), hồ sơ vai khi cần, chip class, "Đủ tiếng Việt", đếm và xoá lọc. */
export function PickerBar({ source, onSource, disabled, counts, classes, onClasses, viOnly, onViOnly, shown, hidden, profile, profileGuess, onProfile, showProfile, clearRecent }: {
  source: PickSource; onSource(s: PickSource): void; /** Nguồn mờ đi + lý do (tooltip). */ disabled: Partial<Record<PickSource, string>>; counts: Partial<Record<PickSource, number>>;
  classes: string[]; onClasses(c: string[]): void; viOnly: boolean; onViOnly(v: boolean): void; shown: number; hidden: number;
  profile: Profile | null; profileGuess: Profile | null; onProfile(p: Profile | null): void; showProfile: boolean; clearRecent?: () => void;
}): React.JSX.Element {
  const filtered = classes.length > 0 || viOnly;
  return (
    <div className="fa-pickbar" data-testid="picker-bar">
      <div className="fa-pick-src" role="radiogroup" aria-label={t('pick.src.label')}>
        {SOURCES.map((s, i) => {
          const why = disabled[s];
          const btn = (
            <button key={s} type="button" role="radio" aria-checked={source === s} aria-disabled={why ? true : undefined} data-testid={`pick-src-${s}`} data-src={s}
              onClick={() => { if (!why) onSource(s); }}>{t(`pick.src.${s}` as 'pick.src.pair')}{counts[s] !== undefined && <i>{counts[s]}</i>}<kbd aria-hidden>{i + 1}</kbd></button>
          );
          return <Tooltip key={s} label={why ?? t(`pick.src.${s}.gloss` as 'pick.src.pair.gloss')}>{btn}</Tooltip>;
        })}
      </div>
      {showProfile && (
        <label className="fa-pick-profile" data-testid="pick-profile-row">
          <span>{profile === null && profileGuess === null ? t('pick.profile.hint') : profile === null ? t('pick.profile.guess', { profile: profileName(profileGuess!) }) : t('pick.profile.label')}</span>
          <select data-testid="pick-profile" value={profile ?? ''} aria-label={t('pick.profile.label')} onChange={e => onProfile((e.target.value || null) as Profile | null)}>
            <option value="">{profileGuess ? profileName(profileGuess) + ' ?' : t('pick.profile.none')}</option>
            {PROFILES.map(p => <option key={p} value={p}>{profileName(p)}</option>)}
          </select>
        </label>
      )}
      <div className="fa-pick-classes" role="group" aria-label={t('pick.class')}>
        <span className="fa-pick-lbl">{t('pick.class')}</span>
        {PICK_CLASSES.map(c => {
          const on = classes.includes(c);
          return <TermTip key={c} term={classTermKey(c)}><button type="button" data-testid="pick-class" data-class={c} aria-pressed={on} onClick={() => onClasses(on ? classes.filter(x => x !== c) : [...classes, c])}>{classLabel(c)}</button></TermTip>;
        })}
      </div>
      <div className="fa-pick-row2">
        <label className="fa-pick-vi"><input type="checkbox" data-testid="pick-vi" checked={viOnly} onChange={e => onViOnly(e.target.checked)} /> {t('pick.viOnly')}</label>
        <span className="fa-pick-count" data-testid="pick-count" aria-live="polite">{tn('faces.nFonts', { n: formatNumber(shown) })}</span>
        {filtered && <button type="button" className="fa-pick-clear" data-testid="pick-clear" onClick={() => { onClasses([]); onViOnly(false); }}>{t('pick.clear')}{hidden > 0 && ` · ${tn('faces.nFonts', { n: formatNumber(hidden) })}`}</button>}
        {source === 'recent' && clearRecent && <button type="button" className="fa-pick-clear" data-testid="pick-recent-clear" onClick={clearRecent}>{t('pick.recent.clear')}</button>}
      </div>
    </div>
  );
}
