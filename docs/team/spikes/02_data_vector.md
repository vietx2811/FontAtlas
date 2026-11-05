# Spike 02 — Data & Vector: thay Qdrant bằng SQLite + chỉ mục trong bộ nhớ

Người làm: Data & Vector · Ngày: 2026-10-15 · Trạng thái: SPIKE (code bỏ đi, không đụng `UX/src-tauri`)
Code đo: worktree `gracious-euler-ca23d9`, thư mục `spikes/data-vector/` (`src/main.rs`, `run_all.sh`, `schema/*.sql`, log ở `data/*.log` — chưa commit).

Ký hiệu: **[ĐO]** = số đo thật trên máy dưới đây · **[SUY]** = suy luận/ước lượng, chưa đo.

---

## TL;DR — khuyến nghị

1. **Chỉ mục vector = brute-force trong bộ nhớ, int8 + xếp lại bằng f32 đọc từ SQLite.** Chính xác tuyệt đối (recall@200 = 1,000), 1,2–3,5 ms ở 10k–200k font, RAM chỉ = n × 512 B (8 / 28 / 101 MB). Không cần crate ANN. [ĐO]
   - Phương án đơn giản hơn nếu thư viện ≤ ~50k: giữ f32 nguyên trong RAM (0,7 ms, 104 MB @50k). [ĐO]
2. **Không dùng sqlite-vec làm chỉ mục chính**: `vec0` cũng là quét tuần tự, đơn luồng — chậm hơn brute-force Rust song song ~45 lần (135 ms vs 3 ms @200k). Bài học giữ lại: nó đóng gói vector gọn hơn (xem mục 6). [ĐO]
3. **Không dùng HNSW (usearch / hnsw_rs)** ở quy mô này: với k=200, recall thấp trên dữ liệu ngẫu nhiên và lệch mạnh khi lọc; xây chỉ mục mất hàng chục giây @200k; tốn RAM hơn brute-force int8. [ĐO]
4. **Tách 2 file SQLite**: `library.db` (dẫn xuất, xoá–dựng lại được, `page_size=16384`) và `user.db` (người dùng, nhỏ, cần backup). Bảng người dùng tham chiếu font bằng `(hash BLAKE3, face_index)`, không FK sang dẫn xuất → rescan/rebuild không bao giờ chạm dữ liệu người dùng. DDL đã chạy thử. [ĐO cho DDL; thiết kế là đề xuất]
5. **Tìm tên tiếng Việt**: FTS5 `trigram remove_diacritics 1` trên cột `search_text` **đã gập dấu ở Rust** (`vi_fold`: NFD → bỏ dấu → `đ→d` → chữ thường). FTS5 tự bỏ được mọi dấu thanh, ơ/ư/â/ă/ê/ô **nhưng không gập `đ`** ("dep" không ra "Đẹp"). [ĐO]
6. **`page_size=16384` cho library.db**: BLOB ~2 KB/hàng với trang 4 KB lãng phí ~50% (827 MB → 489 MB @200k). [ĐO]

---

## 1. Benchmark

### 1.1 Cấu hình

| Mục | Giá trị |
|---|---|
| Máy | Apple M4 Pro, 12 nhân CPU, 24 GB RAM, macOS 27.2 (26B5101f); dữ liệu nằm trên ổ ngoài `/Volumes/VXData` |
| Toolchain | rustc 1.96.0, `--release`, `lto=thin`, `codegen-units=1`, không bật `target-cpu=native` |
| Crate | rusqlite 0.31 bundled (= **SQLite 3.45.0**, giống app hiện tại), sqlite-vec 0.1.9, usearch 2.26.4, hnsw_rs 0.3.5, rayon 1 |
| Dữ liệu | 512 chiều f32 chuẩn hoá L2 (cosine = dot) + 16 số đo f32 + cờ `supports_vi` (~35% true). Hai kiểu: **uniform** (Gauss ngẫu nhiên — đúng đề bài, ca xấu nhất cho ANN) và **clustered** (1000 tâm + nhiễu — gần dữ liệu CLIP thật hơn). Seed cố định. |
| Truy vấn | 50 truy vấn = vector của chính 50 font trong thư viện ("tìm font giống font này"), top-200, cosine. Ground truth = quét f32 chính xác. Lọc kết hợp = chỉ font `supports_vi=1`. |
| Đo | Mỗi phương pháp chạy trong **tiến trình riêng**. RAM = `phys_footprint` (`proc_pid_rusage`) chênh lệch trước/sau nạp. "Lượt 1/lượt 2" = chạy lại để thấy ảnh hưởng page cache (không có quyền `sudo purge`, nên lượt 1 **không phải** cold cache thật). |
| Nhiễu | Máy đang chịu áp lực bộ nhớ trong lúc đo (swap đã dùng 4 GB/5 GB, compressor ~8 GB) và có app khác chạy → p95 dao động; tin p50 hơn. |

### 1.2 Kết quả chính — truy vấn top-200, p50 (ms) [ĐO]

Số của **uniform**; clustered cho kết quả gần như giống hệt với mọi phương pháp quét toàn bộ (khác biệt chỉ đáng kể ở HNSW — mục 1.4).

| Phương pháp | 10k | 50k | 200k | 200k lọc VI | recall@200 | RAM thêm (10k / 50k / 200k) |
|---|---|---|---|---|---|---|
| f32 brute-force, rayon 12 luồng | 0,20 | 0,73 | 3,0 | 3,0 | 1,000 | 23 / 104 / 406 MB |
| f32 brute-force, 1 luồng | 0,31 | 1,7 | 7,4 | 9,7 | 1,000 | như trên |
| int8 brute-force, 12 luồng | 0,17 | 0,41 | 1,4 | 1,8 | 0,90–0,93 (uniform) · 0,93–0,995 (clustered) | 8 / 28 / 101 MB |
| **int8 → 400 ứng viên → xếp lại f32 đọc từ SQLite** | 1,2 | 1,8 | 3,4 | 3,3 | **1,000** | **8 / 28 / 101 MB** |
| sqlite-vec `vec0` KNN (`k=200`, cột metadata `supports_vi`) | 6,7 | 33 | 135 | 117 | 1,000 | ~7 MB (page cache SQLite) |
| sqlite-vec hàm `vec_distance_cosine` trên bảng BLOB thường | 11 | 56–60 | 250 | 165 | 1,000 | ~3 MB |
| usearch HNSW f32 (M=16, ef_search=256) | 1,7 | 2,4 | 3,0 | 8,0 | xem 1.4 | 46 / 213 / 840 MB* |
| usearch HNSW i8 (M=16, ef_search=256) | 0,23 | 0,46 | 0,83 | 2,2 | xem 1.4 | 31 / 140 / 546 MB* |

\* RAM HNSW đo sau khi xây xong và đã drop mảng f32 — allocator macOS chưa trả bộ nhớ ngay nên số này phồng; `memory_usage()` do usearch tự báo: 32 / 144 / 562 MB (f32), 16 / 80 / 178 MB (i8). Cả hai đều lớn hơn brute-force int8.

Số đo phụ [ĐO]:
- **16 số đo giải phẫu**: xếp lại 200 ứng viên theo khoảng cách có trọng số < 0,2 ms; quét toàn bộ 200k × 16 chiều lấy top-200 ≈ 0,9–1,5 ms. → Pairing/Similar theo trục có thể quét toàn bộ, không cần chỉ mục.
- **Lọc kết hợp** với brute-force: kiểm tra mặt nạ `Vec<bool>` ngay trong vòng quét, chính xác 100%; chi phí ~0 khi song song, +30% khi 1 luồng (rẽ nhánh).

### 1.3 Nạp lúc khởi động, ghi, kích thước file [ĐO]

| | 10k | 50k | 200k |
|---|---|---|---|
| Nạp f32 từ SQLite vào `Vec` liền khối (lượt 2, cache ấm) | 0,012 s | 0,055 s | 0,22–0,24 s |
| Nạp f32, lượt 1 (cache có thể đã bị đẩy ra) | 0,15 s | 0,57 s | 2,1–2,4 s |
| Nạp + lượng tử int8 ngay khi đọc (cache ấm) | 0,009 s | 0,043 s | 0,17 s |
| File DB chuẩn (faces + analyses + embeddings BLOB), page 4 KB | 41 MB | 207 MB | 827 MB |
| File `vec0` (chỉ vector + 1 metadata) | 20 MB | 100 MB | 400 MB |
| Ghi DB chuẩn, 1 transaction, 3 bảng | 0,2 s | 1,2–1,4 s | 4,7–5,7 s |
| Ghi `vec0` | 0,3 s | 1,2 s | 4,6–5,3 s |
| usearch: xây f32 / i8 (12 luồng, ef_add=128) | 1,0–1,4 / 0,15–0,19 s | 7,6–15 / 1,2–1,9 s | 29–77 / 5,8–10 s |
| usearch: file lưu / nạp lại (f32) | 21 MB / 5 ms | 105 MB / 24–97 ms | 419 MB / 114–180 ms |

`page_size` của DB chuẩn @200k (cùng dữ liệu): 4 KB = 827 MB · 8 KB = 565 MB · **16 KB = 489 MB** · 64 KB = 446 MB (dữ liệu thô 410 MB). Thời gian nạp cache ấm không đổi đáng kể (0,13–0,18 s). Nguyên nhân: hàng ~2,1 KB → trang 4 KB chỉ chứa 1 hàng.

### 1.4 HNSW nói riêng [ĐO]

Tham số: M=16 cho cả hai; usearch `ef_add/ef_search` = 128/256 hoặc 256/1024 ("ef1k"); hnsw_rs `ef_c=200`, `ef_search=256`, `DistCosine`. Lọc VI: usearch `filtered_search(closure)`, hnsw_rs `search_filter`. Ô ghi: **p50 ms / recall@200**.

| Biến thể | Dữ liệu | 10k | 50k | 200k | 200k lọc VI | Xây @200k |
|---|---|---|---|---|---|---|
| usearch f32 ef256 | uniform | 1,7 / 0,72 | 2,4 / 0,34 | 3,0 / **0,15** | 8,0 / 0,23 | 77 s |
| usearch f32 ef256 | clustered | 1,4 / 0,85 | 1,7 / 0,79 | 1,0 / 0,99 | 3,6 / 0,78 | 29 s |
| usearch f32 ef1k | uniform | 3,5 / 0,98 | 7,9 / 0,72 | 12,5 / 0,35 | 26 / 0,59 | 113 s |
| usearch f32 ef1k | clustered | 3,2 / 0,99 | 6,4 / 0,95 | 6,4 / 0,998 | 17 / 0,95 | 61 s |
| usearch i8 ef256 | uniform | 0,23 / 0,71 | 0,46 / 0,35 | 0,83 / 0,15 | 2,2 / 0,23 | 10 s |
| usearch i8 ef256 | clustered | 0,19 / 0,82 | 0,37 / 0,77 | 0,31 / 0,98 | 0,95 / 0,77 | 5,8 s |
| usearch i8 ef1k | clustered | 0,61 / 0,89 | 1,8 / 0,88 | 2,4 / 0,99 | 5,9 / 0,89 | 14 s |
| hnsw_rs f32 | uniform | 2,8 / 0,81 | 4,1 / 0,43 | 5,2 / 0,21 | 59 / 0,66 | 135 s |
| hnsw_rs f32 | clustered | 2,7 / 0,91 | 3,2 / 0,85 | 1,9 / 0,995 | 20 / 0,90 | 83 s |
| *đối chứng: int8 + rerank f32* | cả hai | *1,2 / 1,0* | *1,8 / 1,0* | *3,4 / 1,0* | *3,3 / 1,0* | *0 (nạp 0,17 s)* |

Đọc kết quả:
- Với **k=200** (lớn so với thông lệ k=10), HNSW cần ef rất lớn; trên dữ liệu uniform (không cấu trúc) recall sụp đổ khi n tăng. Clustered @200k dễ vì mỗi cụm ~200 điểm ≈ đúng k — đừng coi đó là bằng chứng HNSW đủ tốt với CLIP thật.
- **Lọc làm HNSW vừa chậm hơn vừa sai hơn** (hnsw_rs lọc VI @200k: 20–59 ms); brute-force lọc thì không tốn gì thêm và luôn đúng.
- Chỉ HNSW i8 trên dữ liệu clustered, không lọc, mới thắng rõ về tốc độ (0,3 ms vs 1,4 ms int8 brute-force) — chênh lệch người dùng không cảm nhận được, đổi lại phải xây/lưu/đồng bộ chỉ mục và chấp nhận sai.
- hnsw_rs: RAM f32 + đồ thị ~1,5–1,6 GB @200k, không có lượng tử; usearch cần toolchain C++ (cxx) khi build — thêm rủi ro build Windows.

### 1.5 Bẫy đo lường gặp phải (ghi lại để đội khỏi vấp) [ĐO]
- Crate `memory-stats` báo sai trên macOS này (146 MB cho mảng 400 MB). Đo bằng `proc_pid_rusage(...).ri_phys_footprint`.
- `Vec` lớn tăng dần bằng `extend` (realloc nhân đôi) làm footprint lệch → **luôn `Vec::with_capacity(count * 512)`** từ `SELECT count(*)` trước khi nạp (cũng là cách nên làm trong sản phẩm).
- Bộ nhớ đã `drop` (mảng f32 sau khi lượng tử) không trả ngay về OS → **lượng tử int8 ngay khi stream từng hàng**, không bao giờ dựng mảng f32 đầy đủ.
- `hnsw_rs::DistDot` panic (`assert dot >= 0`) do sai số float với vector đã chuẩn hoá → phải dùng `DistCosine`.

---

## 2. Khuyến nghị kiến trúc vector

```
library.db.embeddings (f32 BLOB — nguồn sự thật)
        │ stream lúc khởi động (luồng nền, sau khi UI hiện)
        ▼
VectorIndex { model_version, ids: Vec<u32 face_id>, q8: Vec<i8> (n×512), live: BitVec }
        │ query(q, k=200, filter: &BitVec)
        ├─ quét int8 song song (rayon) + mặt nạ lọc  → top 2k ứng viên
        └─ đọc 2k vector f32 từ SQLite (prepare_cached, theo rowid) → dot chính xác → top k
        ▼
xếp lại theo 16 trục giải phẫu, trọng số người dùng (dữ liệu trong RAM, < 0,2 ms) → "Giống ở / Khác ở"
```

- Mặt nạ lọc (`supports_vi`, nguồn, đang kích hoạt, bộ sưu tập…) dựng từ SQL `SELECT face_id … WHERE …` thành bitset; cache theo bộ lọc. [SUY: dựng bitset 200k < vài ms]
- Cập nhật tăng dần: thêm font → `push` cuối mảng; xoá → tắt bit `live`; compact khi > 10% đã chết. Không có chi phí rebuild như HNSW. [SUY]
- Đổi model (`model_version` mới): dựng chỉ mục thứ hai song song trong lúc embedding nền chạy; chỉ so vector cùng `model_version`.
- `target-cpu`/NEON `sdot` có thể nhanh hơn nữa — chưa thử, không cần.
- Ngưỡng xem lại quyết định: > ~1 triệu face, hoặc cần < 1 ms trên 1 luồng → khi đó mới cân nhắc HNSW int8 (ef lớn) cho truy vấn không lọc và vẫn brute-force cho truy vấn có lọc. [SUY]

---

## 3. Schema SQLite đề xuất

File đầy đủ có chú thích: `spikes/data-vector/schema/library.sql`, `schema/user.sql`. Đã chạy bằng sqlite3 CLI 3.54 (chỉ dùng tính năng có trong 3.45). Đã kiểm thử: tạo bảng, trigger FTS (insert/update/delete), `ATTACH` + join favorites, `ON DELETE CASCADE` (chỉ khi bật `foreign_keys` cho kết nối), CHECK độ dài vector, ràng buộc `source='community' ⇔ snapshot_id`. [ĐO]

### 3.1 Nguyên tắc
- **Hai file**: `library.db` (dẫn xuất: files, file_locations, faces, faces_fts, analyses, embeddings, community_snapshots, pipeline_state) và `user.db` (sources, favorites, collections, collection_items, tags, face_tags, activation_state, legacy_ids, settings), mở chung bằng `ATTACH`. Lý do: rescan/rebuild không thể ghi đè dữ liệu người dùng (vật lý, không chỉ quy ước); "Xoá cache phân tích" = xoá file; backup/export dữ liệu người dùng chỉ vài trăm KB; page_size riêng cho file chứa BLOB lớn.
  - Giá phải trả: không có FK giữa hai file; transaction chạm cả hai file ở chế độ WAL không nguyên tử toàn cục (mỗi file nguyên tử riêng). Chấp nhận được vì thao tác người dùng và thao tác pipeline không cần chung transaction.
  - Nếu Lead muốn 1 file: giữ nguyên DDL, bỏ `ATTACH`, vẫn không đặt FK từ bảng người dùng sang bảng dẫn xuất.
- **Danh tính** = `(hash BLAKE3 32 byte, face_index)`. `face_id INTEGER` chỉ là khoá nội bộ của library.db (gọn cho chỉ mục/FTS/bộ nhớ); bảng người dùng và dữ liệu cộng đồng dùng `(hash, face_index)`.
- `analyses`/`embeddings` khoá `(hash, face_index, version, source)` chứ không theo `face_id` → snapshot cộng đồng có thể chứa hash mà máy chưa có; khi người dùng thêm đúng file đó thì tự khớp. Đọc: ưu tiên `local` > `community` > `legacy`.
- **PRAGMA mỗi lần mở kết nối** (đã thấy: `foreign_keys` là theo kết nối, quên là cascade không chạy): `journal_mode=WAL` (lưu trong file), `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`. `page_size=16384` chỉ đặt lúc tạo library.db (trước bảng đầu tiên, trước khi bật WAL); đổi sau phải `VACUUM` ngoài WAL.
- **Migration**: `PRAGMA user_version` cho từng file; mảng migration tuần tự, mỗi bước trong `BEGIN IMMEDIATE … COMMIT` kèm lệnh tăng `user_version`; `VACUUM INTO` backup trước bước phá huỷ. library.db có lối thoát: migration lỗi hoặc version quá cũ → dựng lại (dữ liệu dẫn xuất), nhưng cố giữ `embeddings` nếu `model_version` còn khớp (embedding là phần đắt nhất để tính lại).

### 3.2 DDL (rút gọn — bản đầy đủ trong các file .sql)

```sql
-- ===== library.db =====
PRAGMA page_size=16384; PRAGMA journal_mode=WAL;
CREATE TABLE files (file_id INTEGER PRIMARY KEY, hash BLOB NOT NULL UNIQUE CHECK(length(hash)=32),
  size INTEGER NOT NULL, format TEXT NOT NULL, num_faces INTEGER NOT NULL DEFAULT 1, first_seen_at INTEGER NOT NULL);
CREATE TABLE file_locations (location_id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL REFERENCES files ON DELETE CASCADE,
  source_id INTEGER NOT NULL,                 -- user.db.sources (khác file, không FK)
  path TEXT NOT NULL UNIQUE, size INTEGER NOT NULL, mtime_ns INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL, missing INTEGER NOT NULL DEFAULT 0);
CREATE TABLE faces (face_id INTEGER PRIMARY KEY, file_id INTEGER NOT NULL REFERENCES files ON DELETE CASCADE,
  face_index INTEGER NOT NULL, meta_version INTEGER NOT NULL, family_name TEXT NOT NULL, subfamily TEXT,
  typo_family TEXT, full_name TEXT, postscript_name TEXT, foundry TEXT, weight_class INTEGER, width_class INTEGER,
  is_italic INTEGER NOT NULL DEFAULT 0, is_variable INTEGER NOT NULL DEFAULT 0, var_axes TEXT,
  supports_vi INTEGER NOT NULL DEFAULT 0, glyph_count INTEGER,
  family_key TEXT NOT NULL,                   -- vi_fold(typo_family ?? family_name)
  search_text TEXT NOT NULL,                  -- vi_fold(family + subfamily + full + ps + foundry)
  UNIQUE(file_id, face_index));
CREATE VIRTUAL TABLE faces_fts USING fts5(search_text, content='faces', content_rowid='face_id',
  tokenize='trigram remove_diacritics 1');   -- + 3 trigger AFTER INSERT/DELETE/UPDATE OF search_text
CREATE TABLE community_snapshots (snapshot_id INTEGER PRIMARY KEY, name TEXT NOT NULL, version TEXT NOT NULL,
  format_version INTEGER NOT NULL, analyzer_version TEXT, model_version TEXT, sha256 BLOB NOT NULL,
  imported_at INTEGER NOT NULL, UNIQUE(name, version));
CREATE TABLE analyses (hash BLOB NOT NULL, face_index INTEGER NOT NULL, analyzer_version TEXT NOT NULL,
  source TEXT NOT NULL CHECK(source IN ('local','community','legacy')),
  snapshot_id INTEGER REFERENCES community_snapshots ON DELETE CASCADE,
  axes BLOB NOT NULL,                         -- f32[n] chuẩn hoá 0–1
  axes_conf BLOB,                             -- f32[n] độ tin cậy
  axes_schema INTEGER NOT NULL, extra TEXT, analyzed_at INTEGER NOT NULL,
  PRIMARY KEY(hash, face_index, analyzer_version, source),
  CHECK((source='community') = (snapshot_id IS NOT NULL))) WITHOUT ROWID;
CREATE TABLE embeddings (hash BLOB NOT NULL, face_index INTEGER NOT NULL, model_version TEXT NOT NULL,
  source TEXT NOT NULL CHECK(source IN ('local','community','legacy')),
  snapshot_id INTEGER REFERENCES community_snapshots ON DELETE CASCADE,
  dim INTEGER NOT NULL, dtype TEXT NOT NULL CHECK(dtype IN ('f32','f16','i8')), vec BLOB NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY(hash, face_index, model_version, source),
  CHECK((source='community') = (snapshot_id IS NOT NULL)),
  CHECK(length(vec) = dim * CASE dtype WHEN 'f32' THEN 4 WHEN 'f16' THEN 2 ELSE 1 END));
  -- rowid table (hàng lớn ~2 KB → không dùng WITHOUT ROWID)
CREATE TABLE pipeline_state (face_id INTEGER NOT NULL REFERENCES faces ON DELETE CASCADE,
  stage TEXT NOT NULL CHECK(stage IN ('meta','analysis','embedding')),
  status TEXT NOT NULL CHECK(status IN ('pending','running','done','failed','skipped')),
  version TEXT NOT NULL, error_code TEXT, error_detail TEXT, attempts INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL, PRIMARY KEY(face_id, stage)) WITHOUT ROWID;
CREATE INDEX file_locations_file   ON file_locations(file_id);
CREATE INDEX file_locations_source ON file_locations(source_id, missing);
CREATE INDEX faces_family          ON faces(family_key, weight_class, is_italic);
CREATE INDEX faces_vi              ON faces(supports_vi) WHERE supports_vi = 1;
CREATE INDEX embeddings_model      ON embeddings(model_version, source);
CREATE INDEX pipeline_todo         ON pipeline_state(stage, status) WHERE status IN ('pending','failed');
PRAGMA user_version = 1;

-- ===== user.db =====
CREATE TABLE sources (source_id INTEGER PRIMARY KEY, legacy_id TEXT UNIQUE,
  kind TEXT NOT NULL CHECK(kind IN ('directory','system','google_fonts','adobe_fonts','community')),
  name TEXT NOT NULL, path TEXT, enabled INTEGER NOT NULL DEFAULT 1, watch INTEGER NOT NULL DEFAULT 1,
  metadata TEXT, created_at INTEGER NOT NULL, UNIQUE(kind, path));
CREATE TABLE favorites (hash BLOB NOT NULL CHECK(length(hash)=32), face_index INTEGER NOT NULL,
  created_at INTEGER NOT NULL, PRIMARY KEY(hash, face_index)) WITHOUT ROWID;
CREATE TABLE collections (collection_id INTEGER PRIMARY KEY, legacy_id TEXT UNIQUE, name TEXT NOT NULL,
  color TEXT, position INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE collection_items (collection_id INTEGER NOT NULL REFERENCES collections ON DELETE CASCADE,
  hash BLOB NOT NULL, face_index INTEGER NOT NULL, position INTEGER NOT NULL DEFAULT 0, added_at INTEGER NOT NULL,
  PRIMARY KEY(collection_id, hash, face_index)) WITHOUT ROWID;
CREATE TABLE tags (tag_id INTEGER PRIMARY KEY, name TEXT NOT NULL,
  name_key TEXT NOT NULL UNIQUE,              -- vi_fold(name): 'Cổ điển' == 'co dien'
  color TEXT, created_at INTEGER NOT NULL);
CREATE TABLE face_tags (tag_id INTEGER NOT NULL REFERENCES tags ON DELETE CASCADE, hash BLOB NOT NULL,
  face_index INTEGER NOT NULL, added_at INTEGER NOT NULL, PRIMARY KEY(tag_id, hash, face_index)) WITHOUT ROWID;
CREATE TABLE activation_state (hash BLOB PRIMARY KEY CHECK(length(hash)=32),  -- theo FILE: OS kích hoạt file
  state TEXT NOT NULL CHECK(state IN ('active','inactive')),
  scope TEXT NOT NULL CHECK(scope IN ('session','persistent')),
  method TEXT NOT NULL, active_path TEXT, updated_at INTEGER NOT NULL) WITHOUT ROWID;
CREATE TABLE legacy_ids (legacy_id TEXT PRIMARY KEY, hash BLOB, face_index INTEGER,  -- hash NULL = chưa nối lại
  legacy_path TEXT NOT NULL, payload TEXT);   -- payload = JSON hàng fonts cũ
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE INDEX collection_items_face ON collection_items(hash, face_index);
CREATE INDEX face_tags_face        ON face_tags(hash, face_index);
CREATE INDEX legacy_ids_unresolved ON legacy_ids(legacy_path) WHERE hash IS NULL;
PRAGMA user_version = 1;
```

Chỗ cho cộng đồng: `community_snapshots` + cột `source`/`snapshot_id` ở `analyses`/`embeddings` + `sources.kind='community'`. Snapshot chỉ chứa dữ liệu dẫn xuất khoá theo hash (đúng hướng B: không bao giờ gửi file font). Xoá snapshot → `ON DELETE CASCADE` dọn sạch dữ liệu của nó.

### 3.3 Tìm tên tiếng Việt [ĐO trên SQLite 3.45.0 bundled]

8 tên có dấu, 13 truy vấn:

| Truy vấn | `unicode61 remove_diacritics 2` | `trigram remove_diacritics 1` |
|---|---|---|
| `ĐẸP`, `đep` → Đẹp Sans | ✔ | ✔ |
| `dep` → Đẹp Sans | ✘ (`đ` là chữ riêng, không phải dấu) | ✘ |
| `pho`, `PHỞ`, `ha noi`, `quoc ngu`, `ngư`, `duong`, `dương` | ✔ | ✔ |
| chuỗi con giữa từ `noi gro`, `ài g` | ✘ (chỉ khớp nguyên token/tiền tố) | ✔ |

Đề xuất:
- Rust `vi_fold(s)`: NFD (crate `unicode-normalization`) → bỏ ký tự combining → `đ/Đ → d` → `to_lowercase` → gộp khoảng trắng. Dùng cho `faces.search_text`, `faces.family_key`, `tags.name_key` và cho chuỗi truy vấn. Không đăng ký thành hàm SQL (trigger chạy từ công cụ ngoài app sẽ hỏng) — tính ở Rust rồi lưu cột.
- FTS5 `trigram` cho tìm chuỗi con ("gro" → "Grotesk"); `remove_diacritics 1` làm lưới an toàn. Truy vấn < 3 ký tự: trigram không khớp → fallback `search_text LIKE ? || '%'` hoặc theo `family_key` ([SUY] quét 200k chuỗi ngắn vài ms).
- Bọc truy vấn trong `"..."` và loại `"` để người dùng không gõ được cú pháp FTS.

---

## 4. Kế hoạch di chuyển từ schema cũ (`fontatlas.db`, bảng `fonts` 35 cột)

Hiện trạng cần lưu ý (theo `02_rust_backend.md`): `id` là UUID v4 sinh lại mỗi lần add; bảng `collections` có thể **không tồn tại** (B1); quan hệ 1 font – 1 collection (`collection_id`); `tags` là JSON text; metrics do Python tính, không có version; Qdrant giữ vector `perceptual` 512D (OpenCLIP ViT-B-32/openai, `sidecar.py:220`) và `metrics` 9D, point id = `font_id` hoặc `uuid5(file_path)`.

Các bước (chạy một lần, có màn hình tiến độ, chạy lại được nếu bị ngắt — mỗi bước idempotent):

1. **Sao lưu**: `VACUUM INTO 'fontatlas.v0-backup.db'`. Không sửa file cũ trong suốt quá trình.
2. **Tạo `user.db` + `library.db`** theo DDL v1.
3. **Nguồn**: `sources` cũ → `user.sources` (giữ `legacy_id`). Font có `source_id` NULL/mồ côi → gom vào nguồn `directory` theo `original_directory`.
4. **Nhận diện file**: mỗi `fonts.file_path` còn tồn tại → BLAKE3 → `files`, `file_locations`; đọc metadata bằng bộ đọc Rust mới → `faces` (mọi face của TTC). Khớp hàng cũ ↔ face: file 1 face → `face_index=0`; TTC → khớp theo `family_name` + `subfamily`, không khớp thì gán face 0 và ghi cảnh báo. Song song hoá theo file. [SUY: bị giới hạn bởi tốc độ đọc ổ; chưa đo]
5. **`legacy_ids` cho MỌI hàng cũ** (kể cả file đã mất): `legacy_id = fonts.id`, `hash/face_index` (NULL nếu file không còn), `legacy_path`, `payload` = JSON toàn bộ hàng cũ. → Không mất dữ liệu người dùng dù file biến mất; khi lần quét sau thấy file ở `legacy_path` (hoặc cùng hash) thì nối lại và áp dữ liệu người dùng từ `payload`.
6. **Dữ liệu người dùng** (cho hàng đã có hash; hàng chưa có hash được áp khi nối lại ở bước 5):
   - `is_favorite=1` → `favorites`.
   - `collections` cũ (nếu bảng tồn tại) → `user.collections` giữ `legacy_id`; `collection_id` trỏ tới collection không tồn tại → tạo collection "Đã khôi phục (n)" thay vì bỏ.
   - `fonts.collection_id` → `collection_items`.
   - `tags` JSON → `tags` (dedupe theo `vi_fold`) + `face_tags`. JSON hỏng → ghi log, vẫn còn trong `payload`.
7. **Dữ liệu dẫn xuất cũ** (tuỳ chọn, chỉ để có kết quả ngay khi lõi mới chưa chạy xong):
   - 8 metric + phân loại (`category`, `classification`, `primary_class`, `*_subtype`, `secondary_tags`) → `analyses` với `analyzer_version='legacy-py'`, `source='legacy'`, giá trị trong `extra` JSON; UI ghi "ước lượng cũ". Bị thay thế khi analyzer Rust chạy xong.
   - Vector Qdrant: nếu Qdrant đang chạy lúc migrate → `scroll` toàn bộ, ghép point id ↔ `legacy_id` / `uuid5(path)` → `embeddings` với `model_version='openclip-vitb32-openai@legacy-render'`, `source='legacy'`. Chỉ tái dùng được nếu pipeline ONNX mới dùng **cùng model và cùng cách render ảnh mẫu** — nếu không, coi là model khác và để pipeline mới tính lại. Không có Qdrant → bỏ qua.
   - Bỏ: `is_synced` (→ `pipeline_state`), `file_exists` (→ `file_locations.missing`), `date_modified`, `id` UUID (→ `legacy_ids`).
8. **Kiểm tra**: số favorite / số cặp collection–font / số tag–font cũ vs mới (tính cả phần còn treo trong `legacy_ids`) phải khớp → mới đặt `settings.migrated_from_v0` và đổi tên file cũ thành `fontatlas.v0.db` (giữ ít nhất 2 phiên bản app).
9. **Chuyển tiếp API**: command nhận cả `legacy_id` (tra `legacy_ids`) lẫn id mới trong 1–2 phiên bản để frontend đổi dần.

---

## 5. Rủi ro

| Rủi ro | Mức | Ghi chú / giảm thiểu |
|---|---|---|
| Số đo trên dữ liệu tổng hợp; vector CLIP thật có cấu trúc khác | Vừa | Brute-force không phụ thuộc phân bố (thời gian tuyến tính, recall=1). Recall int8 thô phụ thuộc phân bố — đã che bằng bước xếp lại f32. Nên đo lại trên 1–2k embedding thật khi có pipeline ONNX. |
| Máy đo chịu áp lực bộ nhớ, ổ ngoài; "lượt 1" không phải cold cache thật | Vừa | Nạp 200k lượt 1 = 2,1–2,4 s; có thể tệ hơn trên HDD/Windows. Nạp ở luồng nền sau khi UI hiện; vài giây đầu "Tìm tương tự" báo đang chuẩn bị. Chưa đo trên Windows. |
| RAM nếu giữ f32 = 2 KB/face (406 MB @200k) | Thấp nếu dùng int8 | int8 = 512 B/face. Nếu đổi model sang dim lớn hơn thì tăng tương ứng. |
| Rerank đọc 400 BLOB từ SQLite mỗi truy vấn | Thấp | Đo 1,0–2 ms thêm. Cần `prepare_cached`; nếu muốn có thể giữ f16 trong RAM (1 KB/face) thay đọc đĩa. [SUY] |
| `page_size` khó đổi sau khi tạo + WAL | Thấp | Đặt đúng lúc tạo library.db; sai thì dựng lại (dữ liệu dẫn xuất). |
| `foreign_keys`, `busy_timeout` là theo kết nối | Thấp | Một hàm `open_conn()` duy nhất đặt mọi PRAGMA; test bằng `open_in_memory`. |
| Hai file DB → không FK chéo; dữ liệu người dùng có thể trỏ font không còn | Chủ ý | Là tính năng (favorite sống sót khi font tạm vắng). UI hiện "không còn trong thư viện"; job dọn tuỳ chọn. |
| `trigram` không khớp truy vấn < 3 ký tự | Thấp | Fallback LIKE tiền tố (mục 3.3). |
| sqlite-vec 0.1.x chưa 1.0, KNN tuần tự | — | Không phụ thuộc. Nếu cần thì chỉ dùng hàm `vec_distance_*` để debug. |
| Migration: TTC trong DB cũ không có face index; file đã mất không băm được | Vừa | `legacy_ids.payload` giữ nguyên dữ liệu; nối lại khi tìm thấy file. |
| Vector Qdrant cũ khác cách render với pipeline mới | Vừa | `model_version` riêng, không trộn; mặc định tính lại. |

---

## 6. Ghi chú thêm

- `vec0` nhỏ hơn một nửa DB chuẩn vì đóng gói vector theo chunk; DB chuẩn với `page_size=16384` đạt 489 MB vs 400 MB của `vec0` @200k — đủ gần, không cần sqlite-vec chỉ để tiết kiệm đĩa.
- Có thể cân nhắc lưu `embeddings.vec` dạng f16 (1 KB) để giảm nửa file; mất chính xác không đáng kể với cosine [SUY, chưa đo].

## 7. Đã đo vs. suy luận (tóm tắt)

- **Đã đo**: mọi số trong mục 1, kích thước file theo `page_size`, hành vi FTS5 với tiếng Việt trên SQLite 3.45.0, DDL chạy được + trigger FTS + cascade + CHECK, các bẫy đo RAM.
- **Suy luận**: chi phí dựng bitset lọc, cập nhật tăng dần, thời gian băm BLAKE3 khi migrate, hiệu năng trên Windows/HDD, fallback LIKE < 3 ký tự, ngưỡng ~1 triệu face để xem lại HNSW, khả năng tái dùng vector Qdrant cũ, f16.
- **Chưa làm**: đo trên embedding CLIP thật; cold start thật (`sudo purge`); f16 trong RAM; Windows.
