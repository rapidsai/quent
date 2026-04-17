use std::marker::PhantomData;

use uuid::Uuid;

// user facing model declaration types:

// An event that can only be emitted <= 1 time per entity
struct Once<T> {
    _phantom: PhantomData<T>,
}

// An event that can be emitted >= 0 times per entity
struct Multi<T> {
    _phantom: PhantomData<T>,
}

struct OnceRg<T> {
    _phantom: PhantomData<T>,
}

// user example:

struct MyEventAttributesA {
    q: u64,
    r: String,
}

struct MyEventAttributesB {
    s: f64,
    t: Vec<String>,
}

#[derive(Entity)]
struct Foo {
    #[instance_name]    // optional, mark this event to carry the instance name
    a: Once<MyEventAttributesA>,
    b: Once<MyEventAttributesB>,
    c: Once<MyEventAttributesB>, // same attributes, semantically different event
    d: Multi<MyEventAttributesA>,
}

// An entity can be a resource group, which means that at least one of its
// events needs to carry certain attributes.
//
// 0. How to make this a resource group?
//
// a. #[derive(ResourceGroup)] ?
// b. #[quent(resource_group)] ? may be useful because see below
// c. ...?
//
// 1. How do we mark that event?
//
// a. Make the field use some OnceWithResourceGroupAttributes<T> ?
// b. With a field annotation? #[quent(resource_group)] ?
// c. With a struct annotation? #[quent(resource_group(a))] ?
// d. ... ?
//
// 2. if an entity is the root resource group, it does not require a parent.
// How to convey that?
//
// a. OnceWithRootResourceGroupAttributes<T> ?  -> ugly and potential state explosion
// b. Event field annotation #[quent(resource_group(root))] ?
// c. #[quent(resource_group(a, root))] ?
// d. ... ?
//
// Should multi events be able to carry the resource group attributes?
#[derive(Entity, ResourceGroup)]
struct FooResourceGroup {
    #[quent(rg)]
    a: Once<MyEventAttributesA>,
}

#[derive(Entity, ResourceGroup)]
#[quent(root)]
struct FooRootResourceGroup {
    #[quent(rg)]
    a: Once<MyEventAttributesA>,
}


// user facing result (rust):

struct FooHandle {
    id: Uuid,
    a_emitted: bool,
    b_emitted: bool,
    c_emitted: bool,
}

impl FooHandle {
    fn a(instance_name: String, attributes: MyEventAttributesA) {}
    fn b(attributes: MyEventAttributesA) {}
    fn c(attributes: MyEventAttributesA) {}
    fn d(attributes: MyEventAttributesA) {}
}

struct FooObserver {}

impl FooObserver {
    fn handle() -> FooHandle;
}

struct FooResourceGroupHandle {
    id: Uuid,
    a_emitted: bool,
    b_emitted: bool,
    c_emitted: bool,
}

impl FooResourceGroupHandle {
    fn a(instance_name: String, parent_group_id: Uuid, attributes: MyEventAttributesA) {}
}

struct FooRootResourceGroupHandle {}

impl FooObserver {
    fn a(instance_name: String, attributes: MyEventAttributesA) {}
}

// in macros/codegen, not required, just a brainstorm, may need to look completely different:

enum RgAttributes {
    Root,
    Regular,
}

struct EventDecl {
    // to know the fn names on the handle
    name: String,
    // type of the event
    attributes: syn::Type,
    // to know whether to prevent multiple events from being emitted per handle
    multi: bool,
    // to know whether to append rg attributes and which ones
    rg_attributes: Option<RgAttributes>,
}

trait EntityDecl {
    fn events() -> impl Iterator<Item = EventDecl;
}

impl EntityDecl for Foo {
    fn events() -> impl Iterator<Item = EventDecl> {
        [
            EventDecl {
                name: "a",
                attributes: MyEventAttributesA,
                multi: false,
                rg_attributes: None,
            },
            EventDecl {
                name: "b",
                attributes: MyEventAttributesB,
                multi: false,
                rg_attributes: None,
            },
            EventDecl {
                name: "c",
                attributes: MyEventAttributesB,
                multi: false,
                rg_attributes: None,
            },
            EventDecl {
                name: "d",
                attributes: MyEventAttributesA,
                multi: true,
                rg_attributes: None,
            },
        ]
    }
}

impl EntityDecl for FooResourceGroup {
    fn events() -> impl Iterator<Item = EventDecl> {
        [
            EventDecl {
                name: "a",
                attributes: MyEventAttributesA,
                multi: false,
                rg_attributes: Some(RgAttributes::Regular),
            },
        ]
    }
}


impl EntityDecl for FooRootResourceGroup {
    fn events() -> impl Iterator<Item = EventDecl> {
        [
            EventDecl {
                name: "a",
                attributes: MyEventAttributesA,
                multi: false,
                rg_attributes: Some(RgAttributes::Regular),
            },
        ]
    }
}
