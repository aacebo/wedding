pub mod contact;
pub mod schedule;
pub mod story;
pub mod travel;
pub mod venue;
pub mod welcome;

struct PageMetadata {
    title: &'static str,
    description: &'static str,
    url: &'static str,
}

impl PageMetadata {
    const fn new(title: &'static str, description: &'static str, url: &'static str) -> Self {
        Self {
            title,
            description,
            url,
        }
    }
}
