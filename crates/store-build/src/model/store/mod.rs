// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of model stores across supported backends.

use proc_macro2::TokenStream;
use quent_schema::Schema;

use crate::GenerateError;

mod native;

/// Store fragments for each supported backend of one schema model.
pub(super) struct ModelStoreCode {
    /// Fragments for the native in-memory backend.
    native: native::NativeModelStoreCode,
}

/// Generates model-store declarations and implementations for supported backends.
pub(super) fn generate(
    schema: &Schema,
    events: &syn::File,
) -> Result<ModelStoreCode, GenerateError> {
    Ok(ModelStoreCode {
        native: native::generate(schema, events)?,
    })
}

/// Assembles model stores from their backend fragments.
pub(super) fn generate_model(code: &ModelStoreCode) -> TokenStream {
    native::generate_model(&code.native)
}
