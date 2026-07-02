// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use smallvec::SmallVec;

use crate::{Annotations, DataType, Entity, Event, Field, Record, Schema};

/// Trait for types that can visit a schema and its elements.
pub trait Visitor {
    /// The output type after the entire schema is walked.
    type Output;

    /// Visit a schema element at the `cursor` location.
    fn visit(&mut self, cursor: &Cursor);

    /// Finish visiting the schema and produce output.
    fn finish(self) -> Self::Output;
}

impl Schema {
    /// Walk this schema with `visitor`, returning its
    /// [`Visitor::Output`].
    ///
    /// Every element is visited exactly once in the following order:
    /// 1. the element itself, before any of its children;
    /// 2. its own [`Annotations`], first among its children;
    /// 3. its children
    ///
    /// A field's [`DataType`] is visited by variant:
    /// 1. [`DataType::Option`] and [`DataType::List`] visit their inner
    ///    type.
    /// 2. [`DataType::EntityRef`] first visits its [`Annotations`], then
    ///    its carried data type (if any).
    /// 3. `[DataType::Record`] is considered a leaf. Its constituents are
    ///    visited once from [`Schema::records`] instead.
    pub fn walk<T: Visitor>(&self, mut visitor: T) -> T::Output {
        let mut cursor = Cursor::new(self);

        visitor.visit(&cursor);
        walk_annotations(&mut cursor, &mut visitor, self.annotations());
        for entity in self.entities() {
            walk_entity(&mut cursor, &mut visitor, entity);
        }
        for record in self.records() {
            walk_record(&mut cursor, &mut visitor, record);
        }

        visitor.finish()
    }
}

/// Reference to a schema element.
#[derive(Clone, Copy, PartialEq)]
pub enum Element<'s> {
    Schema(&'s Schema),
    Annotations(&'s Annotations),
    Entity(&'s Entity),
    Event(&'s Event),
    Field(&'s Field),
    Record(&'s Record),
    DataType(&'s DataType),
}

/// Path to the [`Schema`] element currently being visited.
///
/// The root is always the [`Schema`] itself.
#[derive(Clone, PartialEq)]
pub struct Cursor<'s>(SmallVec<[Element<'s>; 5]>);

impl<'s> Cursor<'s> {
    pub fn new(schema: &'s Schema) -> Self {
        let mut path = SmallVec::new();
        path.push(Element::Schema(schema));
        Self(path)
    }
    /// The element currently being visited.
    pub fn current(&self) -> Element<'s> {
        // unwrap ok: inner vec can't be constructed without a schema
        self.0.last().copied().unwrap()
    }
    /// The parent of the current element, if any.
    pub fn previous(&self) -> Option<Element<'s>> {
        self.0.iter().rev().nth(1).copied()
    }
    /// The schema being walked.
    pub fn root(&self) -> &'s Schema {
        match self.0.first() {
            Some(Element::Schema(schema)) => schema,
            _ => unreachable!(),
        }
    }
    /// The full path, from the schema root to the current element.
    pub fn elements(&self) -> &[Element<'s>] {
        &self.0
    }
    fn enter(&mut self, element: Element<'s>) {
        self.0.push(element);
    }
    fn leave(&mut self) {
        self.0.pop();
    }
}

impl Display for Element<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(s) => Display::fmt(s.name(), f),
            Self::Entity(e) => Display::fmt(e.name(), f),
            Self::Event(e) => Display::fmt(e.name(), f),
            Self::Record(r) => Display::fmt(r.name(), f),
            Self::Field(fi) => Display::fmt(fi.name(), f),
            Self::Annotations(_) => f.write_str("Annotations"),
            Self::DataType(ty) => match ty {
                DataType::Bool => f.write_str("Bool"),
                DataType::Uuid => f.write_str("UUID"),
                DataType::String => f.write_str("String"),
                DataType::U8 => f.write_str("U8"),
                DataType::U16 => f.write_str("U16"),
                DataType::U32 => f.write_str("U32"),
                DataType::U64 => f.write_str("U64"),
                DataType::I8 => f.write_str("I8"),
                DataType::I16 => f.write_str("I16"),
                DataType::I32 => f.write_str("I32"),
                DataType::I64 => f.write_str("I64"),
                DataType::F32 => f.write_str("F32"),
                DataType::F64 => f.write_str("F64"),
                DataType::Option(_) => f.write_str("Option"),
                DataType::List(_) => f.write_str("List"),
                DataType::Record(name) => write!(f, "Record({})", name),
                DataType::DynamicRecord => f.write_str("DynamicRecord"),
                DataType::EntityRef { .. } => f.write_str("EntityRef"),
            },
        }
    }
}

impl Display for Cursor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            Display::fmt(first, f)?;
            for element in iter {
                write!(f, ".{element}")?;
            }
        }
        Ok(())
    }
}

fn walk_annotations<'s, V: Visitor>(
    cursor: &mut Cursor<'s>,
    visitor: &mut V,
    annotations: &'s Annotations,
) {
    cursor.enter(Element::Annotations(annotations));
    visitor.visit(cursor);
    cursor.leave();
}

fn walk_entity<'s, V: Visitor>(cursor: &mut Cursor<'s>, visitor: &mut V, entity: &'s Entity) {
    cursor.enter(Element::Entity(entity));
    visitor.visit(cursor);
    walk_annotations(cursor, visitor, entity.annotations());
    for event in entity.events() {
        walk_event(cursor, visitor, event);
    }
    cursor.leave();
}

fn walk_event<'s, V: Visitor>(cursor: &mut Cursor<'s>, visitor: &mut V, event: &'s Event) {
    cursor.enter(Element::Event(event));
    visitor.visit(cursor);
    walk_annotations(cursor, visitor, event.annotations());
    for field in event.fields() {
        walk_field(cursor, visitor, field);
    }
    cursor.leave();
}

fn walk_record<'s, V: Visitor>(cursor: &mut Cursor<'s>, visitor: &mut V, record: &'s Record) {
    cursor.enter(Element::Record(record));
    visitor.visit(cursor);
    walk_annotations(cursor, visitor, record.annotations());
    for field in record.fields() {
        walk_field(cursor, visitor, field);
    }
    cursor.leave();
}

fn walk_field<'s, V: Visitor>(cursor: &mut Cursor<'s>, visitor: &mut V, field: &'s Field) {
    cursor.enter(Element::Field(field));
    visitor.visit(cursor);
    walk_annotations(cursor, visitor, field.annotations());
    walk_data_type(cursor, visitor, field.ty());
    cursor.leave();
}

fn walk_data_type<'s, V: Visitor>(cursor: &mut Cursor<'s>, visitor: &mut V, ty: &'s DataType) {
    cursor.enter(Element::DataType(ty));
    visitor.visit(cursor);
    match ty {
        DataType::Option(inner) | DataType::List(inner) => {
            walk_data_type(cursor, visitor, inner);
        }
        DataType::EntityRef { data, annotations } => {
            walk_annotations(cursor, visitor, annotations);
            if let Some(inner) = data {
                walk_data_type(cursor, visitor, inner);
            }
        }
        _ => {}
    }
    cursor.leave();
}

// The empty visitor: visits nothing, produces nothing. Lets a walk run with no
// visitor of its own.
impl Visitor for () {
    type Output = ();
    fn visit(&mut self, _cursor: &Cursor) {}
    fn finish(self) -> Self::Output {}
}

// Macro to create impls for tuples of visitors, so output can be collected
// without having to upcast.
macro_rules! tuple_impls {
    ($($T:ident => $idx:tt),+) => {
        impl<$($T: Visitor),+> Visitor for ($($T,)+) {
            type Output = ($($T::Output,)+);
            fn visit(&mut self, cursor: &Cursor) {
                $( self.$idx.visit(cursor); )+
            }
            fn finish(self) -> Self::Output {
                ($( self.$idx.finish(), )+)
            }
        }
    };
}
tuple_impls!(A => 0);
tuple_impls!(A => 0, B => 1);
tuple_impls!(A => 0, B => 1, C => 2);
tuple_impls!(A => 0, B => 1, C => 2, D => 3);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8, J => 9);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8, J => 9, K => 10);
tuple_impls!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8, J => 9, K => 10, L => 11);

#[cfg(test)]
mod test {
    use super::*;
    use crate::builder::AnnotationsBuilder;
    use crate::test_utils::{entity, event, field, ident, record, schema};

    // Stateless no-op visitor
    #[derive(Default)]
    struct FooVisitor;
    impl Visitor for FooVisitor {
        type Output = ();
        fn visit(&mut self, _cursor: &Cursor) {}
        fn finish(self) -> Self::Output {}
    }
    // Stateful visitor.
    #[derive(Default)]
    struct BarVisitor {
        beers: u8,
    }
    impl Visitor for BarVisitor {
        type Output = u8;
        fn visit(&mut self, _cursor: &Cursor) {
            self.beers += 1;
        }
        fn finish(self) -> u8 {
            self.beers
        }
    }

    // Counts how often each kind of element is visited.
    #[derive(Default)]
    struct ElementCounter {
        schemas: usize,
        annotations: usize,
        entities: usize,
        events: usize,
        fields: usize,
        records: usize,
        data_types: usize,
    }
    impl Visitor for ElementCounter {
        type Output = ElementCounter;
        fn visit(&mut self, cursor: &Cursor) {
            match cursor.current() {
                Element::Schema(_) => self.schemas += 1,
                Element::Annotations(_) => self.annotations += 1,
                Element::Entity(_) => self.entities += 1,
                Element::Event(_) => self.events += 1,
                Element::Field(_) => self.fields += 1,
                Element::Record(_) => self.records += 1,
                Element::DataType(_) => self.data_types += 1,
            }
        }
        fn finish(self) -> ElementCounter {
            self
        }
    }

    fn sample_schema() -> Schema {
        schema(
            "S",
            vec![entity(
                "E",
                vec![event("Ev", vec![field("f", DataType::U64)])],
            )],
            vec![record("R", vec![field("rf", DataType::U64)])],
        )
    }

    #[test]
    fn visits_every_element() {
        let counter = sample_schema().walk(ElementCounter::default());
        assert_eq!(counter.schemas, 1);
        assert_eq!(counter.entities, 1);
        assert_eq!(counter.events, 1);
        assert_eq!(counter.fields, 2);
        assert_eq!(counter.records, 1);
        assert_eq!(counter.data_types, 2);
        assert_eq!(counter.annotations, 6);
    }

    #[test]
    fn recurses_data_types_and_entity_ref_annotations() {
        let entity_ref = DataType::EntityRef {
            data: None,
            annotations: AnnotationsBuilder::new()
                .constraint("my.constraint.v1", None)
                .unwrap()
                .build(),
        };
        let schema = schema(
            "S",
            vec![entity(
                "E",
                vec![event(
                    "Ev",
                    vec![field(
                        "x",
                        DataType::Option(Box::new(DataType::List(Box::new(entity_ref)))),
                    )],
                )],
            )],
            vec![],
        );

        let census = schema.walk(ElementCounter::default());
        // Option -> List -> EntityRef.
        assert_eq!(census.data_types, 3);
        // Schema, Entity, Event, the Field, and the EntityRef's own annotations.
        assert_eq!(census.annotations, 5);
    }

    #[test]
    fn tuple_visitor_collects_each_output() {
        let (counter, _, beers) =
            sample_schema().walk((ElementCounter::default(), FooVisitor, BarVisitor::default()));
        let total = counter.schemas
            + counter.annotations
            + counter.entities
            + counter.events
            + counter.fields
            + counter.records
            + counter.data_types;
        assert_eq!(total as u8, beers);
    }

    #[test]
    fn cursor_is_available_during_visit() {
        struct Probe;
        impl Visitor for Probe {
            type Output = ();
            fn visit(&mut self, cursor: &Cursor) {
                // The root is always the schema being walked.
                assert_eq!(*cursor.root().name(), ident("S"));
                // The path begins at the schema root.
                assert!(matches!(
                    cursor.elements().first(),
                    Some(Element::Schema(_))
                ));
                // A field's parent is the event or record that declares it.
                if let Element::Field(_) = cursor.current() {
                    assert!(matches!(
                        cursor.previous(),
                        Some(Element::Event(_) | Element::Record(_))
                    ));
                }
                // Declared names resolve through the root mid-walk.
                if let Element::Schema(_) = cursor.current() {
                    assert!(cursor.root().entity(&ident("E")).is_some());
                    assert!(cursor.root().record(&ident("R")).is_some());
                    assert!(cursor.root().entity(&ident("nope")).is_none());
                }
            }
            fn finish(self) {}
        }

        sample_schema().walk(Probe);
    }

    #[test]
    fn record_reference_is_a_leaf() {
        let schema = schema(
            "S",
            vec![entity(
                "E",
                vec![event("Ev", vec![field("f", DataType::Record(ident("R")))])],
            )],
            vec![record("R", vec![field("rf", DataType::U64)])],
        );

        let counter = schema.walk(ElementCounter::default());
        // The `Record(R)` reference is one DataType leaf; R's field is visited
        // only under records, not inlined at the reference.
        assert_eq!(counter.fields, 2);
        assert_eq!(counter.data_types, 2);
        assert_eq!(counter.records, 1);
    }

    #[test]
    fn walks_empty_schema() {
        let schema = schema("S", vec![], vec![]);

        let counter = schema.walk(ElementCounter::default());
        // Only the schema node and its annotations are visited.
        assert_eq!(counter.schemas, 1);
        assert_eq!(counter.annotations, 1);
        assert_eq!(counter.entities, 0);
        assert_eq!(counter.events, 0);
        assert_eq!(counter.fields, 0);
        assert_eq!(counter.records, 0);
        assert_eq!(counter.data_types, 0);
    }

    #[test]
    fn cursor_displays_as_dot_path() {
        struct Capture(Vec<String>);
        impl Visitor for Capture {
            type Output = Vec<String>;
            fn visit(&mut self, cursor: &Cursor) {
                self.0.push(cursor.to_string());
            }
            fn finish(self) -> Vec<String> {
                self.0
            }
        }

        // One field per DataType variant so every path segment is exercised.
        let event = event(
            "Ev",
            vec![
                field("bool_f", DataType::Bool),
                field("uuid_f", DataType::Uuid),
                field("str_f", DataType::String),
                field("u8_f", DataType::U8),
                field("u16_f", DataType::U16),
                field("u32_f", DataType::U32),
                field("u64_f", DataType::U64),
                field("i8_f", DataType::I8),
                field("i16_f", DataType::I16),
                field("i32_f", DataType::I32),
                field("i64_f", DataType::I64),
                field("f32_f", DataType::F32),
                field("f64_f", DataType::F64),
                field("opt_f", DataType::Option(Box::new(DataType::U64))),
                field("list_f", DataType::List(Box::new(DataType::Bool))),
                field("rec_f", DataType::Record(ident("R"))),
                field("dyn_f", DataType::DynamicRecord),
                field(
                    "ref_f",
                    DataType::EntityRef {
                        data: Some(Box::new(DataType::U32)),
                        annotations: Annotations::default(),
                    },
                ),
            ],
        );
        let schema = schema(
            "S",
            vec![entity("E", vec![event])],
            vec![record("R", vec![])],
        );

        let paths = schema.walk(Capture(Vec::new()));

        // Root and structural paths.
        assert_eq!(paths[0], "S");
        assert!(paths.contains(&"S.Annotations".to_string()));
        assert!(paths.contains(&"S.E.Ev".to_string()));

        // Scalar types.
        assert!(paths.contains(&"S.E.Ev.bool_f.Bool".to_string()));
        assert!(paths.contains(&"S.E.Ev.uuid_f.UUID".to_string()));
        assert!(paths.contains(&"S.E.Ev.str_f.String".to_string()));
        assert!(paths.contains(&"S.E.Ev.u8_f.U8".to_string()));
        assert!(paths.contains(&"S.E.Ev.u16_f.U16".to_string()));
        assert!(paths.contains(&"S.E.Ev.u32_f.U32".to_string()));
        assert!(paths.contains(&"S.E.Ev.u64_f.U64".to_string()));
        assert!(paths.contains(&"S.E.Ev.i8_f.I8".to_string()));
        assert!(paths.contains(&"S.E.Ev.i16_f.I16".to_string()));
        assert!(paths.contains(&"S.E.Ev.i32_f.I32".to_string()));
        assert!(paths.contains(&"S.E.Ev.i64_f.I64".to_string()));
        assert!(paths.contains(&"S.E.Ev.f32_f.F32".to_string()));
        assert!(paths.contains(&"S.E.Ev.f64_f.F64".to_string()));

        // Nested types: outer wrapper then inner type as separate path segments.
        assert!(paths.contains(&"S.E.Ev.opt_f.Option".to_string()));
        assert!(paths.contains(&"S.E.Ev.opt_f.Option.U64".to_string()));
        assert!(paths.contains(&"S.E.Ev.list_f.List".to_string()));
        assert!(paths.contains(&"S.E.Ev.list_f.List.Bool".to_string()));

        // Record reference shows the record name.
        assert!(paths.contains(&"S.E.Ev.rec_f.Record(R)".to_string()));

        // DynamicRecord.
        assert!(paths.contains(&"S.E.Ev.dyn_f.DynamicRecord".to_string()));

        // EntityRef with carried data: EntityRef -> Annotations, then data type.
        assert!(paths.contains(&"S.E.Ev.ref_f.EntityRef".to_string()));
        assert!(paths.contains(&"S.E.Ev.ref_f.EntityRef.Annotations".to_string()));
        assert!(paths.contains(&"S.E.Ev.ref_f.EntityRef.U32".to_string()));
    }
}
