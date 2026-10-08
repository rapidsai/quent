# Instrumentation

The canonical way to use Quent is to first instrument your code with an
application-specific instrumentation library. Quent generates this library
entirely from an **Application Event Schema**. From here on, this tutorial
refers to it simply as a schema.

You can define a schema in several ways, including with a YAML-based DSL or
programmatically. This tutorial focuses on the YAML-based approach only. To do
it programmatically, check the
[`quent-schema` crate documentation][quent-schema].

[quent-schema]: https://github.com/rapidsai/quent/blob/main/crates/schema/src/lib.rs
