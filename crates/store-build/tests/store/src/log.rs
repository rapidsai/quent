// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "ui")]
use quent_store::events::Uuid;

#[allow(unused_imports, dead_code)]
mod logs {
    include!(concat!(env!("OUT_DIR"), "/log.rs"));
}
#[allow(unused_imports, dead_code)]
mod log_events_only {
    include!(concat!(env!("OUT_DIR"), "/log_events_only.rs"));
}

#[cfg(feature = "ui")]
#[test]
fn log_projection_preserves_structured_fields() {
    use quent_log::analysis::LogEventPayload;
    use quent_log::ui::{
        DynamicAttribute, DynamicList, DynamicStruct, DynamicValue, LogAttributes,
    };
    let payload = logs::LogEvent::Critical {
        message: "failed".into(),
        http_status: 503,
        flag: true,
        id: Uuid::from_u128(7),
        optional: Some("present".into()),
        missing: None,
        nested_option: Some(Some(8)),
        record: logs::Context {
            r#type: false,
            memory: u64::MAX,
        },
        dynamic: vec![DynamicAttribute::string("key", "value")].into(),
        numbers: vec![-1, 2],
        strings: vec!["text".into()],
        flags: vec![true, false],
        ids: vec![Uuid::nil()],
        records: vec![logs::Context {
            r#type: true,
            memory: 1,
        }],
        dynamics: vec![vec![DynamicAttribute::null("nil")].into()],
        matrix: vec![vec![1, 2]],
        nullable: vec![Some("yes".into()), None],
        reference: logs::EntityRef::new(
            Uuid::from_u128(9),
            logs::Context {
                r#type: true,
                memory: 42,
            },
        ),
        bare_reference: logs::EntityRef::new(Uuid::from_u128(10), ()),
        references: vec![logs::EntityRef::new(Uuid::from_u128(11), ())],
        error_code: 17,
    };
    let message = payload.log_message().unwrap();
    assert_eq!(
        (message.message, message.level.name, message.level.rank),
        ("failed", "critical", 1)
    );
    let attributes = payload.log_attributes();
    let get = |key: &str| {
        &attributes
            .iter()
            .find(|attribute| attribute.key == key)
            .unwrap()
            .value
    };
    assert!(
        !attributes
            .iter()
            .any(|attribute| attribute.key == "message")
    );
    assert_eq!(*get("HTTPStatus"), Some(DynamicValue::U16(503)));
    assert_eq!(*get("flag"), Some(DynamicValue::U8(1)));
    assert_eq!(
        *get("id"),
        Some(DynamicValue::String(Uuid::from_u128(7).to_string()))
    );
    assert_eq!(*get("missing"), None);
    assert_eq!(*get("nested_option"), Some(DynamicValue::U8(8)));
    assert_eq!(
        *get("numbers"),
        Some(DynamicValue::List(DynamicList::I64(vec![-1, 2])))
    );
    assert_eq!(
        *get("matrix"),
        Some(DynamicValue::List(DynamicList::List(vec![
            DynamicList::U32(vec![1, 2])
        ])))
    );
    assert_eq!(
        *get("nullable"),
        Some(DynamicValue::List(DynamicList::Struct(vec![
            DynamicStruct(vec![DynamicAttribute::string("value", "yes")]),
            DynamicStruct(vec![DynamicAttribute::null("value")]),
        ])))
    );
    assert_eq!(
        *get("record"),
        Some(DynamicValue::Struct(DynamicStruct(vec![
            DynamicAttribute::u8("type", 0),
            DynamicAttribute::u64("memory", u64::MAX),
        ])))
    );
    assert!(matches!(get("reference"), Some(DynamicValue::Struct(value)) if value.0.len() == 2));
    assert!(logs::LogEvent::Initialized.log_message().is_none());
    assert!(logs::LogEvent::Initialized.log_attributes().is_empty());
    assert_eq!(log_events_only::LogEvent::LEVELS[1].name, "critical");
}

#[allow(unused_imports, dead_code)]
mod namespaced {
    include!(concat!(env!("OUT_DIR"), "/namespaced_log.rs"));
}

#[cfg(feature = "analysis")]
#[test]
fn namespaced_log_mapping_compiles() {
    use quent_log::analysis::LogEventPayload;
    #[cfg(feature = "ui")]
    use quent_log::ui::LogAttributes;
    let event = namespaced::nested::LogEvent::Critical {
        message: "critical".into(),
    };
    assert_eq!(event.log_message().unwrap().level.rank, 1);
    #[cfg(feature = "ui")]
    assert!(event.log_attributes().is_empty());
}
