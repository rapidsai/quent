//! Telemetry analysis functionality based on modeling primitives.

use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

pub use crate::error::AnalyzerError;

pub mod error;
pub mod fsm;
pub mod resource;
pub mod timeline;

pub type AnalyzerResult<T> = std::result::Result<T, AnalyzerError>;

/// Trait for entities.
pub trait Entity {
    /// Return the universally unique identifier of this entity.
    fn id(&self) -> Uuid;
    /// The type name of this entity.
    fn type_name(&self) -> &str;
    /// The instance name of this entity.
    // TODO(johanpel): consider making this optional.
    fn instance_name(&self) -> &str;
}

/// Trait for entities associated with a single moment in time.
pub trait Instant: Entity {
    /// Return the timestamp associated with this type.
    fn instant(&self) -> AnalyzerResult<TimeUnixNanoSec>;
}

/// Trait for entities that are associated with a span of time.
///
/// Typically represents the entire lifetime of the entity.
pub trait Span: Entity {
    /// Return the span of time this type is associated with.
    ///
    /// # Errors
    ///
    /// This function can return an [`AnalyzerError`] in cases such as:
    /// - Events are missing to form a complete entity model.
    /// - The sequence of FSM transition events violates model specifications.
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec>;
}
