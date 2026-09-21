// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Temporary analysis implementations for generated schema events.

// TODO(johanpel): Generate this module from schema metadata. See
// https://github.com/rapidsai/quent/issues/288.

use crate::{QueryEvent, TaskEvent};
use quent_analyzer::{
    fsm::native::TransitionEvent,
    resource::{AnalyzedUsage, CapacityValue},
};
use smallvec::{SmallVec, smallvec};

mod query;
mod task;
