// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export const DEEP_LINK_COMPATIBILITY_PATH = '/profile/engine/engine-a/query/query-a/operators';

interface DeepLinkCompatibilityState {
  route: { engineId: string; queryId: string; tab: 'operators' };
  timeline: { zoomRange: { start: number; end: number } };
  selection: { operatorNodeIds: string[] };
}

export const DEEP_LINK_COMPATIBILITY_STATE: DeepLinkCompatibilityState = {
  route: { engineId: 'engine-a', queryId: 'query-a', tab: 'operators' },
  timeline: { zoomRange: { start: 0, end: 10 } },
  selection: { operatorNodeIds: ['operator-a'] },
};

export const DEEP_LINK_COMPATIBILITY_PAYLOAD =
  'v3.H4sIAAAAAAACEz2NsQ6DMAxE_-VmKsGaP-jSgRV1cIuFIkFcHDO0KP-Oidpud0_n5x0qmzHCDk5TTHwdEb7xQmiwbqzvCmuqzOjhXV6sZKIZxVFcePabU_QRWXpKUy3ZSA2hbVzqlq4tvs4889OipHPx89xk9O8ZYfgj_3Yv5QB2zVL6pAAAAA';
