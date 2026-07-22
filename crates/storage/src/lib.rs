use sqlx::PgPool;

pub mod types;

mod google_account;
mod guest;
mod rsvp;

pub use google_account::*;
pub use guest::*;
pub use rsvp::*;

pub struct Storage<'a> {
    _guests: GuestStorage<'a>,
    _rsvps: RsvpStorage<'a>,
    _google: GoogleAccountStorage<'a>,
}

impl<'a> Storage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            _guests: GuestStorage::new(pool),
            _rsvps: RsvpStorage::new(pool),
            _google: GoogleAccountStorage::new(pool),
        }
    }

    pub fn guests(&self) -> &GuestStorage<'a> {
        &self._guests
    }

    pub fn rsvps(&self) -> &RsvpStorage<'a> {
        &self._rsvps
    }

    pub fn google(&self) -> &GoogleAccountStorage<'a> {
        &self._google
    }
}
