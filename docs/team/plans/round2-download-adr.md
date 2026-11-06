# ADR — Model download async, bounded Range (2026-11-06)

Trạng thái: triển khai riêng sau checkpoint backend `57694d5`, chờ Lead/reviewer nghiệm thu. Không sửa công thức analyzer, manifest, nguồn model, cấu hình người dùng, IPC/DTO hay frontend ở giai đoạn này.

## Nguồn và quyết định

Đã tra trực tiếp nguồn chính thống ngày 2026-11-06:

- [Hugging Face migration v1.0](https://huggingface.co/docs/huggingface_hub/en/concepts/migration): hf_transfer đã bị gỡ, hướng hiện tại dùng hf_xet. Không khuyên cài hf_transfer hoặc mặc định dùng mirror.
- [hf-hub Rust README](https://github.com/huggingface/hf-hub), [mã repository/download.rs trên main](https://github.com/huggingface/hf-hub/blob/main/hf-hub/src/repository/download.rs): có download_file_stream/to_bytes với Range start-inclusive/end-exclusive, Xet streaming; không được nói OSS thiếu partial download. Nhánh HTTP fallback trong source đọc không tự kiểm exact Content-Range theo trust contract của FontAtlas. `main` là snapshot tra cứu thời điểm trên, không ghim được SHA nguồn nghiên cứu; không dùng snapshot đó làm dependency.
- [ureq 3.4.2 timeout config](https://docs.rs/ureq/3.4.2/ureq/config/struct.ConfigBuilder.html): recv_body là ngân sách toàn body; baseline pump spawn luồng blocking reader rồi bỏ luồng khi cancel. Không giữ cách này khi thêm concurrency/retry.
- API thực tế đã đối chiếu mã local đúng phiên bản Cargo.lock: `reqwest-0.12.26/src/async_impl/{client,response}.rs`, `tokio-1.48.0`. Không dùng tên feature của tài liệu latest 0.13.x. [reqwest 0.12.26 docs](https://docs.rs/reqwest/0.12.26/reqwest/), [Tokio 1.48.0 runtime](https://docs.rs/tokio/1.48.0/tokio/runtime/).

Giữ extractor safetensors, checksum ghim và API hiện có; thay riêng transport bằng reqwest async `=0.12.26` (default features off, rustls-tls), Tokio `=1.48.0` (rt/time/net), futures-util `=0.3.31`. Hai crate đầu đã có phiên bản trong lock; TLS cần thêm hyper-rustls/tokio-rustls. Không broad cargo update. Không vô hiệu TLS/certificate verification, không thay proxy, không thêm CLI/Python runtime.

Lý do chọn: bảo toàn luồng lấy riêng vision span, checksum header/vision/local và fallback toàn nguồn; giải quyết cancel bằng drop future/Response thay vì tự xây transport ureq có thể abort. Đã cân nhắc hf-hub/Xet và xác nhận có range; chưa benchmark Xet hoặc Internet thật nên không kết luận nhanh hơn Xet. Có thể xem xét Xet trong đợt riêng nếu lợi ích đo được đủ bù dependency và integration.

## Bounded transfer và resume

- Nguồn bundled ghim revision `75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2`: source 1,500,800,904 byte, vision 371,536,896 byte, local vision safetensors 371,561,688 byte. Manifest SHA không thay đổi.
- Chỉ khi header 206 + exact Content-Range được kiểm, mới lấy vision. Với phần còn lại >=8 MiB, lấy trước một vision chunk 4 MiB để xác nhận behavior payload; sau đó tối đa ba future HTTP cùng lúc, mỗi chunk <=4 MiB. Batch barrier giới hạn payload out-of-order <=12 MiB (cộng buffer nội bộ HTTP/TLS); không tạo hàng đợi tăng theo kích thước file.
- try_join_all giữ thứ tự đầu vào. Chỉ feed các chunk khi cả batch thành công, theo thứ tự offset; Extract/chốt `.part.json` chỉ ghi contiguous prefix đã sync_data. Chunk out-of-order không được coi là durable và không được nhảy qua hole khi resume.
- Từng 206 phải có exact Content-Range và exact body length. 200 sau probe làm drop mọi sibling future/Response trước khi mở full stream; chế độ full sticky trong lần ensure đó. Nguồn không hỗ trợ Range chỉ có một full stream, không mở nhiều kết nối thử mù.
- Tối đa 3 attempt cho lỗi network/timeout/truncation, backoff 250/500 ms có kiểm cancel. Checksum mật mã/oversize/Content-Range sai không auto retry. HTTP 429 không auto retry: trả Retry-After nguyên văn nếu có, để người dùng thử sau; không lặp ngay.
- Client được reuse trong lần ensure. Connect/header/per-body-read timeout 30 giây. Kiểm cờ cancel khi chờ I/O mỗi 50 ms, drop I/O futures; không spawn blocking body reader, không có worker đọc detached tích tụ qua retry. Idle pooled connections khác active transfers.
- Synchronous ensure_resumable chỉ gọi trên dedicated std::thread ngoài mọi Tokio runtime context. Runtime guard chạy trước lock/file mutation và trả lỗi có cấu trúc cho async/spawn_blocking; không quảng cáo spawn_blocking là call-site hỗ trợ. Call-site production duy nhất: services/model.rs::run_download, chạy trong thread `fontatlas-model`. API ensure sync, các test sync và parity test dùng cùng contract; không mở async public API mới.
- Checksum header và vision, stream checksum local, rồi đọc lại TOÀN file `.part` đã sync_all, so size và SHA-256 với manifest trước rename. Chỉ sau đó hàm thành công, service phát ModelEvent::Ready. Không có Ready cho mismatch.
- Unix: fsync thư mục sau rename. Non-Unix/Windows: không File::open directory theo cách thiếu backup semantics; file content vẫn sync_all, nhưng độ bền rename qua mất điện chưa được bảo đảm bằng directory fsync. Portable helper được compile và chạy trong unit test native; chưa có Windows target/runtime để kiểm Windows thực.

## Progress: transport khác durable prefix

Không đổi IPC hoặc bindings. DownloadProgress/ModelEvent::Progress:

- done/total là byte transport hữu ích của range header+vision; fallback là byte full source. Không phải số byte local model vì hai header khác nhau. ModelStatus ở Ready dùng local model size như baseline.
- Trong batch, done gồm byte đã nhận của mọi chunk, kể cả out-of-order chưa durable. Khi retry, high-water giữ progress không lùi, không vượt total; dữ liệu bỏ đi/retransmit không được cộng hai lần. Durable resume bytes chỉ đọc từ `.part.json`, không lấy từ event.done.
- bytesPerSec là tốc độ tiến triển transport hữu ích trung bình của lần ensure, trừ prefix đã resume; không phải tổng wire throughput gồm retransmit/TLS. UI đổi decimal MB/s bằng /1,000,000. Retry/replay/checksum làm ETA này kém chính xác; ETA chỉ ước lượng payload còn lại.
- done == total nghĩa nhận đủ payload, vẫn chưa xác nhận model dùng được. UI phải giữ downloading cho tới Ready; ETA null lúc hết payload không có nghĩa đã xong checksum. Cancel được kiểm trước/sau kiểm toàn file; thao tác file/hash đồng bộ không có SLA 50 ms như polling I/O.

## Bằng chứng và giới hạn

HTTP fixtures chỉ bind 127.0.0.1; không tải model thật hoặc thay dữ liệu thật. Bộ test gồm baseline range/full/redirect, checksum và load model tiny; thêm max-active=3 + out-of-order completion, cancel/stall và không còn active transfer, resume đúng prefix 4 MiB, worker sai Content-Range, worker200 fallback, truncation retry và progress không lùi, persistent timeout dừng đúng giới hạn, SHA local cuối sai không Ready/retry, 429 Retry-After không retry, metadata resume malformed và guard async/spawn_blocking không mutate filesystem. Fixture raw socket trên macOS phải set_nonblocking(false) sau accept vì kế thừa listener flags.

Local 16 MiB fixture có delay được dùng để chứng minh overlap và in elapsed/peak, không phải benchmark tốc độ Internet hay so với Xet. Chưa kiểm Internet/CDN thật, Windows runtime/target build, mất điện thực, hoặc UI native live cho download.

Kiểm tra đã chạy từ worktree này (manifest `UX/src-tauri/Cargo.toml`):

- `cargo test --offline -p fontatlas-embed --test model --manifest-path UX/src-tauri/Cargo.toml`: 29 passed; `/tmp/fontatlas-download-final.log`.
- `cargo test --offline -p fontatlas-embed --lib --manifest-path UX/src-tauri/Cargo.toml`: 5 passed, gồm portable durability helper; `/tmp/fontatlas-embed-lib-final.log`.
- `cargo test --offline -p app --test embed --manifest-path UX/src-tauri/Cargo.toml`: 10 passed, gồm service Ready khởi động embed và lỗi model qua restart; `/tmp/fontatlas-app-model-final.log`.
- `cargo clippy --offline --manifest-path UX/src-tauri/Cargo.toml -p fontatlas-embed --lib --tests -- -D warnings`: qua, không warning; `/tmp/fontatlas-embed-clippy-final.log`.
- `cargo clippy --offline --manifest-path UX/src-tauri/Cargo.toml -p app --lib`: qua, không warning; `/tmp/fontatlas-app-clippy-final.log`.
- `cargo test --offline -p fontatlas-embed --test model parallel_ranges_are_bounded_ordered_and_sha_verified --manifest-path UX/src-tauri/Cargo.toml -- --nocapture`: qua, localhost 16 MiB khoảng 2.98 giây, peak=3, assert completion out-of-order; `/tmp/fontatlas-local-overlap.log`. Đây là bằng chứng overlap, không so sánh tốc độ Internet hoặc single-stream/Xet.
- rustfmt check chỉ hai file Rust đã sửa và git diff --check: qua. Không reformat toàn repository.

HTTP fixture tests cần quyền bind localhost vì sandbox mặc định chặn bind socket; đã chạy với phê duyệt hẹp cho kiểm thử localhost. Không thay permission profile hoặc đổi mode agent.
