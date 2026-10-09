// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::task_executor_thread::{
    TaskExecutorThreadEventStorage, TaskExecutorThreadEvents,
};

use super::*;

pub(crate) type TaskExecutorThread =
    StoredEntity<schema::TaskExecutorThread, TaskExecutorThreadEventStorage>;

impl TaskExecutorThread {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::unit("task_executor_thread")
    }
}

impl RefTreeEntity for TaskExecutorThread {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.task_executor_id.target)
    }
}

impl Resource for TaskExecutorThread {}
