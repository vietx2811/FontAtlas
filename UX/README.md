<div align="center">
<img width="1200" height="475" alt="GHBanner" src="https://github.com/user-attachments/assets/0aa67016-6eaf-458a-adb2-6e31a0763ed6" />
</div>

# Run and deploy your AI Studio app

This contains everything you need to run your app locally.

View your app in AI Studio: https://ai.studio/apps/drive/1GCkhE2yL2c1hFSEXp2bx8jCJRa9hgM8K

## Run Locally

**Prerequisites:**  Node.js


1. Install dependencies:
   `npm install`
2. Set the `GEMINI_API_KEY` in [.env.local](.env.local) to your Gemini API key
3. Run the app:
   `npm run dev`

## Chạy với backend thật hoặc mock

- Trong cửa sổ Tauri (`cargo tauri dev`) giao diện gọi lệnh thật qua `api/tauri.ts` (kiểu từ `bindings/`). Ngoài Tauri (trình duyệt, Vitest) dùng `api/mock.ts`.
- Ép chọn: `VITE_FA_API=mock npm run dev` hoặc `VITE_FA_API=tauri`.
- Dữ liệu tách khỏi máy thật: `FONTATLAS_DATA_DIR=$(mktemp -d) cargo tauri dev` (bản debug).
- Preset cho dev/QA qua hash của `devUrl` (chỉ bản dev): `#addsource=<thư mục>&ob=done` (thêm nguồn, bỏ qua onboarding), `#ob=o1..o7|end` (mở thẳng một bước), `#sel=<họ>:<kiểu>&tab=spec|vi|anat|sim|pair&style=…&info=1`, `#enable=1`. Mock còn có `#model=…&embed=…`.
- Không bấm "Bật kích hoạt font" hoặc kích hoạt font trên máy có font thật khi chưa có đồng ý: engine CoreText là thật.
