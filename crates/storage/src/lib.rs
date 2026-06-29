use sqlx::PgPool;

pub mod types;

mod guest;
mod rsvp;

pub use guest::*;
pub use rsvp::*;

pub struct Storage<'a> {
    _guests: GuestStorage<'a>,
    _rsvps: RsvpStorage<'a>,
}

impl<'a> Storage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            _guests: GuestStorage::new(pool),
            _rsvps: RsvpStorage::new(pool),
        }
    }

    pub fn guests(&self) -> &GuestStorage<'a> {
        &self._guests
    }

    pub fn rsvps(&self) -> &RsvpStorage<'a> {
        &self._rsvps
    }
}
