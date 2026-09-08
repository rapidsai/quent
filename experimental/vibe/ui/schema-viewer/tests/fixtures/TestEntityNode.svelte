<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { EntityNodeProps } from '../../src/public';

  let { schema, path }: EntityNodeProps = $props();
</script>

<span data-test-node>{schema.name}:{[...path.namespace, path.name].join('/')}</span>
