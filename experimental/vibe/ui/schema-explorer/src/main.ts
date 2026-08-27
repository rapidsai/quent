// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { mount } from 'svelte';

import App from './App.svelte';
import './style.css';

mount(App, { target: document.getElementById('app')! });
