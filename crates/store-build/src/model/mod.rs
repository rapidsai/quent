// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of model-wide types.

use proc_macro2::TokenStream;
use quent_schema::Schema;

use crate::GenerateError;

mod store;

#[cfg(test)]
mod tests;

/// Generates model-wide stores.
pub(super) fn generate(schema: &Schema, events: &syn::File) -> Result<TokenStream, GenerateError> {
    let code = store::generate(schema, events)?;
    Ok(store::generate_model(&code))
}
