// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Access to log messages defined in an application's schema.

use quent_events::{Event, EventPayload};

/// A log level defined in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLevel {
    /// The level name exactly as declared in the schema, such as `info` or `error`.
    pub name: &'static str,
    /// The level's position in the schema's level list, starting at zero.
    ///
    /// Higher ranks mean more severe messages. Rank `0` is the least severe.
    pub rank: u8,
}

/// A message and log level borrowed from the original event.
#[derive(Debug, Clone, Copy)]
pub struct LogMessage<'a> {
    /// The event's level, using the name and rank declared by its log sink.
    pub level: LogLevel,
    /// The event's `message` field, borrowed without copying or formatting its text.
    pub message: &'a str,
}

/// Allows analysis to convert application-specific log payloads into an
/// application-agnostic representation.
///
/// Compare severity ranks only within the same sink type, identified by
/// [`EventPayload::NAME`].
pub trait LogEventPayload: EventPayload {
    /// All levels declared by the sink's [`LogDefinition`](crate::LogDefinition), in rank order.
    const LEVELS: &'static [LogLevel];

    /// Borrows this event's message and level, or returns `None` for non-message events.
    ///
    /// The level must match an entry in [`Self::LEVELS`].
    fn log_message(&self) -> Option<LogMessage<'_>>;
}

/// The original events in timestamp order, including events that are not log messages.
#[derive(Debug)]
pub struct LogCollection<T> {
    events: Vec<Event<T>>,
}

impl<T: LogEventPayload> LogCollection<T> {
    /// Keeps all events and sorts them by timestamp.
    ///
    /// Events with the same timestamp keep their input order, including duplicates.
    /// Sorting costs `O(n log n)` time.
    pub fn new(mut events: Vec<Event<T>>) -> Self {
        events.sort_by_key(|event| event.timestamp);
        Self { events }
    }

    /// Returns the original events in timestamp order.
    pub fn events(&self) -> &[Event<T>] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_events::Uuid;

    enum Payload {
        Initialized,
        Info(String),
    }

    impl EventPayload for Payload {
        const NAME: &'static str = "TestLog";
    }

    impl LogEventPayload for Payload {
        const LEVELS: &'static [LogLevel] = &[LogLevel {
            name: "info",
            rank: 0,
        }];

        fn log_message(&self) -> Option<LogMessage<'_>> {
            match self {
                Self::Info(message) => Some(LogMessage {
                    message,
                    level: Self::LEVELS[0],
                }),
                Self::Initialized => None,
            }
        }
    }

    #[test]
    fn keeps_typed_events_duplicates_and_equal_timestamp_order() {
        let id = Uuid::from_u128(1);
        let logs = LogCollection::new(vec![
            Event::new(id, 2, Payload::Info("last".into())),
            Event::new(id, 1, Payload::Info("first".into())),
            Event::new(id, 1, Payload::Info("second".into())),
            Event::new(id, 1, Payload::Info("first".into())),
            Event::new(id, 0, Payload::Initialized),
        ]);
        assert!(matches!(logs.events()[0].data, Payload::Initialized));
        let messages: Vec<_> = logs
            .events()
            .iter()
            .filter_map(|event| event.data.log_message())
            .map(|message| message.message)
            .collect();
        assert_eq!(messages, ["first", "second", "first", "last"]);
        assert!(logs.events().iter().all(|event| event.id == id));
    }
}
