//! Đo trên 100k face (chạy tay: `cargo test -p fontatlas-store --release --test bench_100k -- --ignored --nocapture`).
mod common;
use common::*;
use fontatlas_store::{FaceQuery, SortKey, SourceKind, Store};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn big(n: usize) -> (tempfile::TempDir, Store) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), fonts.join("roboto.ttf")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    let c = s.conn();
    c.execute_batch(&format!(
        "CREATE TEMP TABLE seq AS WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i < {n}) SELECT i, randomblob(32) h FROM n;
         INSERT INTO files(hash,size,container,num_faces,first_seen_at) SELECT h, 1000, 'sfnt', 1, i FROM seq;
         INSERT INTO file_locations(file_id,source_id,path,size,mtime_ns,last_seen_at,created_ns,created_src)
           SELECT fi.file_id, 1, '/bench/'||seq.i||'.ttf', 1000, 1, 1, 1000000000000000000 + seq.i*1000000000, 'disk' FROM files fi JOIN seq ON seq.h = fi.hash;
         INSERT INTO faces(file_id,face_index,family_name,weight_class,is_italic,outline_format,license_class,vi_status,family_key,search_text)
           SELECT fi.file_id, 0, 'Fam'||(seq.i/5), 100+(seq.i%9)*100, seq.i%2, 'glyf','ofl','full','fam'||(seq.i/5),'fam'||(seq.i/5) FROM files fi JOIN seq ON seq.h = fi.hash;
         INSERT INTO analyses(hash,face_index,analyzer_version,source,metadata_json,vietnamese_json,anatomy_json,analyzed_at)
           SELECT seq.h, 0, 'bench', 'local', '{{\"extras\":{{\"years\":{{\"created\":2010,\"modified\":2012,\"copyright\":2005}}}}}}', '{{}}', '{{}}', seq.i FROM seq;"
    ))
    .unwrap();
    (d, s)
}

#[test]
#[ignore]
fn backfill_and_grouped_sorts_on_100k() {
    let (d, s) = big(100_000);
    let t = Instant::now();
    let n = {
        s.conn().execute_batch("DELETE FROM meta WHERE key='release_backfill'; UPDATE faces SET release_year=NULL, release_year_src=NULL;").unwrap();
        drop(s);
        let t0 = Instant::now();
        let s2 = Store::open(&d.path().join("db")).unwrap().0;
        println!("open+backfill 100k: {:?}", t0.elapsed());
        s2.query_faces(&FaceQuery { limit: 1, ..Default::default() }).unwrap().total
    };
    println!("total {n} ({:?})", t.elapsed());
    let s = Store::open(&d.path().join("db")).unwrap().0;
    let t0 = Instant::now();
    drop(Store::open(&d.path().join("db")).unwrap());
    println!("re-open (flag set): {:?}", t0.elapsed());
    for sort in [SortKey::Family, SortKey::Released, SortKey::FileCreated] {
        for grouped in [false, true] {
            let q = FaceQuery { sort, descending: true, group_by_family: grouped, limit: 100, include_hidden: std::env::var("BENCH_HIDDEN").is_ok(), ..Default::default() };
            let t0 = Instant::now();
            let p = s.query_faces(&q).unwrap();
            let t1 = t0.elapsed();
            let t0 = Instant::now();
            let g = if grouped { s.family_groups(&q, None).unwrap().len() } else { 0 };
            println!("{sort:?} grouped={grouped}: query_faces {:?} (total {}), family_groups {:?} ({g} groups)", t1, p.total, t0.elapsed());
        }
    }
}
