// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { NavigationMenuItem } from '@quent/components';
import { DEEP_LINK_NAV_SLOT_ID } from './deepLink.constants';
import { useDeepLinkNavTarget } from './deepLinkNavTarget.context';

export function DeepLinkNavSlot() {
  const { setTarget } = useDeepLinkNavTarget();
  return <NavigationMenuItem ref={setTarget} id={DEEP_LINK_NAV_SLOT_ID} />;
}
