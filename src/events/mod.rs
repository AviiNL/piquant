use valence::registry::{Entity, Event};

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct WandCastEvent {
    pub client: Entity,
    pub slot: u16,
}
