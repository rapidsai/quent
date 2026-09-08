# Attributes

An Attribute is a pair consisting of a key and a typed value.

In this specification, Attributes are denoted as follows:

- `<key name>: <value type>`.

This specification aims to describe only an absolute minimal set of Attributes
with a pre-defined meaning necessary to relate modeling constructs in a
meaningful way. Any application-specific model and implementation of the
instrumentation thereof may choose to add a set of arbitrary Attributes (to e.g.
[Transitions][transition] of [FSMs][finite-state-machine]) as long as they do
not replace Attributes that have been assigned a specific meaning by this
specification.

## Value types

Attribute values are of the following types.

### Non-numeric primitive types

- Boolean (`bool`)
- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (`uuid`)
- UTF-8 strings (`string`)

### Numeric primitive types

- Unsigned integers of size 8, 16, 32, or 64 bits (`u{8,16,32,64}`)
- Signed integers of size 8, 16, 32, or 64 bits (`i{8,16,32,64}`)
- IEEE 754 floating-point values of types _binary32_ and _binary64_ (`f32`,
  `f64`)

### Compound types

- Lists of variable lengths between `[0, 2^64-1]` of exactly one of the above
  types, that may be empty (`list<T>` where `T` is one of the above).
  Nested lists (`list<list<T>>`) are not yet supported
  (see [#79](https://github.com/rapidsai/quent/issues/79)).
- A set of Attributes (`struct { field_name_1: T, field_name_2: U, ... }`)

Implementations may choose to explicitly provide an alias for variable-length
list of 8-bit unsigned integers (`list<u8>`) to capture binary data.

This specification explicitly forbids the use of architecture-specific
pointer-sized integers (such as `usize` in Rust, or `size_t` in C++).
This ensures telemetry is portable and deterministically serializable across
architectures with different pointer widths.

## Keys

If constructs described in this specification allow having arbitrary run-time
defined [Attributes][attributes], the names of arbitrary keys (of
application-specific key-value pairs not defined by this specification) must be
of the type `string`.

Names of predefined keys shall use alphanumeric characters (A..Z, a..z, 0..9)
and underscores (`_`) only, starting with a non-digit.

## Nullability

Attribute values may be optional, also known as nullable, i.e. their value may
not exist. To denote nullability, this specification will denote such attributes
as being of type `option<T>` where `T` is the non-null value type, or list them
under a "may have" section. Null values are expressed as `none`.

[attributes]: #attributes
[finite-state-machine]: ./fsm.md
[transition]: ./fsm.md#transition
