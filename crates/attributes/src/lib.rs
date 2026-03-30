// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Support for custom attributes defined at run-time.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

/// Error type for Value conversions.
#[derive(Error, Debug)]
pub enum ValueError {
    #[error("not numeric: {0}")]
    NotNumeric(String),
}

/// A group of [`Attribute`]s.
#[derive(TS, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Struct(pub Vec<Attribute>);

/// A sequence of [`Value`]s.
#[derive(TS, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[ts(untagged)]
pub enum List {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
    Struct(Vec<Struct>),
}

/// An [`Attribute`] value.
#[derive(TS, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[ts(untagged)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Struct(Struct),
    List(List),
}

/// A key-value pair.
#[derive(TS, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Attribute {
    pub key: String,
    pub value: Option<Value>,
}

impl Attribute {
    /// Create a new attribute with the given key and no value.
    pub fn null(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
        }
    }

    /// Create an attribute with a u8 value.
    pub fn u8(key: impl Into<String>, value: u8) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::U8(value)),
        }
    }

    /// Create an attribute with a u16 value.
    pub fn u16(key: impl Into<String>, value: u16) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::U16(value)),
        }
    }

    /// Create an attribute with a u32 value.
    pub fn u32(key: impl Into<String>, value: u32) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::U32(value)),
        }
    }

    /// Create an attribute with a u64 value.
    pub fn u64(key: impl Into<String>, value: u64) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::U64(value)),
        }
    }

    /// Create an attribute with an i8 value.
    pub fn i8(key: impl Into<String>, value: i8) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::I8(value as u8)),
        }
    }

    /// Create an attribute with an i16 value.
    pub fn i16(key: impl Into<String>, value: i16) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::I16(value)),
        }
    }

    /// Create an attribute with an i32 value.
    pub fn i32(key: impl Into<String>, value: i32) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::I32(value)),
        }
    }

    /// Create an attribute with an i64 value.
    pub fn i64(key: impl Into<String>, value: i64) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::I64(value)),
        }
    }

    /// Create an attribute with an f32 value.
    pub fn f32(key: impl Into<String>, value: f32) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::F32(value)),
        }
    }

    /// Create an attribute with an f64 value.
    pub fn f64(key: impl Into<String>, value: f64) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::F64(value)),
        }
    }

    /// Create an attribute with a String value.
    pub fn string(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::String(value.into())),
        }
    }

    /// Create an attribute with a Struct value.
    pub fn structure(key: impl Into<String>, value: Struct) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::Struct(value)),
        }
    }

    /// Create an attribute with a List value.
    pub fn list(key: impl Into<String>, value: List) -> Self {
        Self {
            key: key.into(),
            value: Some(Value::List(value)),
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::U8(v) => Ok(v as f64),
            Value::U16(v) => Ok(v as f64),
            Value::U32(v) => Ok(v as f64),
            Value::U64(v) => Ok(v as f64),
            Value::I8(v) => Ok(v as f64),
            Value::I16(v) => Ok(v as f64),
            Value::I32(v) => Ok(v as f64),
            Value::I64(v) => Ok(v as f64),
            Value::F32(v) => Ok(v as f64),
            Value::F64(v) => Ok(v),
            Value::String(_) => Err(ValueError::NotNumeric("String".to_string())),
            Value::Struct(_) => Err(ValueError::NotNumeric("Struct".to_string())),
            Value::List(_) => Err(ValueError::NotNumeric("List".to_string())),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = ValueError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::U8(v) => Ok(*v as f64),
            Value::U16(v) => Ok(*v as f64),
            Value::U32(v) => Ok(*v as f64),
            Value::U64(v) => Ok(*v as f64),
            Value::I8(v) => Ok(*v as f64),
            Value::I16(v) => Ok(*v as f64),
            Value::I32(v) => Ok(*v as f64),
            Value::I64(v) => Ok(*v as f64),
            Value::F32(v) => Ok(*v as f64),
            Value::F64(v) => Ok(*v),
            Value::String(_) => Err(ValueError::NotNumeric("String".to_string())),
            Value::Struct(_) => Err(ValueError::NotNumeric("Struct".to_string())),
            Value::List(_) => Err(ValueError::NotNumeric("List".to_string())),
        }
    }
}
