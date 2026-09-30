// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared loading and analysis lifecycle for asynchronous consumers.
//!
//! A cache key identifies an analysis within the cache's owning store/model.
//! Use [`ContextId`](crate::context::ContextId) for context reconstruction and
//! the application's target ID for aggregate analyses. Loading and analysis
//! run together on a bounded blocking pool, and failed loads remain retryable.

use std::{hash::Hash, num::NonZeroUsize, sync::Arc, time::Duration};

use moka::future::Cache;
use tokio::sync::Semaphore;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("analysis task pool closed: {0}")]
    Closed(#[from] tokio::sync::AcquireError),
    #[error("analysis task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// A shared bound on blocking work, acquired before spawning each task.
#[derive(Clone)]
pub struct BlockingTasks {
    permits: Arc<Semaphore>,
}

impl BlockingTasks {
    pub fn new(max_concurrency: NonZeroUsize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_concurrency.get())),
        }
    }

    /// Retains capacity until the task exits, even if its awaiting caller drops.
    pub async fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, TaskError> {
        let permit = Arc::clone(&self.permits).acquire_owned().await?;
        Ok(tokio::task::spawn_blocking(move || {
            let _permit = permit;
            task()
        })
        .await?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError<E> {
    #[error("{0}")]
    Load(E),
    #[error(transparent)]
    Task(#[from] TaskError),
}

/// Coalesces concurrent loads and retains only successfully analyzed results.
///
/// Cache instances belong to a particular store/model. Callers select a context
/// or analysis target by key; raw protocol handles are not suitable keys here.
/// The error type is fixed per instance so every request for a key participates
/// in the same in-flight load.
pub struct AnalysisCache<K, T, E> {
    results: Cache<K, Arc<T>>,
    tasks: BlockingTasks,
    error: std::marker::PhantomData<fn() -> E>,
}

impl<K, T, E> Clone for AnalysisCache<K, T, E> {
    fn clone(&self) -> Self {
        Self {
            results: self.results.clone(),
            tasks: self.tasks.clone(),
            error: self.error,
        }
    }
}

impl<K, T, E> AnalysisCache<K, T, E>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    pub fn new(capacity: u64, time_to_idle: Duration, tasks: BlockingTasks) -> Self {
        Self {
            results: Cache::builder()
                .max_capacity(capacity)
                .time_to_idle(time_to_idle)
                .build(),
            tasks,
            error: std::marker::PhantomData,
        }
    }

    /// Import and analyze on a miss, sharing one result for concurrent callers.
    ///
    /// Errors, including a missing stream/context reported by `load`, are never
    /// retained as successful entries. The next request can retry after repair.
    pub async fn get_with(
        &self,
        key: K,
        load: impl FnOnce() -> Result<T, E> + Send + 'static,
    ) -> Result<Arc<T>, Arc<AnalysisError<E>>> {
        let tasks = self.tasks.clone();
        self.results
            .try_get_with(key, async move {
                tasks
                    .run(load)
                    .await?
                    .map(Arc::new)
                    .map_err(AnalysisError::Load)
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::context::ContextId;
    use uuid::Uuid;

    use super::*;

    fn tasks(count: usize) -> BlockingTasks {
        BlockingTasks::new(NonZeroUsize::new(count).unwrap())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn runs_off_the_executor_and_releases_capacity_after_a_panic() {
        let tasks = tasks(1);
        let executor = std::thread::current().id();
        assert_ne!(
            tasks.run(|| std::thread::current().id()).await.unwrap(),
            executor
        );
        assert!(matches!(
            tasks.run(|| panic!("failed analysis")).await,
            Err(TaskError::Join(_))
        ));
        assert_eq!(tasks.run(|| 7).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn coalesces_concurrent_loads_and_isolates_contexts() {
        let cache: AnalysisCache<_, _, String> =
            AnalysisCache::new(8, Duration::from_secs(60), tasks(2));
        let context = ContextId::from(Uuid::from_u128(1));
        let calls = Arc::new(AtomicUsize::new(0));
        let a_calls = Arc::clone(&calls);
        let b_calls = Arc::clone(&calls);
        let (a, b) = tokio::join!(
            cache.get_with(context, move || {
                a_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>(7)
            }),
            cache.get_with(context, move || {
                b_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>(7)
            }),
        );
        let (a, b) = (a.unwrap(), b.unwrap());
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let other = cache
            .get_with(ContextId::from(Uuid::from_u128(2)), || Ok::<_, String>(9))
            .await
            .unwrap();
        assert_eq!(*other, 9);
        assert_eq!(*a, 7);
    }

    #[tokio::test]
    async fn failed_loads_are_retried_without_publishing_partial_results() {
        let cache: AnalysisCache<_, _, &'static str> =
            AnalysisCache::new(8, Duration::from_secs(60), tasks(1));
        let failed = cache
            .get_with(1_u32, || Err::<u32, _>("missing stream"))
            .await
            .unwrap_err();
        assert!(matches!(&*failed, AnalysisError::Load("missing stream")));
        let repaired = cache.get_with(1_u32, || Ok::<_, &str>(42)).await.unwrap();
        assert_eq!(*repaired, 42);
        let cached = cache
            .get_with(1_u32, || Err::<u32, _>("must not reload"))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&repaired, &cached));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_retains_capacity_until_the_blocking_task_exits() {
        let tasks = tasks(1);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let first_tasks = tasks.clone();
        let first = tokio::spawn(async move {
            first_tasks
                .run(move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
                .await
        });
        started_rx.await.unwrap();
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        assert_eq!(tasks.permits.available_permits(), 0);

        let (next_tx, mut next_rx) = tokio::sync::oneshot::channel();
        let next = tokio::spawn(async move { tasks.run(move || next_tx.send(()).unwrap()).await });
        tokio::task::yield_now().await;
        assert!(matches!(
            next_rx.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        release_tx.send(()).unwrap();
        next.await.unwrap().unwrap();
        next_rx.await.unwrap();
    }
}
