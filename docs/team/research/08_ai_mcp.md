# 08 — AI & MCP Integration (nghiên cứu + spike)

Ngày 2026-10-15 · FontAtlas·08 · Ký hiệu: **[CHẠY]** đã chạy thật trên máy · **[NGUỒN]** đọc tài liệu chính chủ · **[NGUỒN-3]** nguồn thứ ba · **[SUY]** suy luận.

## TL;DR khuyến nghị
1. **MCP server = binary Rust riêng `fontatlas-mcp`, transport stdio**, đóng gói kèm app (sidecar Tauri), dùng **rmcp 3.5.1** (SDK Rust chính thức, Apache-2.0, đã hỗ trợ spec 2026-07-28 và tương thích ngược `initialize`). Đọc `library.db`/`user.db` ở chế độ **read-only** (SQLite `mode=ro`), dùng lại crate `fontatlas-core`. Không chạy trong tiến trình Tauri: client (Claude Desktop/Code, LM Studio) tự spawn tiến trình stdio, app không cần đang mở. [SUY + CHẠY spike]
2. **Streamable HTTP localhost: để phase sau, tắt mặc định.** Chỉ cần khi muốn app đang chạy phục vụ trạng thái sống (vd. "font đang chọn"). Nếu bật: bind 127.0.0.1, kiểm Host+Origin, bearer token ngẫu nhiên. [NGUỒN spec]
3. **Phân phối:** `.mcpb` cho Claude Desktop (một click, stdio, offline) + lệnh `claude mcp add` cho Claude Code + đoạn `mcp.json` cho LM Studio/Cursor. ChatGPT **không** kết nối được MCP cục bộ trực tiếp → không ưu tiên.
4. **AI trong app:** mặc định (a) CLIP text encoder cục bộ cho tìm bằng mô tả (chỉ trên font mở có embedding) + bộ mẫu câu cho giải thích; tuỳ chọn (c) API Claude/OpenAI do người dùng nhập key, **chỉ gửi số đo/metadata dạng văn bản**, không bao giờ gửi file font hay ảnh render. (b) LLM cục bộ: không đóng gói, chỉ hỗ trợ qua endpoint OpenAI-compatible người dùng tự chạy (Ollama/LM Studio).

## 1. Kiểm chứng hiện trạng
| Mục | Kết quả | Mức |
|---|---|---|
| Spec MCP hiện hành | **2026-07-28**: bỏ session cấp giao thức, bỏ GET stream, metadata theo từng request (`_meta`), `server/discover`; tương tác server→client qua MRTR. stdio: JSON-RPC theo dòng, stdout chỉ chứa message, log ra stderr. | [NGUỒN] |
| Streamable HTTP bảo mật | Server **MUST** kiểm `Origin` (sai → 403); **SHOULD** bind 127.0.0.1; **SHOULD** có xác thực. Đã có nhiều CVE DNS-rebinding ở SDK (TS < 1.24.0, Ruby 07/2026); rmcp sửa bằng allowlist Host mặc định loopback. | [NGUỒN] + [NGUỒN-3] |
| rmcp | `cargo info rmcp` → **3.5.1**, rust-version 1.88, feature `server`, `macros`, `transport-io`, `transport-streamable-http-server`, `auth`. Macro `#[tool]` sinh `inputSchema`/`outputSchema` từ schemars, hỗ trợ `annotations(read_only_hint…)`. | [CHẠY] |
| Claude Desktop | `.mcpb` (trước là `.dxt`): zip + `manifest.json`, chạy cục bộ qua stdio, cài một click, có thể là binary native. | [NGUỒN-3, trang anthropic.com/engineering/desktop-extensions] |
| Claude Code | `claude mcp add --scope local <name> -- <binary>` (stdio). | [NGUỒN] (chưa chạy — xem §6) |
| ChatGPT | Developer Mode/connectors chỉ nối **server MCP từ xa qua HTTPS công khai**; cục bộ phải qua "Secure MCP Tunnel"/ngrok. Gói hỗ trợ không thống nhất giữa các nguồn (Business/Enterprise đầy đủ; Plus/Pro hạn chế). Trang help OpenAI trả 403 khi fetch. | [NGUỒN-3] |
| LM Studio | MCP host thật, `~/.lmstudio/mcp.json` (định dạng Cursor), stdio + remote. | [NGUỒN lmstudio.ai/docs/app/mcp] |
| Ollama | **Không** phải MCP client; cần cầu nối (ollmcp, MCPHost, Open WebUI). | [NGUỒN-3] |
| llama.cpp | Không tìm thấy xác nhận web UI `llama-server` là MCP client; dùng Open WebUI/CLI cầu nối qua API OpenAI-compatible. | [NGUỒN-3], chưa xác minh repo |
| Giá Claude API (USD/MTok in/out) | Haiku 5.5: 0,10/0,50 (≤100k token) · Sonnet 5.5: 2/10 · Opus 5.5: 4/20. | [NGUỒN platform.claude.com pricing] |

## 2. Kiến trúc MCP đề xuất
```
Claude Desktop / Claude Code / LM Studio ──stdio──► fontatlas-mcp (binary sidecar)
                                                   └─ fontatlas-core (read-only) ─► library.db, user.db (mode=ro)
App Tauri (ghi DB)  ── WAL ──┘     (Phase sau, tuỳ chọn: Streamable HTTP 127.0.0.1 trong app, token)
```
- Lý do binary riêng: vòng đời do client quản lý; app crash không ảnh hưởng; dễ đóng `.mcpb`; quyền tối thiểu (chỉ mở DB read-only). SQLite WAL cho phép đọc song song khi app đang ghi. [SUY]
- Không nạp ONNX/CLIP trong MCP bản đầu: `find_similar` đọc embedding đã tính sẵn từ DB, brute-force như spike 02. Tìm bằng câu mô tả (`search_fonts.description`) cần text encoder → phase 2.
- Cấu hình bật/tắt ở app: mặc định **tắt** truy cập `user.db` (collection/ghi chú có thể nhạy cảm) → bật trong Settings.

## 3. Bộ tool / resource (schema rút gọn)
Nguyên tắc: mọi tool `readOnlyHint=true, openWorldHint=false`; tham chiếu font bằng **`face_id` mờ** (vd. `blake3[:16]:face_index`), **không bao giờ nhận/trả đường dẫn**; mọi output có `structuredContent` + `outputSchema`; giới hạn `limit ≤ 50`; trả `license_class` để client biết font thương mại.

| Tool | Input | Output (chính) |
|---|---|---|
| `search_fonts` | `query?` (tên, có/không dấu), `filters?{classification[], weight_min/max, vietnamese_complete, license_class[], collection_id}`, `limit` | `results[]{face_id, family, style, classification, weight, license_class, vietnamese_complete}`, `total` |
| `get_font_anatomy` | `face_id` | `axes{x_height, contrast, aperture, width, weight, stress_angle, …: {value 0–1, percentile, confidence, estimated: bool}}`, `metrics{upm, cap_height, x_height, ascender, descender}`, `analyzer_version` |
| `find_similar` | `face_id`, `axis_weights?{axis: 0–2}`, `limit`, `same_family?: false` | `results[]{face_id, family, score, similar_on[], differs_on[], method: "embedding+axes" \| "axes_only"}` |
| `suggest_pairings` | `face_id`, `role: "heading"\|"body"`, `mode: "safe"\|"controlled"\|"experimental"`, `require_vietnamese?: true`, `limit` | `pairs[]{face_id, family, axes[]{axis, relation: harmony\|contrast\|clash, delta}, constraints[]{type, ok, note}}` (không có điểm tổng tốt/xấu — theo spec phần 3) |
| `explain_pair` | `heading_face_id`, `body_face_id`, `lang: "vi"\|"en"` | `explanation` (mẫu câu), `axes[]` như trên, `constraints[]` |
| `check_vietnamese_coverage` | `face_id`, `level?: "cmap"\|"shaping"` | `complete`, `missing[]`, `decomposed_ok`, `note` |
| `list_collections` | — | `collections[]{collection_id, name, count}` (chỉ khi bật quyền user.db) |
| `get_specimen_image` | `face_id`, `text`, `size` | **Chỉ cho `license_class=open`**; font thương mại → lỗi `license_restricted`. Đề xuất **không làm ở bản đầu** (xem §5). |

Resources: `fontatlas://face/{face_id}` (JSON anatomy), `fontatlas://collection/{id}`. Prompts: `pairing_brief` (brief → gọi search/suggest/explain).

## 4. AI bên trong app — so sánh
| | (a) CLIP text encoder cục bộ | (b) LLM cục bộ nhỏ | (c) API Claude/OpenAI (key người dùng) |
|---|---|---|---|
| Tìm "chữ ấm, cổ điển cho quán cà phê" | Tốt vừa với tính từ thị giác; yếu với tiếng Việt (CLIP chủ yếu tiếng Anh → cần dịch/từ điển nhãn) | Chuyển câu → bộ lọc trục/nhãn, khá | Tốt nhất: hiểu brief, sinh bộ lọc + lý do |
| Giải thích pairing | Không (dùng mẫu câu) | Được, chất lượng tiếng Việt thất thường với model 3–8B | Tốt, tiếng Việt tự nhiên |
| Offline / riêng tư | Có / tuyệt đối | Có / tuyệt đối | Không / dữ liệu văn bản rời máy |
| Chi phí | Kích thước gói (+~250–600 MB model) [SUY] | 2–5 GB model, RAM/GPU; không nên đóng gói | Rẻ: Haiku 5.5 một lượt ~2k in + 500 out ≈ 0,00045 USD [SUY từ bảng giá] |
| Pháp lý | Chỉ chạy trên embedding font mở (đã quyết) | Chỉ đưa số đo/metadata | Chỉ gửi văn bản số đo + tên; **cấm** gửi file/ảnh render font thương mại |

**Khuyến nghị:** lớp "brief → bộ lọc có cấu trúc" (JSON trục + nhãn) là giao diện chung; bộ thực thi có thể là (a) luật/từ điển + CLIP, hoặc (c) LLM có key. LLM **không bao giờ tự xếp hạng font**, chỉ sinh bộ lọc và diễn giải kết quả do `fontatlas-core` tính → kết quả tái lập được, không bịa font không có. (b) hỗ trợ bằng cách cho người dùng trỏ tới endpoint OpenAI-compatible (Ollama/LM Studio) — cùng code với (c). Hiển thị trước nội dung sẽ gửi đi khi dùng (c).

## 5. Bảo mật & pháp lý
- Read-only mặc định: DB mở `mode=ro`; không có tool ghi (activation, sửa collection) trong bản đầu. Nếu sau thêm tool ghi: `destructiveHint`, xác nhận trong app.
- Không lộ đường dẫn: chỉ `face_id`; server từ chối mọi input không khớp regex id (spike: id lạ → `-32602`). Không có tool đọc file/URL tuỳ ý.
- Prompt injection: tên font/metadata do foundry ghi là dữ liệu không tin cậy → cắt độ dài, không đặt vào `description` của tool.
- stdio: không cần xác thực (client là tiến trình cha cùng user). HTTP (phase sau): 127.0.0.1, kiểm Host+Origin, token ngẫu nhiên lưu Keychain/Credential Manager, tắt mặc định.
- Pháp lý: dữ liệu MCP trả về sẽ đi vào LLM đám mây của client → **mặc định chỉ trả số đo + metadata** (mức rủi ro trung bình–thấp theo spike 01). `get_specimen_image` cho font thương mại = gửi glyph render cho AI → vi phạm EULA kiểu MyFonts/Commercial Type → cấm; với font mở cũng hoãn. Thêm cài đặt "chỉ chia sẻ font giấy phép mở qua MCP". Cần luật sư xác nhận việc gửi số đo font thương mại cho LLM.

## 6. Spike — đã chạy thật
Thư mục `spikes/mcp/` (worktree `eloquent-turing-045cb7`, chưa commit): `Cargo.toml` (rmcp =3.5.1, features `server, macros, transport-io, schemars`), `src/main.rs` (~140 dòng), 2 tool `search_fonts`, `check_vietnamese_coverage` trên 4 font giả.
- [CHẠY] `cargo build --release` thành công lần đầu (19 s, rustc 1.96), binary 3,4 MB.
- [CHẠY] Gửi JSON-RPC thô qua stdin: `initialize` (2025-11-25) OK; `tools/list` trả 2 tool có `annotations{readOnlyHint, openWorldHint}` và `outputSchema`; `tools/call` trả `structuredContent` + text, tiếng Việt UTF-8 đúng; `face_id="../../etc/passwd"` → lỗi `-32602`.
- [CHẠY, 2026-11-05, có sự đồng ý của chủ app] Claude Code 2.1.309: `claude mcp add --scope local fontatlas-spike -- <binary>` → `claude mcp get` báo **✔ Connected**; phiên headless `claude -p --output-format stream-json` liệt kê đủ `mcp__fontatlas-spike__search_fonts` và `mcp__fontatlas-spike__check_vietnamese_coverage`. Sau đó đã `claude mcp remove fontatlas-spike -s local` (xác nhận không còn).
  - Ghi chú môi trường: symlink `~/.local/bin/claude` trên máy đang hỏng (trỏ tới thư mục không tồn tại); đã dùng bản CLI do app Claude Desktop đi kèm.
- **Chưa chạy:** để mô hình gọi tool thật qua Claude Code headless — bị chặn vì phiên OAuth của CLI hết hạn (lỗi xác thực tài khoản, không liên quan server; phần gọi tool đã kiểm bằng JSON-RPC thô ở trên). Chưa thử `server/discover` (spec 2026-07-28) và `.mcpb`.

## 7. Rủi ro
| Rủi ro | Mức | Giảm thiểu |
|---|---|---|
| Spec đổi nhanh (2025-11-25 → 2026-07-28 bỏ session) | TB | Bám rmcp; stdio ít bị ảnh hưởng |
| Rò rỉ số đo font thương mại vào LLM đám mây | TB | Mặc định lọc theo license, cài đặt rõ ràng, hỏi luật sư |
| DNS rebinding nếu bật HTTP | Cao nếu làm sai | Không bật mặc định; Host+Origin+token |
| CLIP yếu với mô tả tiếng Việt | TB | Từ điển nhãn VI→EN; eval với câu thật của chủ app |
| ChatGPT không nối cục bộ | Thấp | Không phải mục tiêu bản đầu |

## Nguồn
- https://modelcontextprotocol.io/specification/latest · …/2026-07-28/basic/transports · …/transports/stdio · …/transports/streamable-http
- https://crates.io/crates/rmcp · https://github.com/modelcontextprotocol/rust-sdk
- https://osv.dev/vulnerability/GHSA-89vp-x53w-74fx (rmcp DNS rebinding) · https://cvepremium.circl.lu/vuln/cve-2025-66414 (TS SDK)
- https://www.anthropic.com/engineering/desktop-extensions · https://claude.com/docs/connectors/building/mcpb
- https://help.openai.com/en/articles/12584461-developer-mode-apps-and-full-mcp-connectors-in-chatgpt-beta (403 khi fetch) · https://peliqan.io/blog/chatgpt-mcp/ · https://designrevision.com/blog/add-mcp-server-to-chatgpt
- https://lmstudio.ai/docs/app/mcp · https://lmstudio.ai/blog/lmstudio-v0.3.17
- https://www.morphllm.com/ollama-mcp · https://pypi.org/project/mcp-client-for-ollama/ · https://docs.openwebui.com/alternatives/llama-cpp.md
- https://platform.claude.com/docs/en/about-claude/pricing
