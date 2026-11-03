// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export interface AskCommand {
  id: string;
  explanation: string;
  usage: string;
  run(args: string[]): Promise<void>;
}
