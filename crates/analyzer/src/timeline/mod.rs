use quent_entities::{
    EntityRef,
    fsm::Fsm,
    timeline::{ResourceTimeline, ResourceTimelineUse},
};
use quent_time::Span;
use uuid::Uuid;

use crate::{Result, entities::Entities, error::Error};

// TODO(johanpel): maybe move everything into a single crate and make traits for this
// TODO(johanpel); combine / chain this with Related::use_relations
pub fn make_fsm_resource_timeline_uses(
    fsm: &Fsm,
    resource_id: Uuid,
) -> impl Iterator<Item = ResourceTimelineUse> {
    let usage_states = fsm.state_sequence.iter().enumerate().flat_map({
        move |(index, state)| {
            state
                .uses
                .iter()
                .filter(move |u| u.resource == resource_id)
                .map(move |u| (index, u))
        }
    });

    usage_states.map(|(state_index, usage)| {
        let span = fsm.state_span(state_index).unwrap();
        ResourceTimelineUse {
            span,
            amounts: usage.amounts.clone(),
            entity: EntityRef::CustomFsm(fsm.id),
        }
    })
}

pub fn make_resource_timeline_for_resource(
    entities: &Entities,
    resource_id: Uuid,
) -> Result<ResourceTimeline> {
    // TODO(johanpel): could be supplied with an entity name and a state name filter

    // Sanity checks
    match entities.get_entity_ref_from_id(resource_id) {
        Some(EntityRef::Resource(_)) => {}
        Some(entity_ref) => {
            return Err(Error::Logic(format!(
                "ID {resource_id} is an entity but it is not a resource ({entity_ref:?})"
            )));
        }
        None => return Err(Error::InvalidId(resource_id)),
    }

    // TODO(johanpel): not unwrap
    let span = Span::try_new(
        entities.engine.timestamps.init.unwrap(),
        entities.engine.timestamps.exit.unwrap(),
    )?;

    let uses = entities
        .iter_use_relations()
        .filter_map(|(user, resource)| (Uuid::from(resource) == resource_id).then_some(user))
        .filter_map(|user| match user {
            quent_entities::EntityRef::CustomFsm(uuid) => {
                Some(entities.custom_fsms.get(&uuid).unwrap())
            }
            _ => None,
        })
        .flat_map(|fsm| make_fsm_resource_timeline_uses(fsm, resource_id))
        .collect::<Vec<_>>();

    Ok(ResourceTimeline { span, uses })
}
