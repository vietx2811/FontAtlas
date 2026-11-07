//! Hợp đồng TS: bản đồ lệnh, kiểu DTO (ts-rs ghi khi `cargo test`), fixture JSON.
//! Cập nhật: `UPDATE_BINDINGS=1 cargo test -p app --test bindings`.
use std::path::PathBuf;

use app_lib::bindings::{bindings_dir, commands_ts, index_ts};
use app_lib::commands::COMMANDS;
use app_lib::dto::activation as ad;
use app_lib::dto::analysis::*;
use app_lib::dto::events::*;
use app_lib::dto::library::*;
use app_lib::dto::model::*;
use app_lib::dto::settings::*;
use app_lib::error::CmdError;
use serde_json::{Value, json};

fn updating() -> bool {
    std::env::var("UPDATE_BINDINGS").as_deref() == Ok("1")
}

fn check_file(path: PathBuf, content: String) {
    if updating() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &content).unwrap();
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("thiếu {}; chạy với UPDATE_BINDINGS=1", path.display()));
    assert_eq!(
        content.trim_end(),
        want.trim_end(),
        "{} lệch; chạy lại với UPDATE_BINDINGS=1",
        path.display()
    );
}

#[test]
fn bindings_up_to_date() {
    // ts-rs ghi các file kiểu khi chạy test của chính nó; commands.ts/index.ts sinh ở đây
    check_file(bindings_dir().join("commands.ts"), commands_ts());
    check_file(bindings_dir().join("index.ts"), index_ts(&bindings_dir()));
}

#[test]
fn every_command_in_handler_is_in_map() {
    use tauri::Manager;
    use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
    let d = tempfile::tempdir().unwrap();
    let st = app_lib::state::AppState::new(
        app_lib::state::AppDirs::under(d.path()),
        Box::new(app_lib::services::activation::NoActivation),
    );
    let app = mock_builder()
        .manage(st)
        .invoke_handler(app_lib::app_handler!())
        .build(mock_context(noop_assets()))
        .unwrap();
    let w = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap()
    });
    let map = commands_ts();
    for c in COMMANDS {
        assert!(
            map.contains(&format!("  {}: {{ args:", c.name)),
            "{} thiếu trong commands.ts",
            c.name
        );
        let req = tauri::webview::InvokeRequest {
            cmd: c.name.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({})),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        };
        let r = get_ipc_response(&w, req);
        let msg = match &r {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        };
        assert!(
            !msg.contains("not found"),
            "{} chưa có trong invoke_handler: {msg}",
            c.name
        );
    }
    assert!(COMMANDS.len() > 60);
}

// ---- fixture dựng tay (giá trị đại diện, tất định) ----

fn key(h: u8) -> FaceKeyDto {
    FaceKeyDto {
        hash_hex: format!("{h:02x}").repeat(32),
        face_index: 0,
    }
}

fn row(id: i64, h: u8, family: &str, sub: &str, w: u16, italic: bool) -> FaceRowDto {
    FaceRowDto {
        face_id: id,
        hash_hex: key(h).hash_hex,
        face_index: 0,
        family: family.into(),
        subfamily: Some(sub.into()),
        full_name: Some(format!("{family} {sub}")),
        weight_class: Some(w),
        width_class: Some(5),
        is_italic: italic,
        is_variable: false,
        vi_status: "full".into(),
        supports_vi: true,
        vi_missing: 0,
        license_class: "ofl".into(),
        path: Some(format!(
            "/Users/u/Fonts/{}-{sub}.ttf",
            family.replace(' ', "")
        )),
        file_size: 176_000 + id as u64,
        missing: false,
        is_favorite: id == 1,
        added_at: 1_700_000_000_000 + id,
    }
}

fn reading(k: &str, v: Option<Value>, q: &str, p: Option<f32>) -> AxisReadingDto {
    AxisReadingDto {
        key: k.into(),
        raw_value: v.as_ref().and_then(Value::as_f64),
        unit: app_lib::services::analysis::axis_unit(k).map(String::from),
        value: v,
        quality: q.into(),
        reason: (q == "failed").then(|| "missing_glyph".into()),
        method: Some("method".into()),
        percentile: p,
    }
}

fn detail() -> FaceDetailDto {
    FaceDetailDto {
        row: row(3, 3, "Playfair Display", "Regular", 400, false),
        summary: "Serif, tương phản cao, x-height thấp. Đủ 134 ký tự Việt.".into(),
        classification: Some("Serif".into()),
        classification_info: ClassificationDto {
            class: "serif".into(),
            label: "Serif".into(),
            subclass: Some("didone".into()),
            confidence: 0.91,
            candidate: None,
            traits: vec!["decorative".into()],
            signals: vec![ClassSignalDto {
                source: "measure".into(),
                detail: "contrast 3.4".into(),
                vote: "serif".into(),
                weight: 1.5,
            }],
            usage: vec!["display".into()],
            source: "auto".into(),
            auto_class: None,
        },
        source: "axes_and_image".into(),
        has_image: true,
        file: FileInfoDto {
            format: "ttf".into(),
            container: Some("sfnt".into()),
            outline: "glyf".into(),
            num_glyphs: 1230,
            size_bytes: 179_000,
            path: Some("/Users/u/Fonts/PlayfairDisplay-Regular.ttf".into()),
        },
        metadata: MetadataDto {
            license: Some(
                "This Font Software is licensed under the SIL Open Font License, Version 1.1."
                    .into(),
            ),
            license_url: Some("https://openfontlicense.org".into()),
            designer: Some("Claus Eggers Sørensen".into()),
            manufacturer: None,
            version: Some("Version 1.203".into()),
            copyright: Some("Copyright 2017 The Playfair Display Project Authors".into()),
            vendor_url: None,
            variable_axes: vec![VariableAxisDto {
                tag: "wght".into(),
                min: 400.0,
                default: 400.0,
                max: 900.0,
            }],
            named_instances: vec![InstanceDto {
                name: "Bold".into(),
                coords: vec![AxisCoordDto {
                    tag: "wght".into(),
                    value: 700.0,
                }],
            }],
            scripts: vec!["cyrl".into(), "latn".into()],
            opsz_range: None,
            features: FeaturesDto {
                gsub: vec!["liga".into(), "ccmp".into()],
                gpos: vec!["kern".into(), "mark".into()],
            },
        },
        vietnamese: VietnameseDto {
            status: "full".into(),
            supports: true,
            missing: vec![],
            total: 134,
            precomposed_composite: true,
            gpos_mark: true,
            gpos_mkmk: true,
            gsub_ccmp: true,
            dong_sign: false,
            stacked_marks_top_em: Some(1.05),
            stacked_marks_overflow: false,
            stack_style: Some("right".into()),
            mark_weight_ratio: reading("mark_weight_ratio", Some(json!(0.8)), "estimated", None),
        },
        axes: vec![
            reading("contrast", Some(json!(4.9)), "measured", Some(0.94)),
            reading("serif_foot", Some(json!("serif")), "measured", None),
            reading("terminal", None, "failed", None),
        ],
        similar_top: vec![SimilarHitDto {
            face: key(5),
            row: row(5, 5, "Libre Baskerville", "Regular", 400, false),
            distance: 0.21,
            source: SimilarSourceDto::AxesAndImage,
            same: vec!["contrast".into()],
            differ: vec!["width".into()],
        }],
        extras: extras(),
    }
}

fn extras() -> FaceExtrasDto {
    FaceExtrasDto {
        vendor_id: Some("FTH".into()),
        years: YearsDto { created: Some(2017), modified: Some(2021), copyright: Some(2017) },
        ot_groups: vec![
            OtGroupDto { group: "figures".into(), tags: vec!["lnum".into(), "onum".into()], sets: vec![] },
            OtGroupDto {
                group: "stylistic_sets".into(),
                tags: vec!["ss01".into(), "ss02".into()],
                sets: vec![StylisticSetDto { tag: "ss01".into(), name: Some("Alternate a".into()) }, StylisticSetDto { tag: "ss02".into(), name: None }],
            },
        ],
        kerning: KerningDto { source: "gpos".into(), pairs: 8780, class_pairs: 3417 },
        style: StyleStructureDto { italic_kind: "upright".into(), ital_axis: false, slnt_axis: false, wght_range: Some([400.0, 900.0]), wdth_range: None, opsz_range: None },
        embedding: EmbeddingDto { fs_type: Some(0), level: "installable".into(), flags: vec![] },
        license_kind: "ofl".into(),
    }
}

fn compare() -> CompareDto {
    let d = detail();
    let axis = |key: &str, v: f64, base: f64, i: usize| CompareAxisDto {
        key: key.into(),
        unit: Some("ratio".into()),
        raw_value: Some(v),
        quality: if key == "terminal" { "estimated".into() } else { "measured".into() },
        estimated: key == "terminal",
        percentile: Some(0.9),
        delta: (i > 0).then_some(v - base),
        delta_pct: (i > 0).then_some((v - base) / base * 100.0),
    };
    let face = |i: usize| CompareFaceDto {
        face: key(3 + i as u8),
        row: row(3 + i as i64, 3 + i as u8, "Playfair Display", "Regular", 400, false),
        is_baseline: i == 0,
        classification: d.classification_info.clone(),
        axes: vec![axis("contrast", 4.9 - 2.0 * i as f64, 4.9, i), axis("terminal", 1.6, 1.6, i)],
        vietnamese: CompareVietnameseDto {
            supports: true,
            missing: 0,
            total: 134,
            stacked_marks_top_em: Some(1.05),
            hhea_ascender_em: Some(1.082),
            cap_height_em: Some(0.708),
            over_ascender_pct_em: Some(0.0),
            over_cap_pct_em: Some(34.2),
            overflow: false,
            stack_style: Some("right".into()),
        },
        license: LicenseNoteDto {
            kind: "ofl".into(),
            description: d.metadata.license.clone(),
            url: d.metadata.license_url.clone(),
            embedding: extras().embedding,
        },
        extras: extras(),
        family: FamilyStructureDto { family: "Playfair Display".into(), faces: 6, weights: vec![400, 700, 900], widths: vec![5], has_italic: true, has_oblique: false, has_variable: true, has_opsz: false },
    };
    CompareDto { baseline: key(3), faces: vec![face(0), face(1)] }
}

fn candidate() -> PairCandidateDto {
    PairCandidateDto {
        face: key(2),
        row: row(2, 2, "Roboto", "Regular", 400, false),
        gate: "controlled".into(),
        labels: vec![PairLabelDto {
            axis: "contrast".into(),
            relation: "contrast".into(),
            symbol: "◐".into(),
            delta: Some(3.6),
            a: Some(4.9),
            b: Some(1.3),
            role_reversed: false,
        }],
        clashes: vec![],
        body_warnings: vec!["low_x_height".into()],
        heading_overflow: false,
        insufficient_data: false,
        weight_hint: Some("0,14".into()),
        block: None,
        sentences: vec!["tương phản nét chênh rõ (4,89 và 1,30)".into()],
        summary: "Sans. Đủ 134 ký tự Việt.".into(),
    }
}

fn fixtures() -> Vec<(&'static str, Value)> {
    let v = |x: &dyn erased::Ser| x.to_value();
    let page = FacePageDto {
        total: 2,
        offset: 0,
        items: vec![
            row(1, 1, "Open Sans", "Regular", 400, false),
            row(2, 2, "Roboto", "Regular", 400, false),
        ],
    };
    let model = ModelStatusDto {
        state: ModelState::Paused,
        done_bytes: 148_000_000,
        total_bytes: 371_000_000,
        bytes_per_sec: 0.0,
        eta_ms: None,
        location: "/Users/u/Library/Application Support/com.vietx.fontatlas/models".into(),
        disk_free: 52_000_000_000,
        required_bytes: 408_000_000,
        last_error: Some(ErrorInfoDto {
            code: "io".into(),
            message: "mạng đứt".into(),
        }),
    };
    vec![
        ("face_page", v(&page)),
        ("face_detail_playfair", v(&detail())),
        ("compare_two", v(&compare())),
        ("pair_these_combos", v(&vec![PairComboDto { heading: row(3, 3, "Playfair Display", "Regular", 400, false), body: row(5, 5, "Libre Baskerville", "Regular", 400, false), assessment: candidate(), rank: 1 }])),
        ("similar_hits", v(&vec![detail().similar_top[0].clone()])),
        (
            "similar_families",
            v(&SimilarFamilyPageDto {
                total: 1,
                offset: 0,
                items: vec![SimilarFamilyDto {
                    family: "Playfair Display".into(),
                    representative: detail().similar_top[0].clone(),
                    faces: vec![detail().similar_top[0].face.clone()],
                    count: 1,
                }],
            }),
        ),
        (
            "axis_stats",
            v(&vec![AxisStatsDto {
                axis: "width".into(),
                unit: "x_height_ratio".into(),
                count: 2,
                min: Some(0.8),
                max: Some(1.2),
                p1: Some(0.804),
                p99: Some(1.196),
            }]),
        ),
        ("pair_candidates", v(&vec![candidate()])),
        (
            "saved_pairs",
            v(&vec![SavedPairDto {
                id: 1,
                a: key(3),
                a_role: "heading".into(),
                b: key(2),
                b_role: "body".into(),
                mode: Some("safe".into()),
                preset: Some("poster".into()),
                sample_text: Some("Phở bò Hà Nội".into()),
                collection_id: Some(4),
                note: None,
                created_at: 1_700_000_000_000,
            }]),
        ),
        (
            "describe_warm",
            v(&DescribeDto {
                dictionary_version: "0.1.0-draft".into(),
                matched: vec![MatchedTermDto {
                    id: "warm".into(),
                    text: "ấm".into(),
                    group: "tone".into(),
                    subjectivity: "medium".into(),
                    culture: "intl".into(),
                    rationale_vi: "Độ mở rộng và đầu nét mềm.".into(),
                    rationale_en: "Open apertures.".into(),
                    explanation_banned: false,
                }],
                filters: vec![AxisFilterDto {
                    axis: "aperture".into(),
                    range: Some([0.55, 1.0]),
                    equals: None,
                    strength: StrengthDto::Preferred,
                    weight: 1.0,
                }],
                hard: vec!["vi_supported".into()],
                conflicts: vec![DescribeConflictDto {
                    axis: "contrast".into(),
                    term_a: "luxury".into(),
                    term_b: "casual".into(),
                    range_a: Some([0.7, 1.0]),
                    range_b: Some([0.0, 0.4]),
                }],
                unmappable: vec![("contemporary".into(), "Cần metadata năm.".into())],
                partial: vec![],
                unknown_words: vec!["xyz".into()],
            }),
        ),
        (
            "settings",
            v(&SettingsDto {
                embed_enabled: true,
                license_notice_ack: false,
                ui_lang: "vi".into(),
                sample_text: "Phở bò Hà Nội".into(),
                project_vi_default: true,
                keep_active_on_exit: false,
                rescan_on_launch: true,
            }),
        ),
        ("ui_state", v(&UiStateDto::default())),
        (
            "sources",
            v(&vec![SourceDto {
                id: 1,
                kind: "directory".into(),
                name: "Fonts".into(),
                path: Some("/Users/u/Fonts".into()),
                enabled: true,
                available: true,
                face_count: 1200,
                last_scan: None,
                scanning: false,
            }]),
        ),
        (
            "scan_events",
            v(&vec![
                ScanEvent::Queued { source_id: 1 },
                ScanEvent::Progress {
                    source_id: 1,
                    progress: ScanProgressDto {
                        phase: ScanPhaseDto::Analyzing,
                        done: 256,
                        total: 1200,
                        faces_written: 256,
                        not_downloaded: 3,
                    },
                },
                ScanEvent::FacesAdded {
                    source_id: 1,
                    count: 256,
                },
                ScanEvent::Finished {
                    source_id: 1,
                    stats: RescanStatsDto {
                        status: RescanStatusDto::Ok,
                        seen: 1200,
                        unchanged: 0,
                        not_downloaded: 3,
                        relinked: 0,
                        analyzed: 1200,
                        failed: 2,
                        missing: 0,
                        unreadable: vec![],
                    },
                },
                ScanEvent::Idle,
            ]),
        ),
        (
            "boot_status",
            v(&BootStatusDto {
                phase: BootPhase::Ready,
                store_error: None,
                library_recreated: false,
                user_backup: None,
                legacy: LegacyStateDto {
                    found: true,
                    preview: Some(LegacyPreviewDto {
                        fonts: 300,
                        sources: 2,
                        collections: 3,
                        favorites: 12,
                        tags: 5,
                    }),
                    ..Default::default()
                },
            }),
        ),
        (
            "onboarding",
            v(&OnboardingDto {
                step: Some("o3".into()),
                has_sources: false,
                legacy: LegacyStateDto::default(),
                license_notice_ack: false,
                embed_enabled: true,
                model_state: "missing".into(),
                activation_supported: true,
                activation_onboarded: false,
                keep_active_on_exit: false,
                font_managers: vec![FontManagerDto {
                    name: "RightFont".into(),
                    installed: true,
                    active_count: None,
                }],
            }),
        ),
        ("model_status_paused", v(&model)),
        (
            "embed_status",
            v(&EmbedStatusDto {
                enabled: true,
                running: true,
                paused: false,
                pending: 1500,
                done: 300,
                failed: 1,
                device: Some("metal".into()),
                ms_per_face: Some(28.0),
                eta_ms: Some(42_000),
                model_version: Some("siglip2-base-p16-224@75de2d5|white-v2".into()),
                load_error: None,
            }),
        ),
        (
            "font_managers",
            v(&vec![FontManagerDto {
                name: "FontBase".into(),
                installed: false,
                active_count: None,
            }]),
        ),
        (
            "family_styles_public_sans",
            v(&vec![FamilyStyleDto {
                face: key(9),
                subfamily: Some("Bold".into()),
                weight: Some(700),
                italic: false,
                instance: Some(InstanceDto {
                    name: "Bold".into(),
                    coords: vec![AxisCoordDto {
                        tag: "wght".into(),
                        value: 700.0,
                    }],
                }),
            }]),
        ),
        (
            "cmd_error_nested_source",
            v(
                &CmdError::new("nested_source", "đường dẫn lồng với nguồn #1")
                    .with_detail(json!({ "existingId": 1, "existingPath": "/Users/u/Fonts" })),
            ),
        ),
        (
            "model_events",
            v(&vec![
                ModelEvent::Progress {
                    done: 1,
                    total: 2,
                    bytes_per_sec: 10.0,
                    eta_ms: Some(100),
                    resumed: true,
                },
                ModelEvent::Ready,
            ]),
        ),
        (
            "embed_events",
            v(&vec![EmbedEvent::Finished { total_done: 5 }]),
        ),
    ]
}

mod erased {
    use serde::Serialize;
    pub trait Ser {
        fn to_value(&self) -> serde_json::Value;
    }
    impl<T: Serialize> Ser for T {
        fn to_value(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap()
        }
    }
}

#[test]
fn fixtures_match_dtos() {
    let names: Vec<&str> = fixtures().iter().map(|f| f.0).collect();
    assert!(names.len() >= 19);
    for (name, value) in fixtures() {
        check_file(
            bindings_dir().join("fixtures").join(format!("{name}.json")),
            serde_json::to_string_pretty(&value).unwrap() + "\n",
        );
    }
}

#[test]
fn activation_dtos_match_plan7_contract_fixtures() {
    use fontatlas_activate as fa;
    use fontatlas_store::{ActivationReason, ActivationStatus, DesiredState};
    let p7 = |n: &str| -> Value {
        serde_json::from_str(
            &std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("crates/fontatlas-activate/tests/fixtures")
                    .join(n),
            )
            .unwrap(),
        )
        .unwrap()
    };
    let result = |i: u8, o: fa::FileOutcome| fa::FileResult {
        hash_hex: format!("{i:02x}").repeat(32),
        path: Some(format!("/Users/u/Fonts/f{i}.ttf")),
        requested_faces: vec![0],
        extra_faces: if i == 1 { vec![1, 2] } else { vec![] },
        outcome: o,
    };
    let conflict = fa::Conflict {
        postscript_name: "LibreBaskerville-Regular".into(),
        existing_path: "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/L/x.ttf".into(),
        owner: fa::Owner::RightFont,
    };
    use fa::FileOutcome as F;
    let outcomes = vec![
        F::Ready,
        F::Activated,
        F::AlreadyActive,
        F::Deactivated,
        F::AlreadyInactive,
        F::Busy,
        F::NeedsConfirmation {
            conflicts: vec![conflict],
        },
        F::HeldByCollections {
            collection_ids: vec![3, 7],
        },
        F::KeptActive {
            manual: true,
            collection_ids: vec![7],
        },
        F::MissingFile,
        F::NotDownloaded,
        F::Failed {
            code: "engine.rejected".into(),
            message: "hệ điều hành từ chối (999): x".into(),
        },
    ];
    let rep = fa::ActivationReport {
        results: outcomes
            .into_iter()
            .enumerate()
            .map(|(i, o)| result(i as u8, o))
            .collect(),
        changed: 2,
        needs_confirmation: 1,
        failed: 1,
    };
    assert_eq!(
        serde_json::to_value(ad::ActivationReportDto::from(&rep)).unwrap(),
        p7("activation_report.json")
    );
    let rec = fa::ReconcileReport {
        auto_deactivated: vec![fa::AutoDeactivated {
            hash_hex: "cd".repeat(32),
            faces: vec![0],
            removed_reasons: vec![ActivationReason::Collection(5)],
            cause: "collection_deleted".into(),
        }],
        auto_deactivate_failed: vec![fa::AutoDeactivated {
            hash_hex: "ef".repeat(32),
            faces: vec![0, 1],
            removed_reasons: vec![],
            cause: "no_reason".into(),
        }],
        reregistered: 3,
        moved: 1,
        missing: 1,
        not_downloaded: 0,
        cleaned: 1,
        failed: 0,
        results: vec![result(9, F::Activated)],
    };
    assert_eq!(
        serde_json::to_value(ad::ReconcileReportDto::from(&rec)).unwrap(),
        p7("reconcile_report.json")
    );
    let st = fa::ActivationStateDto {
        hash_hex: "ab".repeat(32),
        state: DesiredState::Active,
        scope: fa::Scope::Session,
        status: ActivationStatus::Registered,
        reasons: vec![ActivationReason::Manual, ActivationReason::Collection(3)],
        active_path: Some("/Users/u/Fonts/Phở Đậm.ttf".into()),
        conflict_ps: None,
        last_error: None,
    };
    // DTO của app = hợp đồng Plan 7 + `faces`
    let mut got =
        serde_json::to_value(vec![ad::ActivationStateDto::from_core(st, vec![0])]).unwrap();
    got[0].as_object_mut().unwrap().remove("faces");
    assert_eq!(got, p7("activation_state.json"));
    // fixture phía TS: bản Plan 7 + `faces`
    for n in ["activation_report.json", "reconcile_report.json"] {
        check_file(
            bindings_dir().join("fixtures").join(n),
            serde_json::to_string_pretty(&p7(n)).unwrap() + "\n",
        );
    }
    let mut ts_state = p7("activation_state.json");
    ts_state[0]
        .as_object_mut()
        .unwrap()
        .insert("faces".into(), json!([0]));
    check_file(
        bindings_dir()
            .join("fixtures")
            .join("activation_state.json"),
        serde_json::to_string_pretty(&ts_state).unwrap() + "\n",
    );
}

#[test]
fn dto_serializes_like_core() {
    use fontatlas_store::{
        LegacyImport, LegacyPreview, LegacyResolve, RescanStats, RescanStatus, ScanPhase,
        ScanProgress,
    };
    let prog = ScanProgress {
        phase: ScanPhase::Hashing,
        done: 3,
        total: 9,
        faces_written: 2,
        not_downloaded: 4,
    };
    assert_eq!(
        serde_json::to_value(&prog).unwrap(),
        serde_json::to_value(ScanProgressDto::from(&prog)).unwrap()
    );
    let st = RescanStats {
        status: RescanStatus::SourceUnavailable,
        seen: 5,
        unchanged: 1,
        not_downloaded: 7,
        relinked: 2,
        analyzed: 3,
        failed: 4,
        missing: 6,
        unreadable: vec![("a".into(), "io: x".into())],
    };
    assert_eq!(
        serde_json::to_value(&st).unwrap(),
        serde_json::to_value(RescanStatsDto::from(&st)).unwrap()
    );
    let imp = LegacyImport {
        fonts: 1,
        sources: 2,
        collections: 3,
        recovered_collections: 4,
        bad_tags: 5,
        skipped_sources: 6,
    };
    assert_eq!(
        serde_json::to_value(&imp).unwrap(),
        serde_json::to_value(LegacyImportDto::from(imp.clone())).unwrap()
    );
    let rs = LegacyResolve {
        resolved: 1,
        unresolved: 2,
        ambiguous_faces: 3,
    };
    assert_eq!(
        serde_json::to_value(&rs).unwrap(),
        serde_json::to_value(LegacyResolveDto::from(rs.clone())).unwrap()
    );
    let pv = LegacyPreview {
        fonts: 1,
        sources: 2,
        collections: 3,
        favorites: 4,
        tags: 5,
    };
    assert_eq!(
        serde_json::to_value(&pv).unwrap(),
        serde_json::to_value(LegacyPreviewDto::from(pv.clone())).unwrap()
    );
}

#[test]
fn fixture_dtos_deserialize_for_request_types() {
    let q: FaceQueryDto = serde_json::from_value(json!({ "text": "ro", "sort": "match", "faceKeys": [{ "hashHex": "ab".repeat(32), "faceIndex": 0 }], "axisFilters": [] })).unwrap();
    assert_eq!(q.sort, SortKeyDto::Match);
    let s: SimilarOptsDto = serde_json::from_value(json!({ "k": 5 })).unwrap();
    assert!(s.use_image && s.k == 5);
    let p: SettingsPatchDto = serde_json::from_value(json!({ "uiLang": "en" })).unwrap();
    assert_eq!(p.ui_lang.as_deref(), Some("en"));
}
