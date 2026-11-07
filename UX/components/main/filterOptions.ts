import type { I18nKey } from '../../i18n';

/** Khoá license theo backend (fontatlas-core metadata): ofl | apache | other | unknown. */
export const LICENSE_OPTIONS: ReadonlyArray<[string, I18nKey]> = [['ofl', 'filter.license.ofl'], ['apache', 'filter.license.apache'], ['other', 'filter.license.other'], ['unknown', 'filter.license.unknown']];
export const VI_OPTIONS: ReadonlyArray<['full' | 'supported', I18nKey]> = [['full', 'filter.vi.full'], ['supported', 'filter.vi.supported']];
/** Mặt lọc mà view hiện tại đã ép: chip/ô tương ứng bị ẩn hoặc khoá để khớp query thật. */
export const forcedFacets = (view: string): { favorites: boolean; source: boolean } => ({ favorites: view === 'star', source: view.startsWith('src:') });
