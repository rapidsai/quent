// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Constraint marking an entity reference as tree-forming.

use petgraph::{
    Graph,
    graph::NodeIndex,
    graphmap::DiGraphMap,
    visit::{Bfs, Walker},
};
use rustc_hash::{FxHashMap, FxHashSet};
use thiserror::Error;

use quent_constraints::{Constraint, utils::bullet_list};
use quent_ref_target::RefTarget;
use quent_schema::{
    DataType, Identifier,
    visitor::{Cursor, Element, Visitor},
};

/// Constrains the graph of entities (as vertices) and entity references (as
/// edges) to contain a subgraph that is a tree connecting all entities.
///
/// This constraint can be used for arbitrary purposes. Its canonical purpose
/// is to provide some "preferred" way of traversing entities and their events
/// from a single starting point (the root entity).
///
/// References annotated with this constraint are _typically_ used (but not
/// limited) to express:
///
/// - **Hierarchical relations**, for example: "entity X ... entity Y"
///     - is part of
///     - is owned by
///     - is scoped by
///     - is parented by
///     - sits under
/// - **Causal relations**, for example: "entity X ...entity Y"
///     - is spawned by
///     - is produced by
///     - exists because of
///
/// These types of references are _typically_ not used to express:
/// - **Structural references** at the same hierarchical level, for example:
///   entity X is attached to Y (often causes loops)
/// - **Flow directionality**, for example: entity X is moved to Y
///
/// In order for instrumentation libraries to provide strong guarantees
/// (typically compile-time) that this constraint is met, the tree must be fully
/// defined at "schema-time". Therefore, type-erased entity references cannot
/// carry an annotation with this constraint, as this would allow forming entity
/// graphs that are not trees (i.e. multiple instances of an entity of type A
/// would be able to emit events that refer to both an entity of type B and of
/// type C as its parent). For this reason, this constraint depends on the
/// constraint provided by the [`quent_ref_target`] crate.
///
/// ## Requirements
///
/// 1. The schema must have exactly one entity (a.k.a. the root entity) of
///    which its events do not carry an entity reference annotated with this
///    constraint.
/// 2. Every non-root entity must have _at least one_ event carrying _at most
///    one_ entity reference annotated with this constraint, counting references
///    reached through record-typed fields. Correspondingly, a record carries
///    _at most one_ such reference across its (nested) fields.
/// 3. Every entity reference annotated with this constraint must be annotated
///    with a reference target constraint (defined by the `quent-ref-target`
///    crate).
/// 4. All events of one entity carrying an entity reference with this
///    constraint (req 2) target (req 3) to _exactly one_ type of parent
///    entity.
/// 5. From every non-root entity, entity references with this constraint can be
///    followed such that the root is reached.
///
/// ## Note on possible parent ambiguity (req. 2)
///
/// Parent ambiguity at run-time can exist through multiple parent-declaring
/// events, which is allowed by requirement 2.
///
/// Since client code can have branching behavior where certain events are
/// conditionally emitted, this constraint permits the parent reference to be
/// placed (once) on any number of events, even though logically speaking, it
/// can only have one parent, and it would ideally emit its parent reference
/// exactly once. It is the responsibility of the client code to ensure it
/// produces an unambiguous event stream with regards to this tree-forming
/// constraint.
///
/// This constraint intentionally defers any potential solution for clients
/// producing ambiguous event streams to schema producer / consumer
/// implementations.
///
/// For example, a modeling API or DSL _could_ decide to enforce FSM entities to
/// always declare their parent in the initial state. An instrumentation library
/// _could_ error on emitting a second parent-declaring event if it changes the
/// referenced parent. An analysis library _could_ produce an error when an event
/// stream is ingested exhibiting this ambiguity.
#[derive(Default)]
pub struct RefTreeConstraint {
    // Graph of entities, events with entity refs, and records with entity refs.
    graph: Graph<Node, ()>,
    // Map of a node to its petgraph index.
    index: FxHashMap<Node, NodeIndex>,
    // Name of every entity, in schema order, to figure out which one is root.
    entities: Vec<Identifier>,
    // Quick lookup for records that have already declared a parent.
    records_seen: FxHashSet<Identifier>,
    // Set once a tree-forming reference declares a parent. The tree shape is
    // only validated when at least one reference uses the constraint.
    used: bool,
    // Errors gathered during the walk.
    errors: Vec<RefTreeError>,
}

/// A vertex in the entity/event/record graph.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Node {
    Entity(Identifier),
    Event(Identifier, Identifier),
    Record(Identifier),
}

impl Visitor for RefTreeConstraint {
    type Output = Result<(), RefTreeError>;

    fn visit(&mut self, cursor: &Cursor) {
        match cursor.current() {
            // Add every entity as a node.
            Element::Schema(schema) => {
                for entity in schema.entities() {
                    self.entities.push(entity.name().clone());
                    self.node_or_insert(Node::Entity(entity.name().clone()));
                }
            }
            // If we encounter an event, add it as a node and connect it to its
            // entity.
            Element::Event(event) => {
                if let [_, Element::Entity(entity), ..] = cursor.elements() {
                    let from = self.node_or_insert(Node::Entity(entity.name().clone()));
                    let to = self
                        .node_or_insert(Node::Event(entity.name().clone(), event.name().clone()));
                    self.graph.add_edge(from, to, ());
                }
            }
            // If we encounter an entity ref with this constraint, first ensure
            // it has a constrained target entity.
            Element::DataType(DataType::EntityRef { annotations, .. })
                if annotations.has_constraint(RefTreeConstraint::NAME) =>
            {
                match RefTarget::from_annotations(annotations) {
                    // Req 3 is not met:
                    None => self.errors.push(RefTreeError::NotTargetConstrained {
                        location: cursor.to_string(),
                    }),
                    // The reference must target a declared entity, else the
                    // constraint data is ill-formed
                    Some(target) if cursor.root().entity(target.as_ref()).is_none() => {
                        self.errors.push(RefTreeError::UnknownTarget {
                            location: cursor.to_string(),
                            target: target.as_ref().clone(),
                        });
                    }
                    Some(target) => {
                        let target = self.node_or_insert(Node::Entity(target.as_ref().clone()));

                        // The max per-event tree-forming ref count (req 2) is
                        // enforced after the walk, since refs reached through
                        // records are only known then.
                        if let Some((entity, event)) = event_at_cursor(cursor) {
                            let from =
                                self.node_or_insert(Node::Event(entity.clone(), event.clone()));
                            self.graph.add_edge(from, target, ());
                            self.used = true;
                        } else if let Some(record) = record_at_cursor(cursor) {
                            if self.records_seen.insert(record.clone()) {
                                let from = self.node_or_insert(Node::Record(record.clone()));
                                self.graph.add_edge(from, target, ());
                                self.used = true;
                            } else {
                                // A second parent ref in this record violates
                                // req 2 already:
                                self.errors.push(RefTreeError::MultipleRefsInRecord {
                                    location: cursor.to_string(),
                                });
                            }
                        }
                    }
                }
            }
            // If we encounter a datatype with a record, add it to the graph and
            // connect to either the event or the other record using this
            // record.
            Element::DataType(DataType::Record(name)) => {
                if let Some((entity, event)) = event_at_cursor(cursor) {
                    let from = self.node_or_insert(Node::Event(entity.clone(), event.clone()));
                    let to = self.node_or_insert(Node::Record(name.clone()));
                    self.graph.add_edge(from, to, ());
                } else if let Some(record) = record_at_cursor(cursor) {
                    let from = self.node_or_insert(Node::Record(record.clone()));
                    let to = self.node_or_insert(Node::Record(name.clone()));
                    self.graph.add_edge(from, to, ());
                }
            }
            _ => {}
        }
    }

    fn finish(self) -> Self::Output {
        let RefTreeConstraint {
            graph,
            index,
            entities,
            used,
            mut errors,
            ..
        } = self;
        // The tree is only validated when at least one reference uses the
        // constraint. Type-erased references and references to unknown entities
        // are reported during the walk and make the tree undefined, so exit
        // early if this happened.
        if used
            && !errors.iter().any(|e| {
                matches!(
                    e,
                    RefTreeError::NotTargetConstrained { .. } | RefTreeError::UnknownTarget { .. }
                )
            })
        {
            check_events(&graph, &mut errors);
            check_tree(&graph, &index, &entities, &mut errors);
        }
        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.pop().unwrap()),
            _ => Err(RefTreeError::Multiple(errors)),
        }
    }
}

impl Constraint for RefTreeConstraint {
    const NAME: &'static str = "quent.ref-tree.v1";
}

impl RefTreeConstraint {
    fn node_or_insert(&mut self, node: Node) -> NodeIndex {
        if let Some(&index) = self.index.get(&node) {
            return index;
        }
        let index = self.graph.add_node(node.clone());
        self.index.insert(node, index);
        index
    }
}

fn check_tree(
    graph: &Graph<Node, ()>,
    index: &FxHashMap<Node, NodeIndex>,
    entities: &[Identifier],
    errors: &mut Vec<RefTreeError>,
) {
    let mut roots: Vec<&Identifier> = Vec::new();
    let mut non_roots: Vec<&Identifier> = Vec::new();

    // For each entity, we figure out whether we can collapse its parent edges
    // (through multiple events) into one edge. If not, this violates the
    // requirements.
    let mut entity_edges: Vec<(&Identifier, &Identifier)> = Vec::new();

    for entity in entities {
        // Unwrap should be safe here since we added entities to the set and
        // graph in lockstep.
        let node = *index.get(&Node::Entity(entity.clone())).unwrap();

        let unique_parents = entity_unique_parents(graph, node);
        match unique_parents.as_slice() {
            // No parents, it is a (the?) root entity
            [] => roots.push(entity),
            // One parent type, non-root.
            [parent] => {
                non_roots.push(entity);
                entity_edges.push((*parent, entity));
            }
            // Multiple parent types, this violates req 4:
            _ => {
                errors.push(RefTreeError::ConflictingParents {
                    entity: entity.clone(),
                    parents: unique_parents.into_iter().cloned().collect(),
                });
            }
        }
    }

    // Check req 1, one root
    let root = match roots.len() {
        0 => {
            errors.push(RefTreeError::NoRoot);
            return;
        }
        1 => roots[0],
        _ => {
            errors.push(RefTreeError::MultipleRoots {
                roots: roots.iter().map(|r| (*r).clone()).collect(),
            });
            return;
        }
    };

    // We know:
    // - there is one root
    // - each non-root entity refers to exactly one parent type
    // Now check:
    // - every entity reaches root (req 5)
    // If there are no errors afterwards the whole constraint is validated.
    let mut maybe_tree: DiGraphMap<&Identifier, ()> = DiGraphMap::new();
    maybe_tree.add_node(root);
    for (parent, child) in entity_edges {
        maybe_tree.add_edge(parent, child, ());
    }
    let reachable: FxHashSet<&Identifier> = Bfs::new(&maybe_tree, root).iter(&maybe_tree).collect();
    for entity in non_roots {
        if !reachable.contains(entity) {
            errors.push(RefTreeError::Unreachable {
                entity: entity.clone(),
            });
        }
    }
}

// Report all events that exceed one tree-forming refs in their direct or nested
// record fields.
fn check_events(graph: &Graph<Node, ()>, errors: &mut Vec<RefTreeError>) {
    for node in graph.node_indices() {
        if let Node::Event(entity, event) = &graph[node]
            && event_ref_count(graph, node) > 1
        {
            errors.push(RefTreeError::MultiplePerEvent {
                location: format!("{entity}.{event}"),
            });
        }
    }
}

// Check the graph to discover the number of tree-forming refs an event carries,
// including in its nested records.
fn event_ref_count(graph: &Graph<Node, ()>, event: NodeIndex) -> usize {
    let mut count = 0;
    let mut seen_records: FxHashSet<NodeIndex> = FxHashSet::default();
    let mut records: Vec<NodeIndex> = Vec::new();
    for neighbor in graph.neighbors(event) {
        match &graph[neighbor] {
            Node::Entity(_) => count += 1,
            Node::Record(_) => records.push(neighbor),
            Node::Event(..) => {}
        }
    }
    while let Some(record) = records.pop() {
        if !seen_records.insert(record) {
            continue;
        }
        for neighbor in graph.neighbors(record) {
            match &graph[neighbor] {
                Node::Entity(_) => count += 1,
                Node::Record(_) => records.push(neighbor),
                Node::Event(..) => {}
            }
        }
    }
    count
}

// List all unique parent entities reachable from the supplied entity
fn entity_unique_parents(graph: &Graph<Node, ()>, entity: NodeIndex) -> Vec<&Identifier> {
    let mut parents = Vec::new();
    let mut seen: FxHashSet<NodeIndex> = FxHashSet::default();
    let mut stack: Vec<NodeIndex> = graph.neighbors(entity).collect();
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        match &graph[node] {
            Node::Entity(parent) => parents.push(parent),
            Node::Event(..) | Node::Record(_) => stack.extend(graph.neighbors(node)),
        }
    }
    parents
}

fn event_at_cursor<'s>(cursor: &'s Cursor) -> Option<(&'s Identifier, &'s Identifier)> {
    match cursor.elements() {
        [_schema, Element::Entity(entity), Element::Event(event), ..] => {
            Some((entity.name(), event.name()))
        }
        _ => None,
    }
}
fn record_at_cursor<'s>(cursor: &'s Cursor) -> Option<&'s Identifier> {
    match cursor.elements() {
        [_schema, Element::Record(record), ..] => Some(record.name()),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum RefTreeError {
    #[error("{location}: a tree-forming reference must be target-constrained")]
    NotTargetConstrained { location: String },
    #[error("{location}: tree-forming reference targets unknown entity \"{target}\"")]
    UnknownTarget {
        location: String,
        target: Identifier,
    },
    #[error("{location}: an event carries more than one tree-forming reference")]
    MultiplePerEvent { location: String },
    #[error("{location}: a record carries more than one tree-forming reference")]
    MultipleRefsInRecord { location: String },
    #[error("tree-forming references are used, but there is no root entity")]
    NoRoot,
    #[error("more than one root entity: {}", join_idents(.roots))]
    MultipleRoots { roots: Vec<Identifier> },
    #[error("entity \"{entity}\" declares more than one parent type: {}", join_idents(.parents))]
    ConflictingParents {
        entity: Identifier,
        parents: Vec<Identifier>,
    },
    #[error("entity \"{entity}\" has no path to the root through tree-forming references")]
    Unreachable { entity: Identifier },
    #[error("multiple ref-tree violations:\n{}", bullet_list(.0))]
    Multiple(Vec<RefTreeError>),
}

fn join_idents(ids: &[Identifier]) -> String {
    ids.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(", ")
}
