// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quent version compatibility for YAML model sources.

use crate::diag::Diagnostics;

const SUPPORTED_VERSIONS: &str = ">=0.1, <0.2";

pub(crate) fn validate(version: &str, sink: &mut Diagnostics) {
    let parsed = if version == "alpha" {
        Ok(semver::Version::new(0, 1, 0))
    } else if version.split('.').count() == 2 {
        semver::Version::parse(&format!("{version}.0"))
    } else {
        semver::Version::parse(version)
    };
    let supported = match semver::VersionReq::parse(SUPPORTED_VERSIONS) {
        Ok(supported) => supported,
        Err(error) => {
            sink.error(
                "quent",
                format!("invalid supported Quent version range `{SUPPORTED_VERSIONS}`: {error}"),
                None,
            );
            return;
        }
    };
    if !parsed.is_ok_and(|version| supported.matches(&version)) {
        sink.error(
            "quent",
            format!("unsupported Quent version `{version}`"),
            Some(format!(
                "supported Quent versions: {SUPPORTED_VERSIONS}; legacy alias: alpha"
            )),
        );
    }
}
