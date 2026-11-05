// Luật của onboarding (UX 03 §2): ai thấy, bước nào hiện, đi tới/lui đâu. Thuần, không phụ thuộc giao diện.
import type { OnboardingDto } from '../api';

export const STEPS = ['o1', 'o2', 'o3', 'o4', 'o5', 'o6', 'o7', 'end'] as const;
export type Step = typeof STEPS[number];
/** Giá trị `onboarding_step` khi đã xong hoặc đã bỏ qua hướng dẫn (6a lưu null và "" như nhau nên cần dấu riêng). */
export const DONE = 'done';

/**
 * Quy ước (Lead duyệt 2026-11-06; 6a lưu `onboarding_step` null và "" như nhau nên không phân biệt được "chưa bắt đầu" với "xong"):
 * - lần đầu = step null và chưa xác nhận giấy phép (`licenseNoticeAck`). Người đã có nguồn từ DB cũ vẫn phải thấy giấy phép, model, kích hoạt;
 * - đang dở = step khác null và khác "done"; xong hoặc bỏ qua hướng dẫn thì ghi "done" (không hiện lại, giấy phép nhắc ở lần quét đầu/Similar);
 * - bước "Thư mục font" (O3) bị bỏ chỉ khi nguồn đã đến từ import dữ liệu cũ.
 * Nếu sau này cần phân biệt rõ hơn, xin 02 một trường riêng trong OnboardingDto.
 */
export const isFirstRun = (s: OnboardingDto): boolean => s.step === null && !s.licenseNoticeAck;
export const needsOnboarding = (s: OnboardingDto): boolean => (s.step !== null && s.step !== DONE) || isFirstRun(s);
export const startStep = (s: OnboardingDto): Step => (STEPS as readonly string[]).includes(s.step ?? '') ? (s.step as Step) : 'o1';

export function visibleSteps(s: OnboardingDto): Step[] {
  const legacyPending = s.legacy.found && !s.legacy.imported && !s.legacy.resolved;
  const sourcesFromLegacy = !!s.legacy.imported && s.hasSources;
  return STEPS.filter(step => (step === 'o2' ? legacyPending : step === 'o3' ? !sourcesFromLegacy : step === 'o4' ? s.hasSources : true));
}
export function nextStep(cur: Step, s: OnboardingDto): Step | null {
  const list = STEPS.filter(x => x === cur || visibleSteps(s).includes(x));
  return list[list.indexOf(cur) + 1] ?? null;
}
export function prevStep(cur: Step, s: OnboardingDto): Step | null {
  const list = STEPS.filter(x => x === cur || visibleSteps(s).includes(x));
  return list[list.indexOf(cur) - 1] ?? null;
}

/** Ước lượng thời gian phân tích hình ảnh: 28 ms/face (Metal, đo ở Plan 4b), làm tròn lên theo phút. */
export const estimateAnalysisMinutes = (faces: number): number => Math.ceil((faces * 28) / 60_000);
