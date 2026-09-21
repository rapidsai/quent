// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl TransitionEvent for QueryEvent {
    fn sequence(&self) -> u16 {
        match self {
            Self::Init { seq, .. }
            | Self::Planning { seq }
            | Self::Executing { seq }
            | Self::Done { seq } => *seq,
        }
    }

    fn is_initial(&self) -> bool {
        matches!(self, Self::Init { .. })
    }

    fn is_final(&self) -> bool {
        matches!(self, Self::Done { .. })
    }

    fn is_valid_next(&self, next: &Self) -> bool {
        match self {
            Self::Init { .. } => matches!(next, Self::Planning { .. }),
            Self::Planning { .. } => matches!(next, Self::Executing { .. }),
            Self::Executing { .. } => matches!(next, Self::Done { .. }),
            Self::Done { .. } => false,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Init { .. } => "init",
            Self::Planning { .. } => "planning",
            Self::Executing { .. } => "executing",
            Self::Done { .. } => "done",
        }
    }
}
