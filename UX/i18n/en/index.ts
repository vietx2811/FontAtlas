import type { I18nKey } from '..';
import { EN_ACTIVATION } from './activation';
import { EN_CLOUD } from './cloud';
import { EN_COMPARE_V2 } from './compareV2';
import { EN_DESK } from './desk';
import { EN_FILTER, EN_FILTER_2, EN_FILTER_3, EN_FILTER_4 } from './filter';
import { EN_MAIN } from './main';
import { EN_ONBOARDING } from './onboarding';
import { EN_TERMS } from './terms';
import { EN_VIEW } from './view';

/** Bảng tiếng Anh: `Record<I18nKey, string>` nên tsc báo ngay khi thêm khoá vi mà quên khoá en (và ngược lại). */
export const en: Record<I18nKey, string> = {
  ...EN_TERMS, ...EN_COMPARE_V2, ...EN_ACTIVATION, ...EN_CLOUD, ...EN_FILTER, ...EN_FILTER_2, ...EN_FILTER_3, ...EN_FILTER_4, ...EN_DESK, ...EN_ONBOARDING, ...EN_VIEW, ...EN_MAIN,
};
