//! D14: `claim_embedding_batch` phải lấy lô theo thứ tự cũ mà không quét lại cả hàng đợi ở mỗi lô
//! (trước đây nạp mọi dòng `pending` rồi chỉ dùng BATCH = 4 dòng đầu → O(N²/BATCH) khi nạp N face).
mod common;
use common::*;
use fontatlas_store::{EmbedJob, Store};

const RAW: &str = "claim-raw";
const BATCH: usize = 4;

fn claim_all(s: &mut Store) -> Vec<EmbedJob> {
    let mut out = Vec::new();
    loop {
        let j = s.claim_embedding_batch(RAW, BATCH).unwrap();
        if j.is_empty() {
            return out;
        }
        out.extend(j);
    }
}

fn status_of(s: &Store, hash: &[u8]) -> (String, Option<String>) {
    s.conn()
        .query_row(
            "SELECT status, error_code FROM pipeline_state WHERE hash=?1 AND stage='embedding'",
            [hash],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

#[test]
fn claiming_everything_does_not_rescan_the_queue_for_every_batch() {
    let (_d, mut s) = synthetic_library(40);
    s.enqueue_embeddings(RAW).unwrap();
    let all = claim_all(&mut s);
    assert_eq!(all.len(), 41, "40 face tổng hợp + roboto");
    // 41 face / lô 4 = 11 lần claim có việc + 1 lần rỗng; bộ đệm nạp một lần rồi một lần cuối thấy rỗng
    assert!(s.embed_claim_scans() <= 3, "quét hàng đợi {} lần", s.embed_claim_scans());
}

#[test]
fn claim_order_stays_oldest_update_first_then_hash() {
    let (_d, mut s) = synthetic_library(12);
    s.enqueue_embeddings(RAW).unwrap();
    // updated_at lệch nhau để thứ tự khác thứ tự hash
    s.conn()
        .execute_batch(
            "UPDATE pipeline_state SET updated_at = 5000 - (abs(random()) % 7) WHERE stage='embedding';",
        )
        .unwrap();
    let want: Vec<Vec<u8>> = {
        let mut st = s
            .conn()
            .prepare("SELECT hash FROM pipeline_state WHERE stage='embedding' AND status='pending' ORDER BY updated_at, hash, face_index")
            .unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    let got: Vec<Vec<u8>> = claim_all(&mut s).into_iter().map(|j| j.key.hash.to_vec()).collect();
    assert_eq!(got, want);
}

#[test]
fn a_row_that_stopped_being_pending_after_the_scan_is_not_claimed() {
    let (_d, mut s) = synthetic_library(10);
    s.enqueue_embeddings(RAW).unwrap();
    let first = s.claim_embedding_batch(RAW, BATCH).unwrap();
    assert_eq!(first.len(), BATCH);
    // dòng kế tiếp theo thứ tự đã nằm trong bộ đệm; đổi sang done từ ngoài rồi claim tiếp
    let next: Vec<u8> = s
        .conn()
        .query_row(
            "SELECT hash FROM pipeline_state WHERE stage='embedding' AND status='pending' ORDER BY updated_at, hash, face_index LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    s.conn()
        .execute("UPDATE pipeline_state SET status='done' WHERE hash=?1 AND stage='embedding'", [&next])
        .unwrap();
    let rest = claim_all(&mut s);
    assert!(rest.iter().all(|j| j.key.hash[..] != next[..]), "dòng đã done vẫn bị claim");
    assert_eq!(rest.len(), 11 - BATCH - 1);
}

#[test]
fn released_faces_are_claimed_again_once_the_buffer_drains() {
    let (_d, mut s) = synthetic_library(10);
    s.enqueue_embeddings(RAW).unwrap();
    let first = s.claim_embedding_batch(RAW, BATCH).unwrap();
    let keys: Vec<_> = first.iter().map(|j| j.key).collect();
    s.release_embeddings(RAW, &keys).unwrap();
    let all = claim_all(&mut s);
    assert_eq!(all.len(), 11, "mọi face được claim, kể cả các face vừa trả về pending");
    let mut seen: Vec<_> = all.iter().map(|j| j.key.hash).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 11, "không face nào bị claim hai lần");
}

#[test]
fn faces_enqueued_after_the_queue_drained_are_picked_up() {
    let (_d, mut s) = synthetic_library(3);
    s.enqueue_embeddings(RAW).unwrap();
    assert_eq!(claim_all(&mut s).len(), 4);
    s.conn()
        .execute_batch(
            "INSERT INTO files(hash,size,container,num_faces,first_seen_at) VALUES (randomblob(32),1000,'sfnt',1,99);
             INSERT INTO file_locations(file_id,source_id,path,size,mtime_ns,last_seen_at,created_ns,created_src)
               SELECT file_id,1,'/late.ttf',1000,1,1,1,'disk' FROM files ORDER BY file_id DESC LIMIT 1;
             INSERT INTO faces(file_id,face_index,family_name,weight_class,is_italic,outline_format,license_class,vi_status,family_key,search_text)
               SELECT file_id,0,'Late',400,0,'glyf','ofl','full','late','late' FROM files ORDER BY file_id DESC LIMIT 1;",
        )
        .unwrap();
    s.enqueue_embeddings(RAW).unwrap();
    assert_eq!(claim_all(&mut s).len(), 1);
}

#[test]
fn cloud_only_faces_stay_pending_and_do_not_hold_back_the_others() {
    let (_d, mut s) = synthetic_library(10);
    s.enqueue_embeddings(RAW).unwrap();
    // 3 face đầu theo thứ tự claim chỉ còn bản cloud
    let cloud: Vec<Vec<u8>> = {
        let mut st = s
            .conn()
            .prepare("SELECT hash FROM pipeline_state WHERE stage='embedding' ORDER BY updated_at, hash, face_index LIMIT 3")
            .unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    for h in &cloud {
        s.conn()
            .execute(
                "UPDATE file_locations SET cloud_only=1 WHERE file_id=(SELECT file_id FROM files WHERE hash=?1)",
                [h],
            )
            .unwrap();
    }
    let got = claim_all(&mut s);
    assert_eq!(got.len(), 11 - 3);
    for h in &cloud {
        assert_eq!(status_of(&s, h), ("pending".to_string(), None), "bản chỉ-cloud giữ pending, không bị đánh lỗi");
    }
}

#[test]
fn more_cloud_only_faces_than_one_scan_page_do_not_starve_the_rest() {
    let (_d, mut s) = synthetic_library(1200);
    s.enqueue_embeddings(RAW).unwrap();
    // 1030 face đầu theo thứ tự claim (> một trang quét 1024) chỉ còn bản cloud
    s.conn()
        .execute_batch(
            "UPDATE file_locations SET cloud_only=1 WHERE file_id IN (
                SELECT f.file_id FROM files f JOIN (
                    SELECT hash FROM pipeline_state WHERE stage='embedding' ORDER BY updated_at, hash, face_index LIMIT 1030
                ) p ON p.hash = f.hash);",
        )
        .unwrap();
    let got = claim_all(&mut s);
    assert_eq!(got.len(), 1201 - 1030);
    assert_eq!(count(&s, "SELECT COUNT(*) FROM pipeline_state WHERE stage='embedding' AND status='pending'"), 1030);
    assert!(s.embed_claim_scans() <= 8, "quét hàng đợi {} lần", s.embed_claim_scans());
}

#[test]
fn a_queue_of_exactly_one_scan_page_is_fully_claimed() {
    let (_d, mut s) = synthetic_library(1023);
    s.enqueue_embeddings(RAW).unwrap();
    assert_eq!(claim_all(&mut s).len(), 1024, "trang đầy rồi trang rỗng: không bỏ sót, không lặp");
}

#[test]
fn a_face_with_no_present_location_is_marked_missing_file() {
    let (_d, mut s) = synthetic_library(5);
    s.enqueue_embeddings(RAW).unwrap();
    let gone: Vec<u8> = s
        .conn()
        .query_row("SELECT hash FROM pipeline_state WHERE stage='embedding' ORDER BY updated_at, hash LIMIT 1", [], |r| r.get(0))
        .unwrap();
    s.conn()
        .execute(
            "UPDATE file_locations SET missing=1 WHERE file_id=(SELECT file_id FROM files WHERE hash=?1)",
            [&gone],
        )
        .unwrap();
    let got = claim_all(&mut s);
    assert_eq!(got.len(), 5);
    assert_eq!(status_of(&s, &gone), ("failed".to_string(), Some("missing_file".to_string())));
}
