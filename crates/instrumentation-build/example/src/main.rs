// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

pub mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() {
    let opened = demo::ConnectionEvent::Opened {
        peer: demo::Endpoint {
            host: "localhost".to_owned(),
            port: 8080,
        },
        session: uuid::Uuid::nil(),
    };
    dbg!(opened);
}
