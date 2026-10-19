// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Log filters and response types for the UI and API.

use crate::analysis::LogEventPayload;
use quent_analyzer::{AnalyzerError, AnalyzerResult};
pub use quent_dynamic_attributes::{DynamicAttribute, DynamicList, DynamicStruct, DynamicValue};
use quent_events::Event;
use quent_time::{TimeUnixNanoSec, try_to_secs_relative};
use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use quent_ui::{entities::request::TimeWindow, paginate::PageParams};

/// Converts a log event's extra fields to attributes for an API response.
///
/// Field names and nested records keep the structure defined in the schema.
/// Booleans become [`DynamicValue::U8`] values (`0` or `1`), UUIDs become strings,
/// and entity references become records with `target` and optional `data` fields.
/// Missing optional values become null attributes. Lists with optional values
/// use a record with a `value` attribute for each element, so missing values
/// do not change the list's length or order.
pub trait LogAttributes: LogEventPayload {
    /// Returns newly allocated attributes for fields other than `message`.
    ///
    /// Call this only when building a UI or API response, after filtering and
    /// selecting a page. Events that are not log messages return an empty vector.
    fn log_attributes(&self) -> Vec<DynamicAttribute>;
}

/// Selects log messages and chooses which page to return.
// TODO(johanpel): Support attribute filtering.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct LogListRequest {
    pub window: Option<TimeWindow>,
    pub level_names: Option<Vec<String>>,
    /// Include levels with this rank or higher, using the ranks defined in the schema.
    pub min_rank: Option<u8>,
    pub message_contains: Option<String>,
    /// Match messages using Rust regex syntax.
    /// Patterns match anywhere in a message unless anchored with `^` or `$`.
    pub message_regex: Option<String>,
    pub sink_type_name: Option<String>,
    pub sink_id: Option<Uuid>,
    pub page: PageParams,
}

impl Default for LogListRequest {
    fn default() -> Self {
        Self {
            window: None,
            level_names: None,
            min_rank: None,
            message_contains: None,
            message_regex: None,
            sink_type_name: None,
            sink_id: None,
            page: PageParams { max: 100, page: 0 },
        }
    }
}

/// A log level from the schema, ranked from least to most severe.
#[derive(Debug, Clone, Serialize, TS)]
pub struct LogLevelDeclaration {
    pub name: String,
    pub rank: u8,
}

/// A log sink type and the levels declared for it in the schema.
#[derive(Debug, Clone, Serialize, TS)]
pub struct LogSinkType {
    pub name: String,
    pub levels: Vec<LogLevelDeclaration>,
}

/// A log message with its extra fields converted to attributes.
#[derive(Debug, Clone, Serialize, TS)]
pub struct LogListItem {
    pub sink_id: Uuid,
    pub sink_type_name: String,
    /// Time in seconds since the engine started.
    pub timestamp_s: f64,
    pub level: LogLevelDeclaration,
    pub message: String,
    pub attributes: Vec<DynamicAttribute>,
}

/// A page of messages in timestamp order, with the total number of matches.
///
/// The total counts all matching messages, not just those on this page.
#[derive(Debug, Clone, Serialize, TS)]
pub struct LogListResponse {
    pub items: Vec<LogListItem>,
    pub total: usize,
    pub sink_types: Vec<LogSinkType>,
}

/// Returns a page of matching log messages from events ordered by timestamp.
///
/// Time windows use seconds since `epoch`: the start is included and the end is
/// excluded. A message must match every supplied filter. Level names and message
/// substring searches are case-sensitive. Extra fields are converted to
/// attributes only for messages on the returned page.
///
/// # Errors
///
/// Returns [`AnalyzerError::InvalidArgument`] if the regex is invalid, the page
/// size is zero, or the time window is invalid. A valid window has bounds at or
/// above zero, contains no infinity or `NaN`, fits the supported timestamp range,
/// and ends at or after it starts.
pub fn list_logs<T: LogAttributes>(
    events: &[Event<T>],
    epoch: TimeUnixNanoSec,
    request: LogListRequest,
) -> AnalyzerResult<LogListResponse> {
    let invalid = |message: &str| AnalyzerError::InvalidArgument(message.to_owned());
    if request.page.max == 0 {
        return Err(invalid("log page size must be greater than zero"));
    }
    let window = request
        .window
        .map(|window| {
            let absolute = |seconds: f64| {
                if !seconds.is_finite() || seconds < 0.0 {
                    return Err(invalid("log window bounds must be finite and nonnegative"));
                }
                let nanos = seconds * 1_000_000_000.0;
                if nanos >= u64::MAX as f64 {
                    return Err(invalid("log window exceeds the timestamp range"));
                }
                epoch
                    .checked_add(nanos as u64)
                    .ok_or_else(|| invalid("log window exceeds the timestamp range"))
            };
            if window.end < window.start {
                return Err(invalid("log window end precedes its start"));
            }
            Ok((absolute(window.start)?, absolute(window.end)?))
        })
        .transpose()?;
    let regex = request
        .message_regex
        .as_deref()
        .map(Regex::new)
        .transpose()
        .map_err(|error| {
            AnalyzerError::InvalidArgument(format!("invalid log message regex: {error}"))
        })?;
    let offset = u64::from(request.page.page) * u64::from(request.page.max);
    let mut total = 0;
    let mut selected = Vec::new();
    for event in events {
        let Some(message) = event.data.log_message() else {
            continue;
        };
        if window.is_some_and(|(start, end)| event.timestamp < start || event.timestamp >= end)
            || request.sink_id.is_some_and(|id| id != event.id)
            || request
                .sink_type_name
                .as_ref()
                .is_some_and(|name| name != T::NAME)
            || request
                .level_names
                .as_ref()
                .is_some_and(|names| !names.iter().any(|name| name == message.level.name))
            || request
                .min_rank
                .is_some_and(|rank| message.level.rank < rank)
            || request
                .message_contains
                .as_ref()
                .is_some_and(|text| !message.message.contains(text))
            || regex
                .as_ref()
                .is_some_and(|regex| !regex.is_match(message.message))
        {
            continue;
        }
        if total as u64 >= offset && selected.len() < request.page.max as usize {
            selected.push((event, message));
        }
        total += 1;
    }
    let items = selected
        .into_iter()
        .map(|(event, message)| {
            Ok(LogListItem {
                sink_id: event.id,
                sink_type_name: T::NAME.to_owned(),
                timestamp_s: try_to_secs_relative(event.timestamp, epoch)?,
                level: LogLevelDeclaration {
                    name: message.level.name.to_owned(),
                    rank: message.level.rank,
                },
                message: message.message.to_owned(),
                attributes: event.data.log_attributes(),
            })
        })
        .collect::<AnalyzerResult<_>>()?;
    Ok(LogListResponse {
        items,
        total,
        sink_types: vec![sink_type::<T>()],
    })
}

/// Returns the sink type and all its declared levels, even if no messages use them.
pub fn sink_type<T: LogEventPayload>() -> LogSinkType {
    LogSinkType {
        name: T::NAME.to_owned(),
        levels: T::LEVELS
            .iter()
            .map(|level| LogLevelDeclaration {
                name: level.name.to_owned(),
                rank: level.rank,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{LogCollection, LogLevel, LogMessage};
    use quent_events::EventPayload;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Payload {
        text: Option<&'static str>,
        rank: u8,
        projections: Arc<AtomicUsize>,
    }
    impl EventPayload for Payload {
        const NAME: &'static str = "CustomLog";
    }
    impl LogEventPayload for Payload {
        const LEVELS: &'static [LogLevel] = &[
            LogLevel {
                name: "verbose",
                rank: 0,
            },
            LogLevel {
                name: "critical",
                rank: 1,
            },
        ];
        fn log_message(&self) -> Option<LogMessage<'_>> {
            self.text.map(|message| LogMessage {
                message,
                level: Self::LEVELS[self.rank as usize],
            })
        }
    }
    impl LogAttributes for Payload {
        fn log_attributes(&self) -> Vec<DynamicAttribute> {
            self.projections.fetch_add(1, Ordering::SeqCst);
            vec![DynamicAttribute::u8("rank", self.rank)]
        }
    }
    const EPOCH: u64 = 1_000_000_000;
    fn fixture() -> (LogCollection<Payload>, Arc<AtomicUsize>) {
        let projections = Arc::new(AtomicUsize::new(0));
        let events = [
            (4, Some("ERROR E42"), 1),
            (0, None, 0),
            (1, Some("scan café"), 0),
            (2, Some("error E42"), 1),
            (2, Some("error E42"), 1),
        ]
        .into_iter()
        .map(|(seconds, text, rank)| {
            Event::new(
                Uuid::from_u128(1),
                EPOCH + seconds * 1_000_000_000,
                Payload {
                    text,
                    rank,
                    projections: Arc::clone(&projections),
                },
            )
        })
        .collect();
        (LogCollection::new(events), projections)
    }
    #[test]
    fn filters_and_paginates_before_projecting() {
        let (logs, projections) = fixture();
        assert_eq!(projections.load(Ordering::SeqCst), 0);
        let result = list_logs(
            logs.events(),
            EPOCH,
            LogListRequest {
                window: Some(TimeWindow {
                    start: 2.0,
                    end: 4.0,
                }),
                level_names: Some(vec!["critical".into()]),
                min_rank: Some(1),
                message_contains: Some("E42".into()),
                message_regex: Some("^error E[0-9]+$".into()),
                sink_type_name: Some("CustomLog".into()),
                sink_id: Some(Uuid::from_u128(1)),
                page: PageParams { max: 1, page: 1 },
            },
        )
        .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].timestamp_s, 2.0);
        assert_eq!(result.sink_types[0].levels.len(), 2);
        assert_eq!(projections.load(Ordering::SeqCst), 1);
        let empty = list_logs(
            logs.events(),
            EPOCH,
            LogListRequest {
                page: PageParams {
                    max: 1,
                    page: u32::MAX,
                },
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(empty.total, 4);
        assert!(empty.items.is_empty());
        assert_eq!(projections.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn regex_supports_flags_unicode_and_anchors() {
        let (logs, _) = fixture();
        for (pattern, total) in [
            ("(?i)^error E42$", 3),
            ("café$", 1),
            ("E42", 3),
            ("^E42$", 0),
        ] {
            let response = list_logs(
                logs.events(),
                EPOCH,
                LogListRequest {
                    message_regex: Some(pattern.into()),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(response.total, total, "{pattern}");
        }
    }
    #[test]
    fn rejects_invalid_requests_without_projection() {
        let (logs, projections) = fixture();
        let windows = [
            TimeWindow {
                start: 2.0,
                end: 1.0,
            },
            TimeWindow {
                start: -1.0,
                end: 1.0,
            },
            TimeWindow {
                start: f64::NAN,
                end: 1.0,
            },
            TimeWindow {
                start: 0.0,
                end: f64::INFINITY,
            },
            TimeWindow {
                start: 0.0,
                end: u64::MAX as f64,
            },
        ];
        for window in windows {
            assert!(matches!(
                list_logs(
                    logs.events(),
                    EPOCH,
                    LogListRequest {
                        window: Some(window),
                        ..Default::default()
                    }
                ),
                Err(AnalyzerError::InvalidArgument(_))
            ));
        }
        for request in [
            LogListRequest {
                message_regex: Some("[".into()),
                ..Default::default()
            },
            LogListRequest {
                page: PageParams { max: 0, page: 0 },
                ..Default::default()
            },
        ] {
            assert!(matches!(
                list_logs(logs.events(), EPOCH, request),
                Err(AnalyzerError::InvalidArgument(_))
            ));
        }
        assert_eq!(projections.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn empty_selections_and_time_boundaries() {
        let (logs, projections) = fixture();
        for request in [
            LogListRequest {
                window: Some(TimeWindow {
                    start: 2.0,
                    end: 2.0,
                }),
                ..Default::default()
            },
            LogListRequest {
                level_names: Some(Vec::new()),
                ..Default::default()
            },
            LogListRequest {
                sink_id: Some(Uuid::nil()),
                ..Default::default()
            },
            LogListRequest {
                sink_type_name: Some("Other".into()),
                ..Default::default()
            },
            LogListRequest {
                message_contains: Some("missing".into()),
                ..Default::default()
            },
        ] {
            let result = list_logs(logs.events(), EPOCH, request).unwrap();
            assert_eq!(result.total, 0);
            assert!(result.items.is_empty());
        }
        assert_eq!(projections.load(Ordering::SeqCst), 0);
        let response = list_logs(logs.events(), EPOCH, LogListRequest::default()).unwrap();
        assert_eq!(
            response
                .items
                .iter()
                .map(|item| item.timestamp_s)
                .collect::<Vec<_>>(),
            [1.0, 2.0, 2.0, 4.0]
        );
    }
}
