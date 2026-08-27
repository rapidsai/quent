// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { NavigationMenuItem } from '@quent/components';
import { DEEP_LINK_NAV_SLOT_ID } from './deepLink.constants';

export function DeepLinkNavSlot() {
  return <NavigationMenuItem id={DEEP_LINK_NAV_SLOT_ID} />;
}
