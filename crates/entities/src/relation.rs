use crate::EntityRef;

pub trait Related {
    /// Return an iterator over any direct relations (i.e. not including relations of the relations).
    fn relations(&self) -> impl Iterator<Item = EntityRef>;

    /// Return an iterator over all direct Use relations
    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        std::iter::empty()
    }
}
