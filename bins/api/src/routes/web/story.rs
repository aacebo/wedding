use std::fs;

use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "story.html")]
struct StoryPage {
    meta: PageMetadata,
    images: Vec<String>,
}

fn story_images() -> Vec<String> {
    let mut names: Vec<String> = match fs::read_dir("bins/api/assets/story") {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| {
                let lower = name.to_ascii_lowercase();
                matches!(
                    lower.rsplit('.').next(),
                    Some("jpg" | "jpeg" | "png" | "webp" | "gif")
                )
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    names.sort();
    names
        .into_iter()
        .map(|name| format!("/assets/story/{name}"))
        .collect()
}

#[get("/story")]
pub async fn get() -> Result<Html, Error> {
    let page = StoryPage {
        meta: PageMetadata::new(
            "Our Love Story — Nancy & Alexander",
            "One first date, one wine-fueled text, and a spontaneous trip to Italy. The love story of Nancy and Alexander.",
            "https://baicebo.com/story",
        ),
        images: story_images(),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn story_renders_approved_copy_without_placeholder_notices() {
        let app = test::init_service(App::new().service(get)).await;
        let response =
            test::call_service(&app, test::TestRequest::get().uri("/story").to_request()).await;
        assert!(response.status().is_success());
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        for text in [
            "Our Love Story",
            "one wine-fueled text",
            "The next morning, Nancy asked again.",
            "And Alex said yes.",
            "this time with their favorite people.",
            "their favorite one yet.",
        ] {
            assert!(body.contains(text), "Missing story text: {text}");
        }
        assert!(!body.contains("Full story coming soon"));
        assert!(!body.contains("More details coming soon"));
        assert!(!body.contains("aria-label=\"Page status\""));
    }

    #[test]
    async fn timeline_uses_confirmed_milestones_and_preserves_carousel() {
        let page = StoryPage {
            meta: PageMetadata::new("Story", "Story", "https://baicebo.com/story"),
            images: vec!["/assets/story/1.jpg".into(), "/assets/story/2.jpg".into()],
        };
        let body = page.render().unwrap();
        let timeline = body.split("aria-label=\"Our timeline\"").nth(1).unwrap();
        assert_eq!(timeline.matches("class=\"timeline-entry\"").count(), 5);
        assert_eq!(timeline.matches("datetime=\"2021\"").count(), 3);
        let mut remaining = timeline;
        for milestone in [
            "A beer garden in Bushwick.",
            "Next stop: Italy.",
            "One more question in Rome.",
            "datetime=\"2026\"",
            "A very easy yes in St. Lucia.",
            "datetime=\"2027-06-19\"",
            "Back to Italy, together.",
        ] {
            remaining = remaining
                .split_once(milestone)
                .unwrap_or_else(|| panic!("Missing or out-of-order milestone: {milestone}"))
                .1;
        }
        assert!(!timeline.contains("2024"));
        assert!(timeline.contains("before heading back to New York City"));
        assert!(timeline.contains("Cinque Terre"));
        assert!(body.contains("aria-roledescription=\"carousel\""));
        assert!(body.contains("aria-label=\"Previous photo\""));
        assert!(body.contains("aria-label=\"Next photo\""));
        assert!(body.contains("src=\"/assets/story/1.jpg\""));
        assert!(body.contains("src=\"/assets/story/2.jpg\""));
    }
}
