// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    model: PathBuf,
    targets: Vec<Target>,
    package: Package,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Target {
    Rust,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    name: String,
    version: String,
}

impl Manifest {
    fn validate(&self) -> Result<(), String> {
        if self.model.as_os_str().is_empty() {
            return Err("`model` must not be empty".into());
        }
        if self.targets.is_empty() {
            return Err("`targets` must not be empty".into());
        }
        for (index, target) in self.targets.iter().enumerate() {
            if self.targets[..index].contains(target) {
                return Err("`targets` must not contain duplicates".into());
            }
        }
        if self.package.name.is_empty()
            || !self
                .package
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(
                "`package.name` must contain only ASCII letters, digits, hyphens, or underscores and must not be empty"
                    .into(),
            );
        }
        semver::Version::parse(&self.package.version)
            .map_err(|error| format!("invalid `package.version`: {error}"))?;
        Ok(())
    }
}

pub(crate) fn check(path: &Path) -> Result<Vec<quent_yaml::Diagnostic>, String> {
    let source =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest: Manifest =
        toml::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))?;
    manifest
        .validate()
        .map_err(|error| format!("{}: {error}", path.display()))?;

    let model = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(manifest.model);
    let parsed = quent_yaml::parse_from_file(&model).map_err(|error| match error {
        quent_yaml::Error::Io(error) => format!("{}: {error}", model.display()),
        quent_yaml::Error::Invalid(diagnostics) => diagnostics.to_string(),
    })?;
    Ok(parsed.warnings)
}
