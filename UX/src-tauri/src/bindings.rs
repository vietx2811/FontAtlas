//! Hợp đồng TypeScript: `commands.ts` (bản đồ lệnh) và `index.ts` sinh từ bảng `COMMANDS`; kiểu DTO do ts-rs xuất
//! vào `UX/bindings/` khi chạy `cargo test -p app` (env `TS_RS_EXPORT_DIR`, xem `.cargo/config.toml`).
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::commands::{COMMANDS, CommandSig};

/// Thư mục xuất: `UX/bindings`.
pub fn bindings_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../bindings")
}

fn camel(s: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Tách tham số kiểu generic ở mức ngoài cùng: `Result<A, B>` → ["A", "B"].
fn generic_args(inner: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in inner.char_indices() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&inner[start..]);
    out
}

/// Kiểu Rust (đã bỏ khoảng trắng) → kiểu TS; gom tên kiểu DTO cần import.
pub fn ts_type(rust: &str, uses: &mut BTreeSet<String>) -> String {
    let t: String = rust.chars().filter(|c| !c.is_whitespace()).collect();
    if t == "()" {
        return "null".into();
    }
    if let Some(inner) = t.strip_prefix("Result<").and_then(|x| x.strip_suffix('>')) {
        return ts_type(generic_args(inner)[0], uses);
    }
    if let Some(inner) = t.strip_prefix("Option<").and_then(|x| x.strip_suffix('>')) {
        return format!("{} | null", ts_type(inner, uses));
    }
    if let Some(inner) = t.strip_prefix("Vec<").and_then(|x| x.strip_suffix('>')) {
        let i = ts_type(inner, uses);
        return if i.contains(' ') { format!("Array<{i}>") } else { format!("{i}[]") };
    }
    if let Some(inner) = t.strip_prefix("Channel<").and_then(|x| x.strip_suffix('>')) {
        return format!("Channel<{}>", ts_type(inner, uses));
    }
    match t.as_str() {
        "String" | "&str" => "string".into(),
        "bool" => "boolean".into(),
        "serde_json::Value" | "Value" => "unknown".into(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize" | "f32" | "f64" => "number".into(),
        name => {
            uses.insert(name.to_string());
            name.to_string()
        }
    }
}

pub fn commands_ts_for(table: &[CommandSig]) -> String {
    let mut uses = BTreeSet::new();
    let mut body = String::new();
    for c in table {
        let args = if c.args.is_empty() {
            "Record<string, never>".to_string()
        } else {
            let fields: Vec<String> = c.args.iter().map(|(n, t)| format!("{}: {}", camel(n), ts_type(t, &mut uses))).collect();
            format!("{{ {} }}", fields.join("; "))
        };
        body.push_str(&format!("  {}: {{ args: {args}; result: {} }};\n", c.name, ts_type(c.result, &mut uses)));
    }
    let uses_channel = body.contains("Channel<");
    let mut out = String::from("// Sinh tự động từ bảng lệnh (`cargo test -p app`, UPDATE_BINDINGS=1). Không sửa tay.\n");
    if uses_channel {
        out.push_str("import type { Channel } from '@tauri-apps/api/core';\n");
    }
    for u in uses.iter().filter(|u| u.as_str() != "Channel") {
        out.push_str(&format!("import type {{ {u} }} from './{u}';\n"));
    }
    out.push_str("\n/** Tên lệnh snake_case; khoá đối số camelCase; `Channel` nhận sự kiện đẩy. */\nexport interface TauriCommandMap {\n");
    out.push_str(&body);
    out.push_str("}\n\nexport type CommandName = keyof TauriCommandMap;\n");
    out
}

pub fn commands_ts() -> String {
    commands_ts_for(COMMANDS)
}

/// `index.ts`: re-export mọi file kiểu ts-rs đã xuất (trừ `commands.ts`, `index.ts`).
pub fn index_ts(dir: &Path) -> String {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter_map(|n| n.strip_suffix(".ts").map(String::from))
                .filter(|n| n != "commands" && n != "index")
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut out = String::from("// Sinh tự động. Không sửa tay.\nexport type * from './commands';\n");
    for n in names {
        out.push_str(&format!("export type {{ {n} }} from './{n}';\n"));
    }
    out
}
