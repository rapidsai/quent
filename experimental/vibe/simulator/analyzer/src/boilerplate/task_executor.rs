// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::task_executor::{TaskExecutorEventStorage, TaskExecutorEvents};

use super::*;

pub(crate) type TaskExecutor = StoredEntity<schema::TaskExecutor, TaskExecutorEventStorage>;

impl RefTreeEntity for TaskExecutor {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.worker_id.target)
    }
}
