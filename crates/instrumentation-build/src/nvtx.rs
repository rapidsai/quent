// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generated accessors from canonical schema NVTX types to shared analysis.

use proc_macro2::TokenStream;
use quent_constraints::Constraint as _;
use quent_fsm::FsmConstraint;
use quent_schema::{Cardinality, DataType, Identifier, Path, Schema};
use quote::quote;

use crate::common::relative_type_path;
use crate::{GenerateError, Options};

/// The selected process event that activates a generated private NVTX source.
pub(crate) struct CaptureConfig {
    pub(crate) process: Path,
    pub(crate) process_event: Identifier,
    pub(crate) process_field: Identifier,
}

/// Resolve live-capture generation against the validated canonical schema.
pub(crate) fn capture_config(
    schema: &Schema,
    opts: &Options,
) -> Result<Option<CaptureConfig>, GenerateError> {
    if !opts.nvtx_capture {
        return Ok(None);
    }
    let Some(bindings) = nvtx_schema::validated_bindings(schema)? else {
        return Ok(None);
    };
    if !opts.instrumentation {
        return Err(GenerateError::NvtxCaptureRequiresInstrumentation);
    }

    let process = schema
        .entity(&bindings.process_target)
        .expect("validated NVTX binding targets an existing process entity");
    if process.annotations().has_constraint(FsmConstraint::NAME) {
        return Err(GenerateError::NvtxCaptureFsmProcessUnsupported {
            entity: bindings.process_target,
        });
    }
    let process_record = quent_os::process_path();
    let (process_event, process_field) = process
        .events()
        .find_map(|event| {
            event.fields().find_map(|field| {
                (field.ty() == &DataType::Record(process_record.clone())).then_some((event, field))
            })
        })
        .expect("validated OS process entity has an identity-bearing event");
    debug_assert_eq!(process_event.cardinality(), Cardinality::Once);

    Ok(Some(CaptureConfig {
        process: bindings.process_target,
        process_event: process_event.name().clone(),
        process_field: process_field.name().clone(),
    }))
}

/// Generate analysis bindings when the canonical NVTX extension is present.
pub(crate) fn generate_bindings(
    schema: &Schema,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let Some(bindings) = nvtx_schema::validated_bindings(schema)? else {
        return Ok(TokenStream::new());
    };

    let message = relative_type_path(bindings.records.message.path(), &[], "");
    let color = relative_type_path(bindings.records.color.path(), &[], "");
    let payload = relative_type_path(bindings.records.payload.path(), &[], "");
    let attributes = relative_type_path(bindings.records.attributes.path(), &[], "");
    let event = relative_type_path(bindings.entity.path(), &[], "Event");

    let message_string = nvtx_schema::MESSAGE_KIND_STRING;
    let message_registered = nvtx_schema::MESSAGE_KIND_REGISTERED_HANDLE;
    let payload_unsigned_64 = nvtx_schema::PAYLOAD_VALUE_KIND_UNSIGNED_INT64;
    let payload_signed_64 = nvtx_schema::PAYLOAD_VALUE_KIND_INT64;
    let payload_double = nvtx_schema::PAYLOAD_VALUE_KIND_DOUBLE;
    let payload_unsigned_32 = nvtx_schema::PAYLOAD_VALUE_KIND_UNSIGNED_INT32;
    let payload_signed_32 = nvtx_schema::PAYLOAD_VALUE_KIND_INT32;
    let payload_float = nvtx_schema::PAYLOAD_VALUE_KIND_FLOAT;
    let payload_pointer = nvtx_schema::PAYLOAD_VALUE_KIND_POINTER;

    let capture = opts.instrumentation.then(|| {
        quote! {
            impl ::core::convert::From<::nvtx_events::NvtxColor> for #color {
                fn from(value: ::nvtx_events::NvtxColor) -> Self {
                    Self {
                        color_type: value.color_type,
                        value: value.value,
                    }
                }
            }

            impl ::core::convert::From<::nvtx_events::NvtxMessage> for #message {
                fn from(value: ::nvtx_events::NvtxMessage) -> Self {
                    match value {
                        ::nvtx_events::NvtxMessage::String(string) => Self {
                            kind: #message_string,
                            string: Some(string),
                            registered_handle: None,
                        },
                        ::nvtx_events::NvtxMessage::RegisteredHandle(handle) => Self {
                            kind: #message_registered,
                            string: None,
                            registered_handle: Some(handle),
                        },
                    }
                }
            }

            impl ::core::convert::From<::nvtx_events::NvtxPayload> for #payload {
                fn from(value: ::nvtx_events::NvtxPayload) -> Self {
                    let (value_kind, value_bits) = match value.value {
                        ::nvtx_events::NvtxPayloadValue::UnsignedInt64(value) =>
                            (#payload_unsigned_64, value),
                        ::nvtx_events::NvtxPayloadValue::Int64(value) =>
                            (#payload_signed_64, value as u64),
                        ::nvtx_events::NvtxPayloadValue::Double(value) =>
                            (#payload_double, value.to_bits()),
                        ::nvtx_events::NvtxPayloadValue::UnsignedInt32(value) =>
                            (#payload_unsigned_32, u64::from(value)),
                        ::nvtx_events::NvtxPayloadValue::Int32(value) =>
                            (#payload_signed_32, u64::from(value as u32)),
                        ::nvtx_events::NvtxPayloadValue::Float(value) =>
                            (#payload_float, u64::from(value.to_bits())),
                        ::nvtx_events::NvtxPayloadValue::Pointer(value) =>
                            (#payload_pointer, value),
                    };
                    Self {
                        payload_type: value.payload_type,
                        value_kind,
                        value_bits,
                    }
                }
            }

            impl ::core::convert::From<::nvtx_events::NvtxEventAttributes> for #attributes {
                fn from(value: ::nvtx_events::NvtxEventAttributes) -> Self {
                    Self {
                        category: value.category,
                        color: value.color.map(::core::convert::Into::into),
                        message: value.message.map(::core::convert::Into::into),
                        payload: value.payload.map(::core::convert::Into::into),
                    }
                }
            }

            impl ::core::convert::From<::nvtx_events::NvtxEvent> for #event {
                fn from(value: ::nvtx_events::NvtxEvent) -> Self {
                    match value {
                        ::nvtx_events::NvtxEvent::RangePush {
                            domain,
                            thread_id,
                            attributes,
                        } => Self::RangePush {
                            domain,
                            thread_id,
                            attributes: attributes.into(),
                        },
                        ::nvtx_events::NvtxEvent::RangePop { domain, thread_id } =>
                            Self::RangePop { domain, thread_id },
                        ::nvtx_events::NvtxEvent::RangeStart {
                            domain,
                            range_id,
                            attributes,
                        } => Self::RangeStart {
                            domain,
                            range_id,
                            attributes: attributes.into(),
                        },
                        ::nvtx_events::NvtxEvent::RangeEnd { domain, range_id } =>
                            Self::RangeEnd { domain, range_id },
                        ::nvtx_events::NvtxEvent::Mark { domain, attributes } => Self::Mark {
                            domain,
                            attributes: attributes.into(),
                        },
                        ::nvtx_events::NvtxEvent::DomainCreate { domain, name } =>
                            Self::DomainCreate { domain, name },
                        ::nvtx_events::NvtxEvent::DomainDestroy { domain } =>
                            Self::DomainDestroy { domain },
                        ::nvtx_events::NvtxEvent::RegisterString {
                            domain,
                            handle,
                            string,
                        } => Self::RegisterString { domain, handle, string },
                        ::nvtx_events::NvtxEvent::NameCategory {
                            domain,
                            category,
                            name,
                        } => Self::NameCategory { domain, category, name },
                        ::nvtx_events::NvtxEvent::NameThread { thread_id, name } =>
                            Self::NameThread { thread_id, name },
                        ::nvtx_events::NvtxEvent::ResourceCreate {
                            domain,
                            handle,
                            identifier_type,
                            identifier,
                            message,
                        } => Self::ResourceCreate {
                            domain,
                            handle,
                            identifier_type,
                            identifier,
                            message: message.map(::core::convert::Into::into),
                        },
                        ::nvtx_events::NvtxEvent::ResourceDestroy { handle } =>
                            Self::ResourceDestroy { handle },
                    }
                }
            }

        }
    });

    let capture_platform_guard = opts.nvtx_capture.then(|| {
        quote! {
            #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
            compile_error!("generated NVTX live capture supports 64-bit Linux only");
        }
    });

    Ok(quote! {
        impl ::nvtx_analyzer::NvtxProcessBindingData for #event {
            fn nvtx_process_id(&self) -> Option<::nvtx_analyzer::Uuid> {
                match self {
                    Self::Initialized { process } => Some(process.target),
                    _ => None,
                }
            }
        }

        impl ::nvtx_analyzer::NvtxMessageData for #message {
            fn nvtx_message(&self) -> Option<::nvtx_analyzer::NvtxMessageView<'_>> {
                match (self.kind, self.string.as_deref(), self.registered_handle) {
                    (#message_string, Some(string), None) => Some(
                        ::nvtx_analyzer::NvtxMessageView::String(string),
                    ),
                    (#message_registered, None, Some(handle)) => Some(
                        ::nvtx_analyzer::NvtxMessageView::RegisteredHandle(handle),
                    ),
                    // Imported data can be malformed even though generated
                    // producers always set exactly one tagged arm. Drop that
                    // message rather than panic or invent a value.
                    _ => None,
                }
            }
        }

        impl ::nvtx_analyzer::NvtxAttributesData for #attributes {
            fn nvtx_attributes(&self) -> ::nvtx_analyzer::NvtxAttributesView<'_> {
                ::nvtx_analyzer::NvtxAttributesView {
                    category: self.category,
                    color: self.color.as_ref().map(|color| ::nvtx_analyzer::NvtxColor {
                        color_type: color.color_type,
                        value: color.value,
                    }),
                    message: self.message.as_ref().and_then(
                        ::nvtx_analyzer::NvtxMessageData::nvtx_message,
                    ),
                    payload: self.payload.as_ref().map(|payload| {
                        let value = match payload.value_kind {
                            #payload_unsigned_64 =>
                                ::nvtx_analyzer::NvtxPayloadValue::UnsignedInt64(payload.value_bits),
                            #payload_signed_64 =>
                                ::nvtx_analyzer::NvtxPayloadValue::Int64(payload.value_bits as i64),
                            #payload_double =>
                                ::nvtx_analyzer::NvtxPayloadValue::Double(f64::from_bits(payload.value_bits)),
                            #payload_unsigned_32 =>
                                ::nvtx_analyzer::NvtxPayloadValue::UnsignedInt32(payload.value_bits as u32),
                            #payload_signed_32 =>
                                ::nvtx_analyzer::NvtxPayloadValue::Int32(payload.value_bits as u32 as i32),
                            #payload_float =>
                                ::nvtx_analyzer::NvtxPayloadValue::Float(f32::from_bits(payload.value_bits as u32)),
                            #payload_pointer =>
                                ::nvtx_analyzer::NvtxPayloadValue::Pointer(payload.value_bits),
                            // Unknown integration tags retain all raw bits just
                            // like unknown native payload tags at capture time.
                            _ => ::nvtx_analyzer::NvtxPayloadValue::UnsignedInt64(payload.value_bits),
                        };
                        ::nvtx_analyzer::NvtxPayload {
                            payload_type: payload.payload_type,
                            value,
                        }
                    }),
                }
            }
        }

        impl ::nvtx_analyzer::NvtxEventData for #event {
            fn nvtx_event(&self) -> Option<::nvtx_analyzer::NvtxEventView<'_>> {
                use ::nvtx_analyzer::{NvtxAttributesData as _, NvtxMessageData as _};
                Some(match self {
                    Self::Initialized { .. } => return None,
                    Self::RangePush { domain, thread_id, attributes } =>
                        ::nvtx_analyzer::NvtxEventView::RangePush {
                            domain: *domain,
                            thread_id: *thread_id,
                            attributes: attributes.nvtx_attributes(),
                        },
                    Self::RangePop { domain, thread_id } =>
                        ::nvtx_analyzer::NvtxEventView::RangePop {
                            domain: *domain,
                            thread_id: *thread_id,
                        },
                    Self::RangeStart { domain, range_id, attributes } =>
                        ::nvtx_analyzer::NvtxEventView::RangeStart {
                            domain: *domain,
                            range_id: *range_id,
                            attributes: attributes.nvtx_attributes(),
                        },
                    Self::RangeEnd { domain, range_id } =>
                        ::nvtx_analyzer::NvtxEventView::RangeEnd {
                            domain: *domain,
                            range_id: *range_id,
                        },
                    Self::Mark { domain, attributes } =>
                        ::nvtx_analyzer::NvtxEventView::Mark {
                            domain: *domain,
                            attributes: attributes.nvtx_attributes(),
                        },
                    Self::DomainCreate { domain, name } =>
                        ::nvtx_analyzer::NvtxEventView::DomainCreate {
                            domain: *domain,
                            name,
                        },
                    Self::DomainDestroy { domain } =>
                        ::nvtx_analyzer::NvtxEventView::DomainDestroy { domain: *domain },
                    Self::RegisterString { domain, handle, string } =>
                        ::nvtx_analyzer::NvtxEventView::RegisterString {
                            domain: *domain,
                            handle: *handle,
                            string,
                        },
                    Self::NameCategory { domain, category, name } =>
                        ::nvtx_analyzer::NvtxEventView::NameCategory {
                            domain: *domain,
                            category: *category,
                            name,
                        },
                    Self::NameThread { thread_id, name } =>
                        ::nvtx_analyzer::NvtxEventView::NameThread {
                            thread_id: *thread_id,
                            name,
                        },
                    Self::ResourceCreate {
                        domain,
                        handle,
                        identifier_type,
                        identifier,
                        message,
                    } => ::nvtx_analyzer::NvtxEventView::ResourceCreate {
                        domain: *domain,
                        handle: *handle,
                        identifier_type: *identifier_type,
                        identifier: *identifier,
                        message: message.as_ref().and_then(|message| message.nvtx_message()),
                    },
                    Self::ResourceDestroy { handle } =>
                        ::nvtx_analyzer::NvtxEventView::ResourceDestroy { handle: *handle },
                })
            }
        }

        #capture_platform_guard
        #capture
    })
}

#[cfg(test)]
mod tests {
    use quent_fsm::{FsmEntityBuilder, StateDecl};
    use quent_os::{process_path, process_record};
    use quent_schema::builder::{EntityBuilder, EventBuilder, SchemaBuilder};
    use quent_schema::test_utils::{field, ident, path};
    use quent_schema::{Cardinality, DataType, Schema};

    use crate::{Options, generate_str};

    fn nvtx_schema() -> Schema {
        let process = EntityBuilder::new(path("ApplicationProcess"))
            .with_event(
                EventBuilder::new(ident("started"), Cardinality::Once)
                    .with_field(field("process", DataType::Record(process_path())))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let base = SchemaBuilder::try_new("Application")
            .unwrap()
            .with_record(process_record())
            .with_entity(process)
            .build()
            .unwrap();
        nvtx_schema::compose(&base, &path("ApplicationProcess")).unwrap()
    }

    #[test]
    fn generates_one_complete_binding_for_live_and_stored_types() {
        for instrumentation in [true, false] {
            let source = generate_str(
                &nvtx_schema(),
                &Options {
                    instrumentation,
                    serde: !instrumentation,
                    ..Options::default()
                },
            )
            .unwrap();

            assert_eq!(
                source
                    .matches("impl ::nvtx_analyzer::NvtxProcessBindingData")
                    .count(),
                1
            );
            assert_eq!(
                source
                    .matches("impl ::nvtx_analyzer::NvtxMessageData")
                    .count(),
                1
            );
            assert_eq!(
                source
                    .matches("impl ::nvtx_analyzer::NvtxAttributesData")
                    .count(),
                1
            );
            assert_eq!(
                source
                    .matches("impl ::nvtx_analyzer::NvtxEventData")
                    .count(),
                1
            );
            assert!(source.contains("impl ::nvtx_analyzer::NvtxEventData for NvtxEventEvent"));
            assert!(
                source.contains("impl ::nvtx_analyzer::NvtxProcessBindingData for NvtxEventEvent")
            );
            assert!(source.contains("Self::Initialized { process } => Some(process.target)"));
            assert!(source.contains("Self::Initialized { .. } => return None"));
            for variant in [
                "RangePush",
                "RangePop",
                "RangeStart",
                "RangeEnd",
                "Mark",
                "DomainCreate",
                "DomainDestroy",
                "RegisterString",
                "NameCategory",
                "NameThread",
                "ResourceCreate",
                "ResourceDestroy",
            ] {
                assert!(source.contains(&format!("Self::{variant}")), "{variant}");
            }
            assert!(source.contains("f64::from_bits(payload.value_bits)"));
            assert!(source.contains("f32::from_bits(payload.value_bits as u32)"));
            assert!(source.contains("payload.value_bits as u32 as i32"));
            assert!(source.contains("identifier: *identifier"));
            if instrumentation {
                assert!(source.contains("From<::nvtx_events::NvtxEvent>"));
                assert!(!source.contains("nvtx_injection"));
                assert!(!source.contains("with_emit_activation"));
            } else {
                assert!(!source.contains("nvtx_events"));
            }
        }
    }

    #[test]
    fn live_capture_is_private_gated_and_attached_to_the_process_once_event() {
        let source = generate_str(
            &nvtx_schema(),
            &Options {
                nvtx_capture: true,
                serde: true,
                collector_sink: true,
                ..Options::default()
            },
        )
        .unwrap();

        assert!(source.contains("pub(crate) struct NvtxEvent;"));
        assert!(!source.contains("pub struct NvtxEvent;"));
        assert!(!source.contains("pub nvtx_event_observer"));
        assert!(source.contains(".emit_once_and_activate::<"));
        assert!(source.contains("0,\n            >"));
        assert!(source.contains("let __quent_native_process_id = process.native_id"));
        assert!(source.contains("with_emit_activation"));
        assert!(source.contains("::nvtx_injection::register_source"));
        assert!(source.contains("::nvtx_injection::SourceBinding"));
        assert!(source.contains("context_id: __quent_nvtx_context_id"));
        assert!(source.contains("process_id: __quent_nvtx_process_id"));
        assert!(source.contains("stream_id: __quent_nvtx_stream_id"));
        assert!(source.contains("generated NVTX live capture supports 64-bit Linux only"));

        let binding_create = source
            .find("let __quent_nvtx_binding")
            .expect("generated binding event construction");
        let install = source
            .find("::nvtx_injection::register_source")
            .expect("generated source registration");
        let binding_send = source
            .find("__quent_nvtx_process_sender.send(__quent_nvtx_binding)")
            .expect("generated binding event send");
        let gate_open = source[binding_send..]
            .find(".take()")
            .map(|offset| binding_send + offset)
            .expect("generated capture gate opening");
        assert!(
            binding_create < install && install < binding_send && binding_send < gate_open,
            "the installed hook must buffer until its binding is queued"
        );

        let hook_end = install
            + source[install..]
                .find(".map_err")
                .expect("generated hook error mapping");
        let hook = &source[install..hook_end];
        assert!(hook.contains("__quent_nvtx_hook_sender"));
        assert!(hook.contains("__quent_nvtx_hook_gate"));
        assert!(hook.contains("__quent_nvtx_buffer.push(__quent_nvtx_event)"));
        assert!(hook.contains(".send(__quent_nvtx_event)"));
        assert!(!hook.contains("NvtxEventEvent::Initialized"));
        assert!(!hook.contains("__quent_nvtx_inner"));
        assert!(!hook.contains("context.observer"));
        assert!(source.contains("HandleError::source_activation"));

        // Collector/replay dispatch keeps using the raw forward path and does
        // not call the handle-only activation path.
        assert!(source.contains("context.observer::<ApplicationProcess>().forward(event)"));
    }

    #[test]
    fn live_capture_option_requires_live_instrumentation() {
        let error = generate_str(
            &nvtx_schema(),
            &Options {
                instrumentation: false,
                nvtx_capture: true,
                ..Options::default()
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            crate::GenerateError::NvtxCaptureRequiresInstrumentation
        ));
    }

    #[test]
    fn live_capture_option_is_inert_without_the_nvtx_schema() {
        let schema = SchemaBuilder::try_new("Application")
            .unwrap()
            .with_entity(
                EntityBuilder::new(path("Task"))
                    .with_event(
                        EventBuilder::new(ident("started"), Cardinality::Once)
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let source = generate_str(
            &schema,
            &Options {
                nvtx_capture: true,
                ..Options::default()
            },
        )
        .unwrap();

        assert!(!source.contains("nvtx_injection"));
        assert!(!source.contains("with_emit_activation"));
    }

    #[test]
    fn live_capture_rejects_an_fsm_process_with_an_entity_specific_error() {
        let process = FsmEntityBuilder::new(path("StatefulProcess"))
            .with_states([
                StateDecl {
                    name: ident("started"),
                    attributes: vec![field("process", DataType::Record(process_path()))],
                    to: vec![ident("stopped")],
                    initial: true,
                },
                StateDecl {
                    name: ident("stopped"),
                    attributes: vec![],
                    to: vec![],
                    initial: false,
                },
            ])
            .build()
            .unwrap();
        let base = SchemaBuilder::try_new("Application")
            .unwrap()
            .with_record(process_record())
            .with_entity(process)
            .build()
            .unwrap();
        let schema = nvtx_schema::compose(&base, &path("StatefulProcess")).unwrap();

        let error = generate_str(
            &schema,
            &Options {
                nvtx_capture: true,
                ..Options::default()
            },
        )
        .unwrap_err();

        assert!(matches!(
            &error,
            crate::GenerateError::NvtxCaptureFsmProcessUnsupported { entity }
                if entity == &path("StatefulProcess")
        ));
        assert!(error.to_string().contains("ordinary entity handle"));
    }

    #[test]
    fn schema_without_nvtx_has_no_analyzer_dependency_or_bindings() {
        let schema = SchemaBuilder::try_new("Application")
            .unwrap()
            .with_entity(
                EntityBuilder::new(path("Task"))
                    .with_event(
                        EventBuilder::new(ident("started"), Cardinality::Once)
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let source = generate_str(&schema, &Options::default()).unwrap();

        assert!(!source.contains("nvtx_analyzer"));
        assert!(!source.contains("NvtxEventEvent"));
    }
}
