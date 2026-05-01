// For sanity, nesting the exploration here into the following way:
//
// mod entity_name {                // name of the entity
//   mod model { ... }              // the model declaration, the only thing the user writes, the things below are derived/generated
//   mod events { ... }             // the generated event types for the model component
//   mod instrumentation { ... }    // the generated instrumentation api for the model component
//   mod usage { ... }              // instrumentation usage example
//   mod analyzer { ... }           // the generated analyzer api for the model component
// }

mod entity;
mod fsm;
mod resource;
mod resource_group;
