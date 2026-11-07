# Round 2 UI — kết quả và cách kiểm

Ngày 2026-11-06; nhánh `feat/round2-ui`, worktree `round2-ui`.

## Thay đổi

- A commit `9d59b0f`: nhiều thư mục bằng native picker `multiple:true`, thêm tuần tự qua API; vẫn có O3 sau legacy import; giữ step/done, cancel không lỗi, duplicate/nested giữ nguồn, list retry/io/busy, khóa thao tác trùng, nguồn current/queued không xóa. Ngưỡng height 760px giảm spacing cho CTA còn trong viewport 1000×700.
- Đã nhập nguyên backend checkpoint `57694d5` bằng merge `f943c9b`; không sửa Rust hoặc generated bindings.
- B: canonical keys, rawValue+unit thật, 21 readings (16 index + terminal/a/g + cap/descender), phân vị Rust 0–1 hiển thị P0–P100, missing/failed/estimated rõ. Số đo không thành từ đánh giá; percentile là thông tin phụ, không phải độ tin cậy. Vi-VN precision do Lead chốt, tooltip giữ số gốc.
- C: `similar_families` qua adapter có kiểu; UI 6 family/trang, offset/total thật, không client-dedupe trang. Mở font → representative/specimen, Xem family → mọi FaceKey của family từ getFamilyStyles (không kèm bộ lọc) + đóng detail; sidebar/tag/search thoát family scope. Cập nhật khi scan version đổi. Family hơn 2.000 kiểu không cắt im lặng: nút Xem family disabled có giải thích, Mở font còn dùng được.
- D: `axis_stats` + `rawAxisRanges` inclusive physical units; metadata/filter được dùng chung; library/count áp thêm membership family/Gần đây/Đang bật, Similar/stats độc lập với membership. Focus P1–P99 với mặc định floor/ceil, input/outlier tới full min/max; biên full làm tròn ra ngoài để giữ endpoint. Empty/constant/loading/error/retry, selected count/reset, Escape/focus. Không sửa ý nghĩa axisFilters cũ.

## Kiểm tra

`npx tsc --noEmit` qua với TypeScript cục bộ trong `UX`; full Vitest 460/460, 49 file; `npm run build` qua. Vẫn có warnings duplicate key trong fixture lưới và mixed dynamic/static import Tauri đã có từ baseline. `git diff --check` qua trước commit. Không chạy activation hoặc thay nguồn native thật.

Vitest mới kiểm serial multi-folder/legacy/cancel/permission/duplicate/busy/queued; raw units/zero/negative/missing/quality/P0/P1/P94; adapter `axis_stats`/`similar_families`/raw range; family exclusion/dedupe/paging/open/scope clear/large family; filter outlier inclusive rounding, missing/constant, metadata/retry/keyboard.

## Visual và browser

Công cụ: Playwright local, Vite :3012, dữ liệu mock. Đã tự mở ảnh bằng view_image sau chụp. Paseo snapshot/click hoạt động nhưng screenshot không paint; gstack thiếu `diff`; CUA host unavailable. Không cài/sửa các công cụ đó.

URL bản cuối: `http://localhost:3012/?qa=round2-final#sel=Public%20Sans:Regular&tab=anat`. Chọn tab Tương tự bằng UI; filter mở nút Lọc, chọn Chiều cao x. Kiểm thực: count từ 91 còn 88 với khoảng 0,463–0,559; Similar từ 15 còn 14 family. Chọn Toàn khoảng 0,462–0,560 giữ đủ 91 font/15 family (ảnh filter cuối). Mở font ra Abril Fatface/specimen, Xem family tiếp theo ra Be Vietnam Pro 18 kiểu và detail đóng; bấm sidebar Tất cả xóa scope. Tab/Shift+Tab ở hai action cho outline solid 2px accent.

Ảnh: `docs/design/screenshots/round2/{onboarding,anatomy,similar,filter}-{1400,1000}-{light,dark}.png` (16 ảnh); kích thước tương ứng 1400×920 và 1000×700. Thêm `family-chip-1000-{light,dark}.png` (tooltip 9/18 khi lọc nghiêng) và `anatomy-stress-1000-dark.png`: 19,8°, Ước lượng, tooltip raw còn đầy đủ. Theme đặt `document.documentElement.dataset.theme` theo case; body không đổi CSS để dàn dựng ảnh. Ảnh onboarding có danh sách cuộn tới hai đường dẫn Việt dài, 5 nguồn; CTA ở y632..666 trong viewport700.

Lỗi visual đã sửa: sidebar trắng/chữ trắng trong dark browser mock; số input float dài tràn ô và dấu phân cách không đồng nhất; anatomy quá nhiều số lẻ/unit lệch dòng; scrollbar dark; panel giữ vị trí cuộn cũ; tên family dài; spacing onboarding thấp.

Console: initial favicon.ico404; sau lần điều hướng cuối không có lỗi app runtime. Một số Vite HMR reload làm mất preset lúc QA, đã mở URL query mới và xác minh panel tồn tại rồi chụp lại toàn bộ ảnh bị ảnh hưởng.

## Dựng onboarding mock (browser evaluate, chỉ QA)

```js
async () => {
  const React = (await import('/node_modules/.vite/deps/react.js')).default;
  const { createRoot } = (await import('/node_modules/.vite/deps/react-dom_client.js')).default;
  const { Onboarding } = await import('/components/onboarding/Onboarding.tsx');
  const { createMockApi } = await import('/api/mock.ts');
  const api = createMockApi();
  await api.setOnboardingStep('o3');
  const host = document.createElement('div');
  host.id = 'round2-onboarding-qa';
  host.style = 'position:fixed;inset:0;z-index:1000';
  document.body.append(host);
  document.getElementById('root').style.display = 'none';
  const root = createRoot(host);
  root.render(React.createElement(Onboarding, {
    api,
    pickFolders: async () => [
      '/Users/me/Thiết kế/Khách hàng Việt/Thư viện font được bổ sung với đường dẫn rất dài',
      '/Volumes/Font/Phông chữ Việt',
    ],
    onDone: () => {},
  }));
  window.round2QA = { api, root, host };
}
```

Bấm “Thêm thư mục” thật trong component; scroll list tới cuối để nhìn đường dẫn dài. Sau ảnh đã bấm Tiếp tục tới bước quét, rồi unmount overlay. Không sửa production để ép onboarding, không dùng mock để chứng minh native picker/IPC.

## Giới hạn bàn giao

Chưa chạy native UI/Tauri end-to-end với thư viện thật, native permission/picker, activation hoặc analyzer validation; Lead nghiệm thu riêng. Backend query implementation có trong checkpoint, adapter test xác nhận payload IPC nhưng browser đang mock. Không chứng nhận tính đúng khoa học mọi trục. Family >2000 giới hạn minh bạch như trên. Filter phiên này không persist sau đóng app. Phần E download tốc độ/pause/resume chưa làm vì hợp đồng mới chưa chuyển; giữ flow cũ, không thêm mirror.


## Follow-up review độc lập (commit mới sau 580c720)

- Similar/stats chỉ nhận metadata + filters + viOnly; không lấy faceKeys của family/recent/active, chọn font không gây stats/Similar refetch chỉ vì recentViews đổi. Selector Zustand giới hạn subscription. Scope đổi reset paging0; response trang rỗng mà total>0 tự lùi về trang hợp lệ.
- Xem family gọi getFamilyStyles không filter, gom mọi FaceKey (named instances chung face chỉ một key), từ chối >2000 minh bạch. Giữ filters/viOnly; chip “Family: X · có lọc” ở hàng riêng, tooltip n/N; bỏ lọc sẽ hiện đầy đủ. API family hiện trả các face đã phân tích, file còn khả dụng; không dựng dữ liệu cho face chưa có trong API.
- Filter giữ stats và mọi AxisRange mounted trong refetch/error, không mất text draft/full mode. Domain slider mở rộng chứa cả cận đã chọn khi chuyển view; blur không chỉnh không gửi lại. Slider kéo/key change là state cục bộ, commit pointerup/keyup/blur, pointercancel bỏ draft. aria-valuetext có unit; trigger chỉ aria-expanded, selected dùng data attribute.
- Input lower floor/upper ceil, checkbox chọn focus floor/ceil; dịch exponent trước rounding để giữ đúng trường hợp 1,001 không hụt do nhân số nhị phân. Trạng thái quét/rỗng/quyền ưu tiên no-match. Onboarding giữ lỗi từng folder khi refresh lỗi.
- Kiểm mới: 460 test/49 file, tsc và build qua. Thêm hồi quy metadata-only/refetch, paging scope round-trip/total shrink, all-family mở khi lọc/bỏ lọc tăng N, chip n/N và clear, draft/stats contraction/refetch, gesture commit, input/focus bounds, decimal1,001, zero priority, batch+refresh errors.
- Browser Playwright tại `http://localhost:3012/?qa=round2-review#sel=Public%20Sans:Regular&tab=sim`: chọn lọc nghiêng, mở family Be Vietnam Pro -> 9/18; chọn font Thin Italic rồi Similar còn 10 family khác, giữ nguyên family scope; thêm x-height còn8/18, stats là43 font trong metadata (không khóa9member); Đặt lại ->18/18 và Similar15 family. Đã xem lại filter4/Similar4, thêm chip1000light/dark2. Sửa toolbar1000px có detail để layout controls cùng hàng tìm kiếm, chip ở hàng riêng. Console phiên cuối không lỗi app.
- Giới hạn giữ nguyên: browser mock không phải native IPC/activation, analyzer chưa được chứng nhận, family>2000 giới hạn minh bạch. Batch thêm folder vẫn chạy tuần tự tới hết, chưa có nút hủy giữa chừng theo phạm vi Lead.
