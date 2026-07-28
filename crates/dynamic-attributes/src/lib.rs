// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed attributes whose keys are defined at runtime.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Error returned when converting a [`DynamicValue`].
#[derive(Error, Debug)]
pub enum DynamicValueError {
    #[error("not numeric: {0}")]
    NotNumeric(String),
}

/// A group of [`DynamicAttribute`]s.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "ts", derive(TS))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicStruct(pub Vec<DynamicAttribute>);

/// A sequence of [`DynamicValue`]s.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "ts", derive(TS), ts(untagged))]
#[derive(Clone, Debug, PartialEq)]
pub enum DynamicList {
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
    Struct(Vec<DynamicStruct>),
}

/// A [`DynamicAttribute`] value.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "ts", derive(TS), ts(untagged))]
#[derive(Clone, Debug, PartialEq)]
pub enum DynamicValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Struct(DynamicStruct),
    List(DynamicList),
}

/// A key-value pair.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "ts", derive(TS))]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicAttribute {
    pub key: String,
    pub value: Option<DynamicValue>,
}

impl DynamicAttribute {
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
            value: Some(DynamicValue::U8(value)),
        }
    }

    /// Create an attribute with a u16 value.
    pub fn u16(key: impl Into<String>, value: u16) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::U16(value)),
        }
    }

    /// Create an attribute with a u32 value.
    pub fn u32(key: impl Into<String>, value: u32) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::U32(value)),
        }
    }

    /// Create an attribute with a u64 value.
    pub fn u64(key: impl Into<String>, value: u64) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::U64(value)),
        }
    }

    /// Create an attribute with an i8 value.
    pub fn i8(key: impl Into<String>, value: i8) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::I8(value)),
        }
    }

    /// Create an attribute with an i16 value.
    pub fn i16(key: impl Into<String>, value: i16) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::I16(value)),
        }
    }

    /// Create an attribute with an i32 value.
    pub fn i32(key: impl Into<String>, value: i32) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::I32(value)),
        }
    }

    /// Create an attribute with an i64 value.
    pub fn i64(key: impl Into<String>, value: i64) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::I64(value)),
        }
    }

    /// Create an attribute with an f32 value.
    pub fn f32(key: impl Into<String>, value: f32) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::F32(value)),
        }
    }

    /// Create an attribute with an f64 value.
    pub fn f64(key: impl Into<String>, value: f64) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::F64(value)),
        }
    }

    /// Create an attribute with a String value.
    pub fn string(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::String(value.into())),
        }
    }

    /// Create an attribute with a struct value.
    pub fn structure(key: impl Into<String>, value: DynamicStruct) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::Struct(value)),
        }
    }

    /// Create an attribute with a list value.
    pub fn list(key: impl Into<String>, value: DynamicList) -> Self {
        Self {
            key: key.into(),
            value: Some(DynamicValue::List(value)),
        }
    }
}

/// A collection of attributes whose keys are defined at runtime.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize), serde(transparent))]
pub struct DynamicAttributes(pub Vec<DynamicAttribute>);

impl DynamicAttributes {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, attr: DynamicAttribute) {
        self.0.push(attr);
    }

    pub fn add_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.push(DynamicAttribute::string(key, value));
    }

    pub fn add_u64(&mut self, key: impl Into<String>, value: u64) {
        self.0.push(DynamicAttribute::u64(key, value));
    }

    pub fn add_i64(&mut self, key: impl Into<String>, value: i64) {
        self.0.push(DynamicAttribute::i64(key, value));
    }

    pub fn add_f64(&mut self, key: impl Into<String>, value: f64) {
        self.0.push(DynamicAttribute::f64(key, value));
    }

    pub fn add_bool(&mut self, key: impl Into<String>, value: bool) {
        self.0.push(DynamicAttribute {
            key: key.into(),
            value: Some(if value {
                DynamicValue::U8(1)
            } else {
                DynamicValue::U8(0)
            }),
        });
    }

    pub fn into_vec(self) -> Vec<DynamicAttribute> {
        self.0
    }
}

impl std::ops::Deref for DynamicAttributes {
    type Target = Vec<DynamicAttribute>;
    fn deref(&self) -> &Vec<DynamicAttribute> {
        &self.0
    }
}

impl From<Vec<DynamicAttribute>> for DynamicAttributes {
    fn from(v: Vec<DynamicAttribute>) -> Self {
        Self(v)
    }
}

impl From<DynamicAttributes> for Vec<DynamicAttribute> {
    fn from(v: DynamicAttributes) -> Self {
        v.0
    }
}

impl TryFrom<DynamicValue> for f64 {
    type Error = DynamicValueError;

    fn try_from(value: DynamicValue) -> Result<Self, Self::Error> {
        match value {
            DynamicValue::U8(v) => Ok(v as f64),
            DynamicValue::U16(v) => Ok(v as f64),
            DynamicValue::U32(v) => Ok(v as f64),
            DynamicValue::U64(v) => Ok(v as f64),
            DynamicValue::I8(v) => Ok(v as f64),
            DynamicValue::I16(v) => Ok(v as f64),
            DynamicValue::I32(v) => Ok(v as f64),
            DynamicValue::I64(v) => Ok(v as f64),
            DynamicValue::F32(v) => Ok(v as f64),
            DynamicValue::F64(v) => Ok(v),
            DynamicValue::String(_) => Err(DynamicValueError::NotNumeric("String".to_string())),
            DynamicValue::Struct(_) => Err(DynamicValueError::NotNumeric("Struct".to_string())),
            DynamicValue::List(_) => Err(DynamicValueError::NotNumeric("List".to_string())),
        }
    }
}

impl TryFrom<&DynamicValue> for f64 {
    type Error = DynamicValueError;

    fn try_from(value: &DynamicValue) -> Result<Self, Self::Error> {
        match value {
            DynamicValue::U8(v) => Ok(*v as f64),
            DynamicValue::U16(v) => Ok(*v as f64),
            DynamicValue::U32(v) => Ok(*v as f64),
            DynamicValue::U64(v) => Ok(*v as f64),
            DynamicValue::I8(v) => Ok(*v as f64),
            DynamicValue::I16(v) => Ok(*v as f64),
            DynamicValue::I32(v) => Ok(*v as f64),
            DynamicValue::I64(v) => Ok(*v as f64),
            DynamicValue::F32(v) => Ok(*v as f64),
            DynamicValue::F64(v) => Ok(*v),
            DynamicValue::String(_) => Err(DynamicValueError::NotNumeric("String".to_string())),
            DynamicValue::Struct(_) => Err(DynamicValueError::NotNumeric("Struct".to_string())),
            DynamicValue::List(_) => Err(DynamicValueError::NotNumeric("List".to_string())),
        }
    }
}
