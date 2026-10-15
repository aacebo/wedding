pub mod contact;
pub mod rsvp;
pub mod schedule;
pub mod story;
pub mod travel;
pub mod venue;
pub mod welcome;

pub(super) struct PageMetadata {
    pub(super) title: &'static str,
    pub(super) description: &'static str,
    pub(super) url: &'static str,
}

impl PageMetadata {
    pub(super) const fn new(
        title: &'static str,
        description: &'static str,
        url: &'static str,
    ) -> Self {
        Self {
            title,
            description,
            url,
        }
    }
}
