// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Filesystem-backed event storage.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};
use quent_events::{EntityEvent, Event, Model as EventModel, ModelEvents};
use quent_io::ImporterProvider;
use quent_io::filesystem::{Format, importer};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::{
    EntityEventLoader, EntityEventStore, EventIterator, ModelEventLoader, ModelEventStore,
    StoredEntity,
};

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

/// Availability of a stream after a context has been loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamAvailability {
    /// The model schema does not declare this stream.
    Undeclared,
    /// The schema declares it, but no stream directory is available.
    Missing,
    /// The stream directory exists and contains no decoded events.
    Empty,
    /// At least one event was decoded from the stream.
    Populated,
}

/// One validated context's decoded model events and stream availability.
///
/// Events retain file/stream traversal order, which is not necessarily timestamp
/// order. Analysis can borrow these events to share one load across consumers.
pub struct LoadedContext<M: ModelEvents> {
    id: Uuid,
    events: Vec<Event<M::UmbrellaEvent>>,
    streams: BTreeMap<&'static str, StreamAvailability>,
}

impl<M: ModelEvents> LoadedContext<M> {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn events(&self) -> &[Event<M::UmbrellaEvent>] {
        &self.events
    }

    pub fn into_events(self) -> Vec<Event<M::UmbrellaEvent>> {
        self.events
    }

    /// Look up a schema stream name, including streams that were not captured.
    pub fn stream_availability(&self, entity: &str) -> StreamAvailability {
        self.streams
            .get(entity)
            .copied()
            .unwrap_or(StreamAvailability::Undeclared)
    }
}

/// Associates a generated model with its filesystem entity-event streams.
#[doc(hidden)]
pub trait Model: ModelEvents {
    /// Returns the streams generated from the model schema.
    fn event_streams() -> &'static [EventStream<Self>]
    where
        Self: Sized;
}

type ImportFn<M> =
    fn(Vec<EventFile>) -> Result<EventIterator<<M as ModelEvents>::UmbrellaEvent, Error>>;

/// Describes one entity-event stream in a generated analysis model.
#[doc(hidden)]
pub struct EventStream<M: ModelEvents> {
    entity: &'static str,
    import: ImportFn<M>,
}

impl<M: ModelEvents> EventStream<M> {
    /// Creates a generated entity-event stream descriptor.
    #[doc(hidden)]
    pub const fn new(entity: &'static str, import: ImportFn<M>) -> Self {
        Self { entity, import }
    }
}

/// Identifies an event file and the importer required to decode it.
#[doc(hidden)]
pub struct EventFile {
    format: Format,
    path: PathBuf,
}

/// Imports files containing entity events and converts them to the model umbrella type.
#[doc(hidden)]
pub fn import_event_files<M, E>(
    files: Vec<EventFile>,
) -> Result<EventIterator<M::UmbrellaEvent, Error>>
where
    M: ModelEvents,
    E: DeserializeOwned + Into<M::UmbrellaEvent> + 'static,
    M::UmbrellaEvent: 'static,
{
    Ok(Box::new(import_files::<E>(files).map(|event| {
        event.map(|event| Event::new(event.id, event.timestamp, event.data.into()))
    })))
}

/// Loads model events from filesystem exporter output.
pub struct Store<M> {
    root: PathBuf,
    model: PhantomData<fn() -> M>,
}

impl<M> Store<M> {
    /// Creates a store rooted at an exporter output directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            model: PhantomData,
        }
    }

    /// Returns the exporter output directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl<M> EntityEventStore<M> for Store<M> {
    type Error = Error;
}

impl<M, E> EntityEventLoader<E> for Store<M>
where
    M: EventModel,
    E: StoredEntity<M>,
    E::Event: DeserializeOwned + 'static,
{
    type Error = Error;

    fn load_entity_events(&self, context_id: Uuid) -> Result<EventIterator<E::Event, Error>> {
        let context = self.context(context_id)?;
        Ok(load_entity_stream::<E::Event>(&context)?
            .unwrap_or_else(|| Box::new(std::iter::empty())))
    }
}

impl<M: Model> ModelEventStore<M> for Store<M> {}

impl<M> ModelEventLoader<M> for Store<M>
where
    M: EventModel + Model + 'static,
{
    type Error = Error;

    fn load_model_events(
        &self,
        context_id: Uuid,
    ) -> Result<EventIterator<M::UmbrellaEvent, Error>> {
        let context = self.context(context_id)?;
        let mut streams = Vec::new();
        for descriptor in M::event_streams() {
            if let Some(files) = event_files(&context, descriptor.entity)? {
                streams.push((descriptor.import)(files)?);
            }
        }
        Ok(Box::new(streams.into_iter().flatten()))
    }
}

impl<M> Store<M>
where
    M: EventModel + Model + 'static,
{
    /// Load every declared stream once, retaining missing-versus-empty state.
    ///
    /// Validates context/model identity before opening any event files. Returns
    /// an error if any stream fails to decode; no partial context is published.
    pub fn load_context(&self, context_id: Uuid) -> Result<LoadedContext<M>> {
        let context = self.context(context_id)?;
        let mut events = Vec::new();
        let mut streams = BTreeMap::new();
        for descriptor in M::event_streams() {
            let availability = match event_files(&context, descriptor.entity)? {
                None => StreamAvailability::Missing,
                Some(files) => {
                    let start = events.len();
                    for event in (descriptor.import)(files)? {
                        events.push(event?);
                    }
                    if events.len() == start {
                        StreamAvailability::Empty
                    } else {
                        StreamAvailability::Populated
                    }
                }
            };
            streams.insert(descriptor.entity, availability);
        }
        Ok(LoadedContext {
            id: context_id,
            events,
            streams,
        })
    }
}

impl<M> Store<M>
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

/// Load one typed stream from a context directory using each file's format.
///
/// `None` means the stream directory is missing or is not a directory. A present
/// directory yields an iterator, even if empty. Files are visited in path order;
/// recognized formats with disabled features and I/O failures remain errors.
///
/// This lower-level entrypoint does not validate model metadata. Generated
/// models use [`Store`], which performs that validation.
fn load_entity_stream<T>(context: &Path) -> Result<Option<EventIterator<T, Error>>>
where
    T: EntityEvent + DeserializeOwned + 'static,
{
    Ok(event_files(context, T::NAME)?.map(import_files::<T>))
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

/// Returns recognized event files for `entity` in path order.
///
/// A missing or non-directory entity path produces `None`. A recognized format whose
/// feature is disabled produces an error.
fn event_files(context: &Path, entity: &str) -> Result<Option<Vec<EventFile>>> {
    let directory = context.join(entity);
    match std::fs::metadata(&directory) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(None),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
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
    Ok(Some(files))
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

    use quent_build_info::ModelInfo;
    use quent_events::Model as EventModel;
    #[cfg(feature = "io-ndjson")]
    use serde::Deserialize;

    use super::*;

    struct TestModel;

    impl EventModel for TestModel {
        const NAME: &'static str = "Test";
    }

    #[test]
    fn validates_context_and_model() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::<TestModel>::new(root.path());

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

    #[cfg(feature = "io-ndjson")]
    mod context_loading {
        use quent_events::Entity;
        use serde::{Deserialize, Serialize};

        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub(super) struct Sample {
            sequence: u32,
        }

        impl EntityEvent for Sample {
            const NAME: &'static str = "Alpha";
        }

        struct Alpha;
        impl Entity for Alpha {
            type Event = Sample;
        }
        impl StoredEntity<TestModel> for Alpha {}

        impl ModelEvents for TestModel {
            type UmbrellaEvent = Sample;
        }

        impl Model for TestModel {
            fn event_streams() -> &'static [EventStream<Self>] {
                static STREAMS: &[EventStream<TestModel>] = &[
                    EventStream::new("Alpha", import_event_files::<TestModel, Sample>),
                    EventStream::new("Empty", import_event_files::<TestModel, Sample>),
                    EventStream::new("ZeroBytes", import_event_files::<TestModel, Sample>),
                    EventStream::new("Missing", import_event_files::<TestModel, Sample>),
                ];
                STREAMS
            }
        }

        fn context(root: &Path, id: Uuid, name: &str) -> PathBuf {
            let path = root.join(id.to_string());
            fs::create_dir_all(&path).unwrap();
            let mut model = ModelInfo::unknown();
            model.name = name.to_owned();
            ArtifactInfo::new(model).write_sidecar(&path).unwrap();
            path
        }

        fn sample(id: Uuid, timestamp: u64, sequence: u32) -> String {
            format!(r#"{{"id":"{id}","timestamp":{timestamp},"data":{{"sequence":{sequence}}}}}"#)
                + "\n"
        }

        #[test]
        fn loaded_context_distinguishes_all_stream_states_and_keeps_envelopes() {
            let root = tempfile::tempdir().unwrap();
            let id = Uuid::from_u128(11);
            let entity = Uuid::from_u128(99);
            let dir = context(root.path(), id, TestModel::NAME);
            for name in ["Alpha", "Empty", "ZeroBytes"] {
                fs::create_dir(dir.join(name)).unwrap();
            }
            fs::write(dir.join("ZeroBytes/events.ndjson"), b"").unwrap();
            fs::write(dir.join("Alpha/a.ndjson"), sample(entity, 30, 1)).unwrap();
            fs::write(dir.join("Alpha/b.ndjson"), sample(entity, 10, 2)).unwrap();
            // A file on disk is not a declaration in the model's schema.
            fs::create_dir(dir.join("Undeclared")).unwrap();
            fs::write(
                dir.join("Undeclared/events.ndjson"),
                b"not part of this model\n",
            )
            .unwrap();

            let store = Store::<TestModel>::new(root.path());
            let loaded = store.load_context(id).unwrap();
            assert_eq!(loaded.id(), id);
            assert_eq!(
                loaded.stream_availability("Alpha"),
                StreamAvailability::Populated
            );
            assert_eq!(
                loaded.stream_availability("Empty"),
                StreamAvailability::Empty
            );
            assert_eq!(
                loaded.stream_availability("ZeroBytes"),
                StreamAvailability::Empty
            );
            assert_eq!(
                loaded.stream_availability("Missing"),
                StreamAvailability::Missing
            );
            assert_eq!(
                loaded.stream_availability("Undeclared"),
                StreamAvailability::Undeclared
            );
            let values = |events: Vec<Event<Sample>>| {
                events
                    .into_iter()
                    .map(|event| (event.id, event.timestamp, event.data.sequence))
                    .collect::<Vec<_>>()
            };
            let expected = vec![(entity, 30, 1), (entity, 10, 2)];
            assert_eq!(
                values(store.events(id).unwrap().collect::<Result<_>>().unwrap()),
                expected
            );
            assert_eq!(
                values(
                    store
                        .entity_events::<Alpha>(id)
                        .unwrap()
                        .collect::<Result<_>>()
                        .unwrap()
                ),
                expected
            );
            assert_eq!(values(loaded.into_events()), expected);
        }

        #[test]
        fn invalid_models_and_broken_streams_never_return_partial_contexts() {
            let root = tempfile::tempdir().unwrap();
            let id = Uuid::from_u128(12);
            let dir = context(root.path(), id, "WrongModel");
            fs::create_dir(dir.join("Alpha")).unwrap();
            fs::write(dir.join("Alpha/a.ndjson"), sample(id, 1, 1)).unwrap();
            fs::write(dir.join("Alpha/b.ndjson"), b"invalid event\n").unwrap();
            let store = Store::<TestModel>::new(root.path());
            assert!(matches!(
                store.load_context(id),
                Err(Error::ModelMismatch { .. })
            ));
            context(root.path(), id, TestModel::NAME);
            assert!(matches!(
                store.load_context(id),
                Err(Error::Importer { .. })
            ));
            fs::write(dir.join("Alpha/b.ndjson"), sample(id, 2, 2)).unwrap();
            assert_eq!(store.load_context(id).unwrap().events().len(), 2);
            assert!(matches!(
                store.load_context(Uuid::nil()),
                Err(Error::ContextNotFound(_))
            ));
        }

        #[test]
        fn typed_stream_loader_preserves_missing_empty_and_decode_errors() {
            let dir = tempfile::tempdir().unwrap();
            assert!(load_entity_stream::<Sample>(dir.path()).unwrap().is_none());
            fs::write(dir.path().join("Alpha"), b"not a directory").unwrap();
            assert!(load_entity_stream::<Sample>(dir.path()).unwrap().is_none());
            fs::remove_file(dir.path().join("Alpha")).unwrap();
            fs::create_dir(dir.path().join("Alpha")).unwrap();
            assert!(
                load_entity_stream::<Sample>(dir.path())
                    .unwrap()
                    .unwrap()
                    .next()
                    .is_none()
            );
            fs::write(dir.path().join("Alpha/events.ndjson"), b"invalid event\n").unwrap();
            assert!(matches!(
                load_entity_stream::<Sample>(dir.path())
                    .unwrap()
                    .unwrap()
                    .next(),
                Some(Err(Error::Importer { .. }))
            ));
        }
    }
}
