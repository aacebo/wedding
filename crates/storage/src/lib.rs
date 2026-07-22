use sqlx::PgPool;

pub mod types;

mod comm_source;
mod google_account;
mod guest;
mod rsvp;
mod sync_state;

pub use comm_source::*;
pub use google_account::*;
pub use guest::*;
pub use rsvp::*;
pub use sync_state::*;

pub struct Storage<'a> {
    _guests: GuestStorage<'a>,
    _rsvps: RsvpStorage<'a>,
    _google: GoogleAccountStorage<'a>,
    _comm_sources: CommSourceStorage<'a>,
    _sync_state: SyncStateStorage<'a>,
}

impl<'a> Storage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            _guests: GuestStorage::new(pool),
            _rsvps: RsvpStorage::new(pool),
            _google: GoogleAccountStorage::new(pool),
            _comm_sources: CommSourceStorage::new(pool),
            _sync_state: SyncStateStorage::new(pool),
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

    pub fn comm_sources(&self) -> &CommSourceStorage<'a> {
        &self._comm_sources
    }

    pub fn sync_state(&self) -> &SyncStateStorage<'a> {
        &self._sync_state
    }
}
