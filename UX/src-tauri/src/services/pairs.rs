//! Cặp font đã lưu (user.db v3).
use fontatlas_store::{FaceKey, NewPair, SavedPair};

use crate::dto::analysis::{NewPairDto, PairRoleDto, SavedPairDto};
use crate::dto::library::FaceKeyDto;
use crate::error::CmdError;
use crate::state::AppState;

fn role_str(r: PairRoleDto) -> String {
    match r {
        PairRoleDto::Heading => "heading",
        PairRoleDto::Body => "body",
        PairRoleDto::Code => "code",
    }
    .into()
}

pub fn save_pair(state: &AppState, p: &NewPairDto) -> Result<i64, CmdError> {
    if let Some(m) = &p.mode
        && !["safe", "controlled", "experimental", "blocked"].contains(&m.as_str())
    {
        return Err(CmdError::invalid_arg(format!("mode không hợp lệ: {m}")));
    }
    let np = NewPair {
        a: FaceKey::try_from(&p.a)?,
        a_role: role_str(p.a_role),
        b: FaceKey::try_from(&p.b)?,
        b_role: role_str(p.b_role),
        mode: p.mode.clone(),
        preset: p.preset.clone(),
        sample_text: p.sample_text.clone(),
        collection_id: p.collection_id,
        note: p.note.clone(),
    };
    state.with_user(|u| Ok(u.save_pair(&np)?))
}

fn dto(p: SavedPair) -> SavedPairDto {
    SavedPairDto {
        id: p.id,
        a: FaceKeyDto::from(&p.a),
        a_role: p.a_role,
        b: FaceKeyDto::from(&p.b),
        b_role: p.b_role,
        mode: p.mode,
        preset: p.preset,
        sample_text: p.sample_text,
        collection_id: p.collection_id,
        note: p.note,
        created_at: p.created_at,
    }
}

pub fn list_pairs(state: &AppState, collection_id: Option<i64>) -> Result<Vec<SavedPairDto>, CmdError> {
    state.with_user(|u| Ok(u.pairs(collection_id)?.into_iter().map(dto).collect()))
}

pub fn delete_pair(state: &AppState, id: i64) -> Result<(), CmdError> {
    state.with_user(|u| Ok(u.delete_pair(id)?))
}
