// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Filesystem-backed event storage.

use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};
use quent_events::{CombinedEventModel, Event, EventModel, EventPayload};
use quent_io::ImporterProvider;
use quent_io::filesystem::{Format, importer};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::context::ContextSet;

use super::{CombinedEventLoader, EntityMarkerInModel, EventIterator, EventLoader};

/// Result returned by filesystem event stores.
pub type Result<T> = std::result::Result<T, Error>;

/// An error encountered while loading filesystem events.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("context `{0}` was not found")]
    ContextNotFound(Uuid),
    #[error("context path `{0}` is not a directory")]
    ContextNotDirectory(PathBuf),
    #[error("context model `{actual}` does not match expected model `{expected}`")]
    ModelMismatch { expected: String, actual: String },
    #[error("event file `{path}` requires the `{feature}` feature for `{format}` data")]
    DisabledFormat {
        path: PathBuf,
        format: String,
        feature: &'static str,
    },
    #[error("failed to {operation} `{path}`: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to import events from `{path}`: {source}")]
    Importer {
        path: PathBuf,
        #[source]
        source: quent_io::ImporterError,
    },
}

/// Associates a generated model with its filesystem event importers.
///
/// # Code generation
///
/// `quent-store-build` implements this trait only when
/// `quent_store_build::Options::filesystem` and
/// `quent_store_build::Options::combined_event` are enabled.
///
/// For example, `impl Model for Demo` includes
/// `EventImporter::<Demo>::import_for_entity::<Task>()`.
#[doc(hidden)]
pub trait Model: CombinedEventModel {
    /// Returns the event importers generated from the model schema.
    fn event_importers() -> &'static [EventImporter<Self>]
    where
        Self: Sized;
}

type ImportFn<M> =
    fn(Vec<EventFile>) -> Result<EventIterator<<M as CombinedEventModel>::CombinedEvent, Error>>;

/// Imports one entity event payload type into a model's combined event type.
///
/// # Code generation
///
/// `quent-store-build` emits one importer per entity marker, such as
/// `EventImporter::<Demo>::import_for_entity::<Task>()`.
#[doc(hidden)]
pub struct EventImporter<M: CombinedEventModel> {
    entity: &'static str,
    import: ImportFn<M>,
}

impl<M: CombinedEventModel> EventImporter<M> {
    /// Creates an importer for one entity marker in `M`.
    #[doc(hidden)]
    pub const fn import_for_entity<E>() -> Self
    where
        E: EntityMarkerInModel<M>,
        E::Payload: DeserializeOwned + Into<M::CombinedEvent> + 'static,
        M::CombinedEvent: 'static,
    {
        Self {
            entity: E::Payload::NAME,
            import: import_event_files::<M, E::Payload>,
        }
    }
}

/// Identifies an event file and its encoding format.
struct EventFile {
    format: Format,
    path: PathBuf,
}

fn import_event_files<M, E>(files: Vec<EventFile>) -> Result<EventIterator<M::CombinedEvent, Error>>
where
    M: CombinedEventModel,
    E: DeserializeOwned + Into<M::CombinedEvent> + 'static,
    M::CombinedEvent: 'static,
{
    Ok(Box::new(import_files::<E>(files).map(|event| {
        event.map(|event| Event::new(event.id, event.timestamp, event.data.into()))
    })))
}

/// Loads model events from filesystem exporter output.
pub struct Loader<M> {
    root: PathBuf,
    contexts: ContextSet,
    model: PhantomData<fn() -> M>,
}

impl<M> Loader<M> {
    /// Creates a loader for the selected contexts under an exporter output directory.
    pub fn new(root: impl Into<PathBuf>, contexts: ContextSet) -> Self {
        Self {
            root: root.into(),
            contexts,
            model: PhantomData,
        }
    }

    /// Returns the exporter output directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the selected contexts.
    pub fn contexts(&self) -> &ContextSet {
        &self.contexts
    }
}

impl<M, E> EventLoader<E> for Loader<M>
where
    M: EventModel,
    E: EntityMarkerInModel<M>,
    E::Payload: DeserializeOwned + 'static,
{
    type Error = Error;

    fn events(&self) -> Result<EventIterator<E::Payload, Error>> {
        let mut files = Vec::new();
        for &context_id in self.contexts.as_slice() {
            let context = self.context(context_id)?;
            files.extend(event_files(&context, E::Payload::NAME)?);
        }
        Ok(import_files::<E::Payload>(files))
    }
}

impl<M> CombinedEventLoader<M> for Loader<M>
where
    M: EventModel + Model + 'static,
{
    type Error = Error;

    fn combined_events(&self) -> Result<EventIterator<M::CombinedEvent, Error>> {
        let mut streams = Vec::new();
        for &context_id in self.contexts.as_slice() {
            let context = self.context(context_id)?;
            for descriptor in M::event_importers() {
                let files = event_files(&context, descriptor.entity)?;
                streams.push((descriptor.import)(files)?);
            }
        }
        Ok(Box::new(streams.into_iter().flatten()))
    }
}

impl<M> Loader<M>
where
    M: EventModel,
{
    /// Returns the context directory after verifying that it exists and belongs to `M`.
    fn context(&self, context_id: Uuid) -> Result<PathBuf> {
        let context = self.root.join(context_id.to_string());
        match std::fs::metadata(&context) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(Error::ContextNotDirectory(context)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::ContextNotFound(context_id));
            }
            Err(source) => {
                return Err(Error::Io {
                    operation: "inspect context path",
                    path: context,
                    source,
                });
            }
        }
        let artifact = ArtifactInfo::read_sidecar(&context).map_err(|source| Error::Io {
            operation: "read context metadata from",
            path: context.join(SIDECAR_FILE_NAME),
            source,
        })?;
        if artifact.model.name != M::NAME {
            return Err(Error::ModelMismatch {
                expected: M::NAME.to_owned(),
                actual: artifact.model.name,
            });
        }
        Ok(context)
    }
}

/// Imports event files in their supplied order and yields importer failures as iterator items.
fn import_files<T>(files: Vec<EventFile>) -> EventIterator<T, Error>
where
    T: DeserializeOwned + 'static,
{
    Box::new(files.into_iter().flat_map(|file| {
        let stream = || {
            let path = file.path;
            let importer = importer::Options {
                format: file.format,
                path: path.clone(),
            }
            .create_importer()
            .map_err(|source| Error::Importer {
                path: path.clone(),
                source,
            })?;
            Ok::<_, Error>(Box::new(importer.map(move |event| {
                event.map_err(|source| Error::Importer {
                    path: path.clone(),
                    source,
                })
            })) as EventIterator<T, Error>)
        };

        stream().unwrap_or_else(|error| Box::new(std::iter::once(Err(error))))
    }))
}

/// Returns recognized event files for payload stream `entity` in path order.
///
/// A missing or non-directory stream path produces an empty list. A recognized format whose
/// feature is disabled produces an error.
fn event_files(context: &Path, entity: &str) -> Result<Vec<EventFile>> {
    let directory = context.join(entity);
    match std::fs::metadata(&directory) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(Vec::new()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::Io {
                operation: "inspect event directory",
                path: directory,
                source,
            });
        }
    }

    let entries = std::fs::read_dir(&directory).map_err(|source| Error::Io {
        operation: "read event directory",
        path: directory.clone(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            operation: "read entry in event directory",
            path: directory.clone(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| Error::Io {
            operation: "inspect event file",
            path: path.clone(),
            source,
        })?;
        if file_type.is_file() {
            paths.push(path);
        }
    }
    paths.sort();

    let mut files = Vec::new();
    for path in paths {
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            tracing::debug!(
                path = %path.display(),
                "ignoring file with unsupported event format"
            );
            continue;
        };
        match Format::try_from(extension) {
            Ok(format) => files.push(EventFile { format, path }),
            Err(_) => {
                let normalized = extension.to_ascii_lowercase();
                let Some(feature) = format_feature(&normalized) else {
                    tracing::debug!(
                        path = %path.display(),
                        "ignoring file with unsupported event format"
                    );
                    continue;
                };
                return Err(Error::DisabledFormat {
                    path,
                    format: normalized,
                    feature,
                });
            }
        }
    }
    Ok(files)
}

fn format_feature(extension: &str) -> Option<&'static str> {
    match extension {
        "ndjson" => Some("io-ndjson"),
        "msgpack" => Some("io-msgpack"),
        "postcard" => Some("io-postcard"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::entity::native;
    use crate::entity::{BorrowedEventSequenceStore, EntityHandle, EntityStore};
    use quent_build_info::{BuildInfo, ModelInfo, ModelSource};
    use quent_events::{CombinedEventModel, EntityMarker, Event, EventModel, EventPayload};
    use quent_instrumentation::{ContextExporter, ContextInner};
    use quent_io::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
    use serde::{Deserialize, Serialize};

    use super::*;

    struct TestModel;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct AlphaEvent(u8);

    impl EventPayload for AlphaEvent {
        const NAME: &'static str = "Alpha";
    }

    struct Alpha;

    impl EntityMarker for Alpha {
        type Payload = AlphaEvent;
    }

    impl EntityMarkerInModel<TestModel> for Alpha {}

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct BetaEvent(u8);

    impl EventPayload for BetaEvent {
        const NAME: &'static str = "Beta";
    }

    struct Beta;

    impl EntityMarker for Beta {
        type Payload = BetaEvent;
    }

    impl EntityMarkerInModel<TestModel> for Beta {}

    #[derive(Debug, PartialEq)]
    enum TestEvent {
        Alpha(AlphaEvent),
        Beta(BetaEvent),
    }

    impl From<AlphaEvent> for TestEvent {
        fn from(event: AlphaEvent) -> Self {
            Self::Alpha(event)
        }
    }

    impl From<BetaEvent> for TestEvent {
        fn from(event: BetaEvent) -> Self {
            Self::Beta(event)
        }
    }

    impl EventModel for TestModel {
        const NAME: &'static str = "Test";
    }

    impl ModelSource for TestModel {
        fn package() -> &'static str {
            "quent-store"
        }

        fn source() -> BuildInfo {
            BuildInfo::unknown()
        }
    }

    impl CombinedEventModel for TestModel {
        type CombinedEvent = TestEvent;
    }

    impl Model for TestModel {
        fn event_importers() -> &'static [EventImporter<Self>] {
            static IMPORTERS: &[EventImporter<TestModel>] = &[
                EventImporter::import_for_entity::<Alpha>(),
                EventImporter::import_for_entity::<Beta>(),
            ];
            IMPORTERS
        }
    }

    fn context<M>(root: &Path, id: Uuid) -> (ContextInner, ExporterOptions)
    where
        M: EventModel + ModelSource,
    {
        let context = ContextInner::try_new(id).unwrap();
        let options = ExporterOptions::FileSystem(FileSystemExporterOptions::new(
            FileSystemFormat::Ndjson,
            root.to_path_buf(),
        ));
        options.prepare_context(id, M::model_info());
        (context, options)
    }

    fn export_events(root: &Path, id: Uuid) {
        export_context_events(
            root,
            id,
            [Event::new(Uuid::from_u128(11), 11, AlphaEvent(1))],
            [Event::new(Uuid::from_u128(12), 1, BetaEvent(2))],
        );
    }

    fn export_context_events(
        root: &Path,
        id: Uuid,
        alpha_events: impl IntoIterator<Item = Event<AlphaEvent>>,
        beta_events: impl IntoIterator<Item = Event<BetaEvent>>,
    ) {
        let (context, options) = context::<TestModel>(root, id);
        let alpha = context
            .block_on(context.observer::<AlphaEvent>(&options))
            .unwrap();
        let beta = context
            .block_on(context.observer::<BetaEvent>(&options))
            .unwrap();

        for event in alpha_events {
            alpha.send(event);
        }
        for event in beta_events {
            beta.send(event);
        }
    }

    #[test]
    fn loads_all_model_events_without_relying_on_order() {
        let root = tempfile::tempdir().unwrap();
        let id = Uuid::from_u128(2);
        export_events(root.path(), id);

        let store = Loader::<TestModel>::new(root.path(), ContextSet::one(id));
        let events = store
            .combined_events()
            .unwrap()
            .map(|event| event.map(|event| event.data))
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 2);
        assert!(events.contains(&TestEvent::Alpha(AlphaEvent(1))));
        assert!(events.contains(&TestEvent::Beta(BetaEvent(2))));
    }

    #[test]
    fn loads_one_entity_type_as_concrete_events() {
        let root = tempfile::tempdir().unwrap();
        let id = Uuid::from_u128(2);
        export_events(root.path(), id);

        let store = Loader::<TestModel>::new(root.path(), ContextSet::one(id));
        let events = EventLoader::<Alpha>::events(&store)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, AlphaEvent(1));
    }

    #[test]
    fn imports_selected_filesystem_events_into_memory_entities() {
        let root = tempfile::tempdir().unwrap();
        let first_context = Uuid::from_u128(1);
        let second_context = Uuid::from_u128(2);
        let excluded_context = Uuid::from_u128(3);
        let first_entity = Uuid::from_u128(11);
        let second_entity = Uuid::from_u128(12);
        export_context_events(
            root.path(),
            first_context,
            [
                Event::new(first_entity, 20, AlphaEvent(1)),
                Event::new(second_entity, 5, AlphaEvent(3)),
            ],
            [Event::new(first_entity, 30, BetaEvent(4))],
        );
        export_context_events(
            root.path(),
            second_context,
            [Event::new(first_entity, 10, AlphaEvent(2))],
            [],
        );
        export_context_events(
            root.path(),
            excluded_context,
            [Event::new(first_entity, 1, AlphaEvent(99))],
            [],
        );
        let contexts = ContextSet::try_new([second_context, first_context]).unwrap();
        let store = Loader::<TestModel>::new(root.path(), contexts);
        let events = EventLoader::<Alpha>::events(&store)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        let entities = native::Store::<Alpha>::new(events);

        let handles = EntityStore::<Alpha>::entities(&entities)
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(handles.len(), 2);
        assert!(handles.iter().any(|handle| handle.id() == first_entity));
        assert!(handles.iter().any(|handle| handle.id() == second_entity));
        let first_handle = EntityStore::<Alpha>::entity(&entities, first_entity)
            .unwrap()
            .unwrap();
        assert_eq!(
            entities
                .event_sequence(&first_handle)
                .unwrap()
                .events()
                .iter()
                .map(|event| event.data.0)
                .collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(
            EntityStore::<Alpha>::entity(&entities, first_entity)
                .unwrap()
                .unwrap()
                .id(),
            first_entity
        );
    }

    #[test]
    fn validates_context_and_model() {
        let root = tempfile::tempdir().unwrap();
        let store = Loader::<TestModel>::new(root.path(), ContextSet::one(Uuid::from_u128(1)));

        let missing = Uuid::from_u128(1);
        assert!(matches!(
            store.context(missing),
            Err(Error::ContextNotFound(id)) if id == missing
        ));

        let mismatch = Uuid::from_u128(2);
        let context = root.path().join(mismatch.to_string());
        fs::create_dir(&context).unwrap();
        let mut model = ModelInfo::unknown();
        model.name = "Other".to_owned();
        ArtifactInfo::new(model).write_sidecar(&context).unwrap();
        assert!(matches!(
            store.context(mismatch),
            Err(Error::ModelMismatch { actual, .. }) if actual == "Other"
        ));
    }

    #[cfg(feature = "io-ndjson")]
    #[test]
    fn returns_event_files_in_path_order() {
        let root = tempfile::tempdir().unwrap();
        let entity = root.path().join("Alpha");
        fs::create_dir(&entity).unwrap();
        for name in ["charlie.ndjson", "alpha.ndjson", "bravo.ndjson"] {
            fs::write(entity.join(name), b"").unwrap();
        }

        let paths = event_files(root.path(), "Alpha")
            .unwrap()
            .into_iter()
            .map(|file| file.path.file_name().unwrap().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(paths, ["alpha.ndjson", "bravo.ndjson", "charlie.ndjson"]);
    }

    #[test]
    fn rejects_event_files_for_disabled_formats() {
        if Format::try_from("msgpack").is_ok() {
            return;
        }

        let root = tempfile::tempdir().unwrap();
        let entity = root.path().join("Alpha");
        fs::create_dir(&entity).unwrap();
        let path = entity.join("events.msgpack");
        fs::write(&path, b"").unwrap();

        assert!(matches!(
            event_files(root.path(), "Alpha"),
            Err(Error::DisabledFormat {
                path: error_path,
                format,
                feature: "io-msgpack",
            }) if error_path == path && format == "msgpack"
        ));
    }

    #[cfg(feature = "io-ndjson")]
    #[test]
    fn reports_import_failures_during_iteration() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("events.ndjson");
        fs::write(&path, b"not json\n").unwrap();

        #[derive(Deserialize)]
        struct TestEvent;

        let mut events = import_files::<TestEvent>(vec![EventFile {
            format: Format::Ndjson,
            path: path.clone(),
        }]);

        assert!(matches!(
            events.next(),
            Some(Err(Error::Importer { path: error_path, .. })) if error_path == path
        ));
        assert!(events.next().is_none());
    }
}
