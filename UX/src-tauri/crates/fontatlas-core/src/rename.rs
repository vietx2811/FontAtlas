//! G7b: bản sao ĐỔI TÊN của font biến thiên (VF) để dùng song song với bản static cùng tên PostScript.
//! Chỉ viết lại bảng `name` (họ, full name, PostScript name, unique ID, PostScript name của các named instance trong `fvar`);
//! mọi bảng khác giữ nguyên từng byte; checksum bảng và `head.checkSumAdjustment` tính lại. KHÔNG sửa file gốc (làm việc trên bản
//! trong bộ nhớ). Quy tắc tên (cố định, Lead đã duyệt): họ `X` → `X VF`; full name `X VF <Kiểu>`; PostScript chèn `VF` vào phần họ
//! (`Oswald-Regular` → `OswaldVF-Regular`, không có gạch thì thêm `VF` ở cuối); unique ID thêm `;FontAtlas-VF`.
//! v1 chỉ nhận sfnt đơn (TTF/OTF); TTC/WOFF/WOFF2 trả `Unsupported`.
use std::collections::BTreeSet;

pub const SUFFIX: &str = "VF";

#[derive(Debug, thiserror::Error)]
pub enum RenameError {
    /// `ttc` | `woff` | `woff2`
    #[error("định dạng chưa hỗ trợ bản sao đổi tên: {0}")]
    Unsupported(&'static str),
    #[error("font không hợp lệ: {0}")]
    Malformed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameOutcome {
    pub bytes: Vec<u8>,
    /// `OS/2.fsType` có bit "Restricted License embedding": vẫn tạo bản sao, UI chỉ hiện cảnh báo (tool, not gatekeeper).
    pub license_restricted: bool,
    pub family: String,
    pub postscript_name: String,
}

fn be16(d: &[u8], o: usize) -> Result<u16, RenameError> {
    d.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]])).ok_or_else(|| RenameError::Malformed("cắt cụt".into()))
}

fn be32(d: &[u8], o: usize) -> Result<u32, RenameError> {
    d.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| RenameError::Malformed("cắt cụt".into()))
}

/// Phần họ của PostScript name + `VF`. Đã có thì giữ nguyên (idempotent).
pub fn ps_name(orig: &str) -> String {
    match orig.split_once('-') {
        Some((fam, rest)) if !fam.ends_with(SUFFIX) => format!("{fam}{SUFFIX}-{rest}"),
        Some(_) => orig.to_string(),
        None if orig.ends_with(SUFFIX) => orig.to_string(),
        None => format!("{orig}{SUFFIX}"),
    }
}

fn family_name(orig: &str) -> String {
    if orig.ends_with(&format!(" {SUFFIX}")) { orig.to_string() } else { format!("{orig} {SUFFIX}") }
}

struct NameRec {
    platform: u16,
    encoding: u16,
    language: u16,
    id: u16,
    data: Vec<u8>,
}

impl NameRec {
    fn text(&self) -> String {
        if matches!(self.platform, 0 | 3) {
            let u: Vec<u16> = self.data.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&u)
        } else {
            self.data.iter().map(|&b| b as char).collect() // MacRoman: giữ nguyên byte (Latin-1 round-trip)
        }
    }
    fn set_text(&mut self, s: &str) {
        self.data = if matches!(self.platform, 0 | 3) {
            s.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
        } else {
            s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
        };
    }
}

struct Table {
    tag: [u8; 4],
    data: Vec<u8>,
}

fn checksum(d: &[u8]) -> u32 {
    let mut sum = 0u32;
    for c in d.chunks(4) {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        sum = sum.wrapping_add(u32::from_be_bytes(w));
    }
    sum
}

/// PostScript name id của các named instance trong `fvar` (bỏ 0xFFFF = không có).
fn instance_ps_ids(fvar: &[u8]) -> BTreeSet<u16> {
    let mut out = BTreeSet::new();
    let mut parse = || -> Result<(), RenameError> {
        let axes_off = be16(fvar, 4)? as usize;
        let axis_count = be16(fvar, 8)? as usize;
        let axis_size = be16(fvar, 10)? as usize;
        let inst_count = be16(fvar, 12)? as usize;
        let inst_size = be16(fvar, 14)? as usize;
        let base = axes_off + axis_count * axis_size;
        if inst_size >= 4 + 4 * axis_count + 2 {
            for i in 0..inst_count {
                let id = be16(fvar, base + i * inst_size + 4 + 4 * axis_count)?;
                if id != 0xFFFF {
                    out.insert(id);
                }
            }
        }
        Ok(())
    };
    let _ = parse();
    out
}

fn parse_name(t: &[u8]) -> Result<(Vec<NameRec>, Vec<Vec<u8>>), RenameError> {
    let format = be16(t, 0)?;
    let count = be16(t, 2)? as usize;
    let storage = be16(t, 4)? as usize;
    let mut recs = Vec::with_capacity(count);
    for i in 0..count {
        let o = 6 + i * 12;
        let (len, off) = (be16(t, o + 8)? as usize, be16(t, o + 10)? as usize);
        let data = t.get(storage + off..storage + off + len).ok_or_else(|| RenameError::Malformed("chuỗi name ngoài bảng".into()))?.to_vec();
        recs.push(NameRec { platform: be16(t, o)?, encoding: be16(t, o + 2)?, language: be16(t, o + 4)?, id: be16(t, o + 6)?, data });
    }
    let mut tags = Vec::new();
    if format == 1 {
        let n = be16(t, 6 + count * 12)? as usize;
        for i in 0..n {
            let o = 6 + count * 12 + 2 + i * 4;
            let (len, off) = (be16(t, o)? as usize, be16(t, o + 2)? as usize);
            tags.push(t.get(storage + off..storage + off + len).ok_or_else(|| RenameError::Malformed("langTag ngoài bảng".into()))?.to_vec());
        }
    }
    Ok((recs, tags))
}

fn build_name(mut recs: Vec<NameRec>, tags: &[Vec<u8>]) -> Vec<u8> {
    recs.sort_by_key(|r| (r.platform, r.encoding, r.language, r.id));
    let format: u16 = if tags.is_empty() { 0 } else { 1 };
    let header = 6 + recs.len() * 12 + if format == 1 { 2 + tags.len() * 4 } else { 0 };
    let mut storage: Vec<u8> = Vec::new();
    let mut rec_bytes = Vec::new();
    for r in &recs {
        rec_bytes.extend_from_slice(&[r.platform.to_be_bytes(), r.encoding.to_be_bytes(), r.language.to_be_bytes(), r.id.to_be_bytes()].concat());
        rec_bytes.extend_from_slice(&(r.data.len() as u16).to_be_bytes());
        rec_bytes.extend_from_slice(&(storage.len() as u16).to_be_bytes());
        storage.extend_from_slice(&r.data);
    }
    let mut tag_bytes = Vec::new();
    if format == 1 {
        tag_bytes.extend_from_slice(&(tags.len() as u16).to_be_bytes());
        for t in tags {
            tag_bytes.extend_from_slice(&(t.len() as u16).to_be_bytes());
            tag_bytes.extend_from_slice(&(storage.len() as u16).to_be_bytes());
            storage.extend_from_slice(t);
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(&format.to_be_bytes());
    out.extend_from_slice(&(recs.len() as u16).to_be_bytes());
    out.extend_from_slice(&(header as u16).to_be_bytes());
    out.extend_from_slice(&rec_bytes);
    out.extend_from_slice(&tag_bytes);
    out.extend_from_slice(&storage);
    out
}

/// Tạo bản sao đổi tên của một VF. `data` = toàn bộ file gốc (không bị sửa).
pub fn rename_vf_copy(data: &[u8]) -> Result<RenameOutcome, RenameError> {
    match data.get(0..4) {
        Some(b"ttcf") => return Err(RenameError::Unsupported("ttc")),
        Some(b"wOFF") => return Err(RenameError::Unsupported("woff")),
        Some(b"wOF2") => return Err(RenameError::Unsupported("woff2")),
        Some([0, 1, 0, 0]) | Some(b"OTTO") | Some(b"true") => {}
        _ => return Err(RenameError::Malformed("không phải sfnt".into())),
    }
    let n = be16(data, 4)? as usize;
    let mut tables: Vec<Table> = Vec::with_capacity(n);
    for i in 0..n {
        let o = 12 + i * 16;
        let tag: [u8; 4] = data.get(o..o + 4).ok_or_else(|| RenameError::Malformed("cắt cụt".into()))?.try_into().unwrap();
        let (off, len) = (be32(data, o + 8)? as usize, be32(data, o + 12)? as usize);
        let d = data.get(off..off + len).ok_or_else(|| RenameError::Malformed(format!("bảng {} ngoài file", String::from_utf8_lossy(&tag))))?;
        tables.push(Table { tag, data: d.to_vec() });
    }
    let find = |tag: &[u8; 4], ts: &[Table]| ts.iter().position(|t| &t.tag == tag);
    let fvar_i = find(b"fvar", &tables).ok_or_else(|| RenameError::Malformed("không phải font biến thiên (thiếu fvar)".into()))?;
    let name_i = find(b"name", &tables).ok_or_else(|| RenameError::Malformed("thiếu bảng name".into()))?;
    let ps_ids = instance_ps_ids(&tables[fvar_i].data);
    let restricted = find(b"OS/2", &tables).and_then(|i| be16(&tables[i].data, 8).ok()).is_some_and(|fs| fs & 0x0002 != 0);

    let (mut recs, tags) = parse_name(&tables[name_i].data)?;
    // họ tham chiếu theo (platform, language): nameID 16 nếu có, không thì 1 — để chèn VF đúng chỗ vào full name
    let fam_of: Vec<((u16, u16), String)> = recs
        .iter()
        .filter(|r| r.id == 16 || r.id == 1)
        .map(|r| ((r.platform, r.language), r.text()))
        .collect();
    let family_for = |r: &NameRec| -> Option<String> {
        recs_family(&fam_of, r)
    };
    let orig_ps = recs.iter().find(|r| r.id == 6 && r.platform == 3).or_else(|| recs.iter().find(|r| r.id == 6)).map(|r| r.text()).unwrap_or_default();
    let mut new_family = String::new();
    let mut new_ps = String::new();
    let fams: Vec<Option<String>> = recs.iter().map(&family_for).collect();
    for (r, fam) in recs.iter_mut().zip(fams) {
        let t = r.text();
        match r.id {
            1 | 16 => {
                let nf = family_name(&t);
                if new_family.is_empty() || r.id == 16 {
                    new_family = nf.clone();
                }
                r.set_text(&nf);
            }
            4 => {
                let nt = match fam {
                    Some(f) if !f.is_empty() && t.starts_with(&f) && !t[f.len()..].starts_with(&format!(" {SUFFIX}")) => format!("{} {SUFFIX}{}", f, &t[f.len()..]),
                    _ if t.contains(&format!(" {SUFFIX}")) => t.clone(),
                    _ => format!("{t} {SUFFIX}"),
                };
                r.set_text(&nt);
            }
            6 => {
                let np = ps_name(&t);
                if new_ps.is_empty() {
                    new_ps = np.clone();
                }
                r.set_text(&np);
            }
            3 => {
                if !t.ends_with(";FontAtlas-VF") {
                    r.set_text(&format!("{t};FontAtlas-VF"));
                }
            }
            25 => {
                if !t.ends_with(SUFFIX) {
                    r.set_text(&format!("{t}{SUFFIX}"));
                }
            }
            id if ps_ids.contains(&id) => r.set_text(&ps_name(&t)),
            _ => {}
        }
    }
    // mô tả: thay mọi nameID 10 bằng một dòng nói rõ đây là bản sao đổi tên
    recs.retain(|r| r.id != 10);
    let mut desc = NameRec { platform: 3, encoding: 1, language: 0x409, id: 10, data: Vec::new() };
    desc.set_text(&format!("Renamed copy of {orig_ps} made by Font Atlas so it can be used alongside the static version."));
    recs.push(desc);
    tables[name_i].data = build_name(recs, &tags);

    // dựng lại sfnt: bảng theo thứ tự tag, 4-byte aligned, checksum và head.checkSumAdjustment
    tables.sort_by_key(|t| t.tag);
    if let Some(h) = find(b"head", &tables)
        && tables[h].data.len() >= 12
    {
        tables[h].data[8..12].copy_from_slice(&[0; 4]);
    }
    let n = tables.len();
    let (mut entry, mut pow) = (0u16, 1u16);
    while pow * 2 <= n as u16 {
        pow *= 2;
        entry += 1;
    }
    let search = pow * 16;
    let mut out = Vec::new();
    out.extend_from_slice(&data[0..4]);
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&search.to_be_bytes());
    out.extend_from_slice(&entry.to_be_bytes());
    out.extend_from_slice(&((n as u16) * 16 - search).to_be_bytes());
    let mut offset = 12 + 16 * n;
    let mut body = Vec::new();
    for t in &tables {
        out.extend_from_slice(&t.tag);
        out.extend_from_slice(&checksum(&t.data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(t.data.len() as u32).to_be_bytes());
        body.extend_from_slice(&t.data);
        let pad = (4 - t.data.len() % 4) % 4;
        body.extend(std::iter::repeat_n(0u8, pad));
        offset += t.data.len() + pad;
    }
    out.extend_from_slice(&body);
    if let Some(h) = find(b"head", &tables) {
        // vị trí của head trong file
        let mut off = 12 + 16 * n;
        for t in &tables[..h] {
            off += t.data.len() + (4 - t.data.len() % 4) % 4;
        }
        let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[off + 8..off + 12].copy_from_slice(&adj.to_be_bytes());
    }
    Ok(RenameOutcome { bytes: out, license_restricted: restricted, family: new_family, postscript_name: new_ps })
}

fn recs_family(fam_of: &[((u16, u16), String)], r: &NameRec) -> Option<String> {
    // ưu tiên họ cùng (platform, language) với bản ghi; nameID 16 đứng sau 1 trong danh sách nên lấy bản cuối khớp
    fam_of.iter().filter(|(k, _)| *k == (r.platform, r.language)).map(|(_, s)| s.clone()).next_back()
}
